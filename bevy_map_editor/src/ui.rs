//! The editor overlay: toolbar, asset browser, inspector, scene list and status bar.

use bevy::prelude::*;
use bevy_egui::{EguiContext, egui};
use egui::{LayerId, Ui, UiBuilder};

use crate::camera::EditorCamera;
use crate::editor::{EditorAction, EditorState};
use crate::library::{Library, LibraryGroup};
use crate::props::{CollisionKind, Ghost, Primitive, Prop, PropData, PropMaterial, PropSource};
use crate::{EditorMode, MapEditorSettings, MapEditorUiPass, editor_open};

pub(crate) struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(MapEditorUiPass, editor_ui.run_if(editor_open));
    }
}

#[derive(Default)]
struct UiLocal {
    search: String,
    uniform_scale: bool,
}

const HINTS: &str = "Hold RMB: look + WASD fly, Q/E down/up, Shift fast, wheel = speed  •  Wheel: dolly  •  MMB: pan  •  \
LMB: select / drag  •  G grab  •  R / Shift+R rotate  •  Arrows, PgUp/PgDn nudge  •  End drop to ground  •  \
F focus  •  Del delete  •  Ctrl+D duplicate  •  Ctrl+Z / Ctrl+Y undo/redo  •  Ctrl+S save";

#[allow(clippy::too_many_arguments)]
fn editor_ui(
    mut egui_ctx: Single<&mut EguiContext, With<EditorCamera>>,
    mut state: ResMut<EditorState>,
    mut settings: ResMut<MapEditorSettings>,
    props: Query<(Entity, &Prop, &Transform, Has<PropMaterial>), Without<Ghost>>,
    library: Res<Library>,
    mut actions: MessageWriter<EditorAction>,
    mut next_mode: ResMut<NextState<EditorMode>>,
    mut local: Local<UiLocal>,
) {
    let ctx = egui_ctx.get_mut().clone();
    let mut root = Ui::new(
        ctx.clone(),
        "map_editor_root".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    // ---- Toolbar ---------------------------------------------------------------------------
    let top = egui::Panel::top("map_editor_toolbar").show(&mut root, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("🗺 MAP EDITOR");
            if ui
                .button(format!("▶ Back to game ({:?})", settings.toggle_key))
                .clicked()
            {
                next_mode.set(EditorMode::Playing);
            }
            ui.separator();

            ui.label("Map:");
            ui.add(egui::TextEdit::singleline(&mut state.map_name).desired_width(120.0));
            if ui.button("💾 Save").clicked() {
                actions.write(EditorAction::Save(state.map_name.clone()));
            }
            egui::ComboBox::from_id_salt("map_editor_load")
                .selected_text("📂 Load")
                .show_ui(ui, |ui| {
                    if library.maps.is_empty() {
                        ui.label("No saved maps yet");
                    }
                    for map in library.maps.clone() {
                        if ui.selectable_label(false, &map).clicked() {
                            actions.write(EditorAction::Load(map));
                        }
                    }
                });
            if ui.button("🗋 New").clicked() {
                actions.write(EditorAction::NewMap);
            }
            ui.separator();

            if ui
                .add_enabled(!state.undo.is_empty(), egui::Button::new("↶ Undo"))
                .clicked()
            {
                actions.write(EditorAction::Undo);
            }
            if ui
                .add_enabled(!state.redo.is_empty(), egui::Button::new("↷ Redo"))
                .clicked()
            {
                actions.write(EditorAction::Redo);
            }
            ui.separator();

            ui.checkbox(&mut settings.snap_enabled, "Snap");
            ui.add(
                egui::DragValue::new(&mut settings.grid_size)
                    .speed(0.05)
                    .range(0.05..=50.0)
                    .prefix("grid "),
            );
            ui.add(
                egui::DragValue::new(&mut settings.rotate_step_deg)
                    .speed(1.0)
                    .range(1.0..=180.0)
                    .prefix("rot ")
                    .suffix("°"),
            );
            ui.checkbox(&mut settings.show_grid, "Grid");
            ui.add(
                egui::DragValue::new(&mut settings.camera_speed)
                    .speed(0.2)
                    .range(0.5..=500.0)
                    .prefix("cam speed "),
            );
            ui.checkbox(&mut settings.pause_game, "Pause game")
                .on_hover_text("Freeze game time while the editor is open (applies next time you open it)");
        });
    });

    // ---- Status bar ------------------------------------------------------------------------
    let bottom = egui::Panel::bottom("map_editor_status").show(&mut root, |ui| {
        ui.label(egui::RichText::new(&state.status).strong());
        ui.label(egui::RichText::new(HINTS).small().weak());
    });

    // ---- Asset browser ---------------------------------------------------------------------
    let left = egui::Panel::left("map_editor_assets")
        .resizable(true)
        .default_size(260.0)
        .show(&mut root, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Assets");
                if ui.small_button("⟳").on_hover_text("Rescan the game's assets").clicked() {
                    actions.write(EditorAction::RefreshLibrary);
                }
            });
            ui.label(
                egui::RichText::new("Drag into the world, or click then click where it goes.")
                    .small()
                    .weak(),
            );
            let pick = ui
                .add(egui::Button::new("🎯 Pick from game").selected(state.picking))
                .on_hover_text("Click any object in the game world to place copies of it");
            if pick.clicked() {
                actions.write(if state.picking { EditorAction::CancelPlace } else { EditorAction::StartPicking });
            }
            ui.add(egui::TextEdit::singleline(&mut local.search).hint_text("🔍 Search"));
            let search = local.search.to_lowercase();
            let matches = |label: &str, hint: &str| {
                search.is_empty() || label.to_lowercase().contains(&search) || hint.to_lowercase().contains(&search)
            };
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::CollapsingHeader::new("Primitives").default_open(true).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for primitive in Primitive::ALL {
                            if matches(primitive.label(), "") {
                                let data = PropData::new(PropSource::Primitive(primitive));
                                asset_tile(ui, primitive.label(), "Built-in shape with collision", data, &mut actions);
                            }
                        }
                    });
                });

                for group in [LibraryGroup::Prefabs, LibraryGroup::Picked, LibraryGroup::Files, LibraryGroup::Loaded] {
                    let entries: Vec<_> = library.group(group).filter(|e| matches(&e.label, &e.hint)).collect();
                    let total = library.group(group).count();
                    if total == 0 && group != LibraryGroup::Files {
                        continue;
                    }
                    let open = group != LibraryGroup::Loaded || !search.is_empty();
                    egui::CollapsingHeader::new(format!("{} ({total})", group.title()))
                        .id_salt(group.title())
                        .default_open(open)
                        .show(ui, |ui| {
                            if total == 0 {
                                ui.label(
                                    egui::RichText::new(
                                        "No scene files in the game's assets folder. Drop .glb/.gltf files on the window to import.",
                                    )
                                    .small()
                                    .weak(),
                                );
                            }
                            const SHOWN: usize = 300;
                            ui.horizontal_wrapped(|ui| {
                                for entry in entries.iter().take(SHOWN) {
                                    asset_tile(ui, &entry.label, &entry.hint, entry.data.clone(), &mut actions);
                                }
                            });
                            if entries.len() > SHOWN {
                                ui.label(
                                    egui::RichText::new(format!("{} more: search to narrow down", entries.len() - SHOWN))
                                        .small()
                                        .weak(),
                                );
                            }
                        });
                }
            });
        });

    // ---- Inspector + scene list ------------------------------------------------------------
    let right = egui::Panel::right("map_editor_inspector")
        .resizable(true)
        .default_size(280.0)
        .show(&mut root, |ui| {
            ui.heading("Inspector");
            match state.selected.and_then(|e| props.get(e).ok()) {
                Some((entity, prop, transform, has_color)) => {
                    inspector(ui, entity, prop, transform, has_color, &mut local.uniform_scale, &mut actions)
                }
                None => {
                    ui.label(egui::RichText::new("Nothing selected. Click a prop in the world.").weak());
                }
            }

            ui.separator();
            ui.heading(format!("Scene ({} props)", props.iter().len()));
            egui::ScrollArea::vertical()
                .id_salt("map_editor_scene_list")
                .show(ui, |ui| {
                    let mut list: Vec<_> = props.iter().collect();
                    list.sort_by_key(|(e, ..)| *e);
                    for (entity, prop, ..) in list {
                        let selected = state.selected == Some(entity);
                        if ui.selectable_label(selected, &prop.name).clicked() {
                            actions.write(EditorAction::Select(Some(entity)));
                        }
                    }
                });
        });

    // ---- Where is the mouse? ---------------------------------------------------------------
    let rects = [top.response.rect, bottom.response.rect, left.response.rect, right.response.rect];
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    state.pointer_over_ui = pointer.is_some_and(|p| {
        rects.iter().any(|r| r.contains(p))
            || ctx.layer_id_at(p).is_some_and(|layer| layer.order != egui::Order::Background)
    });
    state.wants_keyboard = ctx.egui_wants_keyboard_input();

    if state.placing.as_ref().is_some_and(|p| p.from_drag) {
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    }
}

fn asset_tile(
    ui: &mut Ui,
    label: &str,
    hover: &str,
    data: PropData,
    actions: &mut MessageWriter<EditorAction>,
) {
    let response = ui
        .add_sized([108.0, 30.0], egui::Button::new(label).truncate())
        .interact(egui::Sense::click_and_drag())
        .on_hover_text(hover);
    if response.drag_started() {
        actions.write(EditorAction::BeginPlace { data, from_drag: true });
    } else if response.clicked() {
        actions.write(EditorAction::BeginPlace { data, from_drag: false });
    }
}

fn inspector(
    ui: &mut Ui,
    entity: Entity,
    prop: &Prop,
    transform: &Transform,
    has_color: bool,
    uniform_scale: &mut bool,
    actions: &mut MessageWriter<EditorAction>,
) {
    let mut name = prop.name.clone();
    ui.horizontal(|ui| {
        ui.label("Name");
        if ui.text_edit_singleline(&mut name).changed() {
            actions.write(EditorAction::Rename(entity, name.clone()));
        }
    });
    let source = match &prop.source {
        PropSource::Primitive(p) => format!("Primitive: {}", p.label()),
        PropSource::Model(path) => format!("Scene: {path}"),
        PropSource::Parts(parts) => match parts.as_slice() {
            [one] => format!("Mesh: {}", one.mesh),
            many => format!("{} meshes: {}", many.len(), many[0].mesh),
        },
        PropSource::Prefab(name) => format!("Game prefab: {name}"),
    };
    ui.label(egui::RichText::new(source).small().weak());
    ui.add_space(4.0);

    let mut t = *transform;
    let mut changed = false;
    let mut begin = false;
    let mut track = |r: egui::Response| {
        if r.drag_started() || (r.changed() && !r.dragged()) {
            begin = true;
        }
        changed |= r.changed();
    };

    egui::Grid::new("map_editor_transform").num_columns(4).show(ui, |ui| {
        ui.label("Position");
        for (i, axis) in ["x ", "y ", "z "].into_iter().enumerate() {
            track(ui.add(egui::DragValue::new(&mut t.translation[i]).speed(0.05).prefix(axis).max_decimals(3)));
        }
        ui.end_row();

        ui.label("Rotation");
        let (yaw, pitch, roll) = t.rotation.to_euler(EulerRot::YXZ);
        let mut deg = [pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees()];
        let mut rot_changed = false;
        for (i, axis) in ["x ", "y ", "z "].into_iter().enumerate() {
            let r = ui.add(egui::DragValue::new(&mut deg[i]).speed(1.0).prefix(axis).suffix("°").max_decimals(1));
            rot_changed |= r.changed();
            track(r);
        }
        if rot_changed {
            t.rotation = Quat::from_euler(
                EulerRot::YXZ,
                deg[1].to_radians(),
                deg[0].to_radians(),
                deg[2].to_radians(),
            );
        }
        ui.end_row();

        ui.label("Scale");
        if *uniform_scale {
            let mut s = t.scale.x;
            let r = ui.add(egui::DragValue::new(&mut s).speed(0.02).range(0.01..=1000.0).max_decimals(3));
            if r.changed() {
                t.scale = Vec3::splat(s);
            }
            track(r);
        } else {
            for (i, axis) in ["x ", "y ", "z "].into_iter().enumerate() {
                track(ui.add(
                    egui::DragValue::new(&mut t.scale[i]).speed(0.02).range(0.01..=1000.0).prefix(axis).max_decimals(3),
                ));
            }
        }
        ui.end_row();
    });
    ui.checkbox(uniform_scale, "Uniform scale");

    if changed {
        if begin {
            actions.write(EditorAction::PushUndo);
        }
        actions.write(EditorAction::SetTransform(entity, t));
    }

    ui.add_space(4.0);
    if has_color {
        ui.horizontal(|ui| {
            ui.label("Color");
            let mut color = prop.color;
            if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                actions.write(EditorAction::SetColor(entity, color));
            }
        });
    }

    ui.horizontal(|ui| {
        ui.label("Collision");
        let kinds: &[CollisionKind] = match prop.source {
            PropSource::Primitive(_) => &[CollisionKind::None, CollisionKind::Solid],
            _ => &[CollisionKind::None, CollisionKind::Solid, CollisionKind::ConvexHull],
        };
        egui::ComboBox::from_id_salt("map_editor_collision")
            .selected_text(prop.collision.label())
            .show_ui(ui, |ui| {
                for &kind in kinds {
                    if ui.selectable_label(prop.collision == kind, kind.label()).clicked()
                        && kind != prop.collision
                    {
                        actions.write(EditorAction::PushUndo);
                        actions.write(EditorAction::SetCollision(entity, kind));
                    }
                }
            });
    });

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Focus (F)").clicked() {
            actions.write(EditorAction::Focus);
        }
        if ui.button("Duplicate").clicked() {
            actions.write(EditorAction::Duplicate);
        }
        if ui.button("Drop to ground").clicked() {
            actions.write(EditorAction::DropToGround);
        }
        if ui.button("🗑 Delete").clicked() {
            actions.write(EditorAction::Delete);
        }
    });
}
