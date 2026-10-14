//! Editor behaviour: placing, selecting, moving, undo, save/load.

use std::path::{Path, PathBuf};

use bevy::camera::primitives::Aabb;
use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use bevy::window::{FileDragAndDrop, PrimaryWindow};

use crate::camera::EditorCamera;
use crate::library::{Eyedropper, Library};
use crate::props::{CollisionKind, Ghost, MapFile, Prop, PropData, PropMaterial, PropSource, PropSpawner};
use crate::{EditorMode, MapEditorSettings, StartupConfig, assets_root, editor_open, maps_dir};
use crate::props::is_gltf_path;

/// Everything the editor can do. The UI and hotkeys send these; your game can too
/// (e.g. `EditorAction::Load("level2".into())` to switch levels at runtime).
#[derive(Message, Clone, Debug)]
pub enum EditorAction {
    Select(Option<Entity>),
    /// Spawn a prop directly (undoable, and it becomes the selection).
    Spawn(PropData),
    BeginPlace { data: PropData, from_drag: bool },
    CancelPlace,
    Delete,
    Duplicate,
    Focus,
    DropToGround,
    RotateSelected(f32),
    Nudge(Vec3),
    PushUndo,
    Undo,
    Redo,
    SetTransform(Entity, Transform),
    SetColor(Entity, [f32; 4]),
    SetCollision(Entity, CollisionKind),
    Rename(Entity, String),
    Save(String),
    Load(String),
    NewMap,
    RefreshLibrary,
    Import(PathBuf),
    /// Eyedropper mode: the next click in the world copies a game object into the library.
    StartPicking,
    /// Copy this game-world entity (or the object it belongs to) into the library and start placing it.
    PickFrom(Entity),
}

/// Shorthand for loading a map from game code: `commands.write_message(LoadMap("level2".into()))`.
#[derive(Message, Clone, Debug)]
pub struct LoadMap(pub String);

/// Editor state shared between the UI and the world interaction systems.
#[derive(Resource)]
pub struct EditorState {
    pub selected: Option<Entity>,
    pub hovered: Option<Entity>,
    pub map_name: String,
    pub status: String,
    /// The mouse is over an editor panel (clicks belong to the UI, not the world).
    pub pointer_over_ui: bool,
    /// A text field has focus (hotkeys are off).
    pub wants_keyboard: bool,
    /// Right mouse is held: flying the camera.
    pub looking: bool,
    pub(crate) placing: Option<Placing>,
    pub(crate) drag: Option<DragMove>,
    pub(crate) click_consumed: bool,
    pub(crate) undo: Vec<Vec<PropData>>,
    pub(crate) redo: Vec<Vec<PropData>>,
    pub(crate) picking: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected: None,
            hovered: None,
            map_name: "untitled".into(),
            status: "Drag a prop from the left panel into the world.".into(),
            pointer_over_ui: false,
            wants_keyboard: false,
            looking: false,
            placing: None,
            drag: None,
            click_consumed: false,
            undo: Vec::new(),
            redo: Vec::new(),
            picking: false,
        }
    }
}

impl EditorState {
    pub fn is_placing(&self) -> bool {
        self.placing.is_some()
    }
}

pub(crate) struct Placing {
    pub data: PropData,
    pub ghost: Option<Entity>,
    pub from_drag: bool,
    pub yaw: f32,
}

pub(crate) struct DragMove {
    entity: Entity,
    press_pos: Vec2,
    start: Transform,
    bottom: f32,
    active: bool,
    /// Started with G: follows the mouse until clicked, Esc puts it back.
    grab: bool,
}

pub(crate) struct EditorLogicPlugin;

impl Plugin for EditorLogicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorState>()
            .add_message::<EditorAction>()
            .add_message::<LoadMap>()
            .add_systems(Startup, startup_library)
            .add_systems(OnExit(EditorMode::Editing), leave_editor)
            .add_systems(
                Update,
                (
                    (hotkeys, file_drop).run_if(editor_open),
                    forward_load_map,
                    apply_actions,
                    (placement, select_and_drag, draw_gizmos).chain().run_if(editor_open),
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------------------------
// Ray casting from the editor camera

#[derive(SystemParam)]
pub(crate) struct Picker<'w, 's> {
    ray_cast: MeshRayCast<'w, 's>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<EditorCamera>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    parents: Query<'w, 's, &'static ChildOf>,
    props: Query<'w, 's, Has<Ghost>, With<Prop>>,
}

pub(crate) struct Hit {
    pub entity: Entity,
    pub prop: Option<Entity>,
    pub point: Vec3,
}

fn prop_root(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    props: &Query<Has<Ghost>, With<Prop>>,
) -> Option<Entity> {
    loop {
        if props.contains(entity) {
            return Some(entity);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

impl Picker<'_, '_> {
    pub fn cursor(&self) -> Option<Vec2> {
        self.windows.single().ok()?.cursor_position()
    }

    pub fn cursor_ray(&self) -> Option<Ray3d> {
        let (camera, transform) = self.cameras.single().ok()?;
        camera.viewport_to_world(transform, self.cursor()?).ok()
    }

    /// Nearest hit along `ray`, skipping placement ghosts and the `exclude` prop.
    pub fn cast(&mut self, ray: Ray3d, exclude: Option<Entity>, props_only: bool) -> Option<Hit> {
        let parents = &self.parents;
        let props = &self.props;
        let filter = |entity: Entity| match prop_root(entity, parents, props) {
            Some(root) => Some(root) != exclude && !props.get(root).unwrap_or(false),
            None => !props_only,
        };
        let settings = MeshRayCastSettings::default()
            .with_filter(&filter)
            .never_early_exit();
        let (entity, hit) = self.ray_cast.cast_ray(ray, &settings).first()?.clone();
        Some(Hit {
            entity,
            prop: prop_root(entity, parents, props),
            point: hit.point,
        })
    }

    /// Where the cursor points in the world: a surface, or the ground plane, or a spot in front.
    pub fn cursor_target(&mut self, exclude: Option<Entity>) -> Option<Vec3> {
        let ray = self.cursor_ray()?;
        if let Some(hit) = self.cast(ray, exclude, false) {
            return Some(hit.point);
        }
        if let Some(d) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
            && d < 500.0
        {
            return Some(ray.get_point(d));
        }
        Some(ray.get_point(15.0))
    }
}

/// World-space bounds of a prop and everything under it.
#[derive(SystemParam)]
pub(crate) struct Bounds<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    aabbs: Query<'w, 's, (&'static Aabb, &'static GlobalTransform)>,
    globals: Query<'w, 's, &'static GlobalTransform>,
}

impl Bounds<'_, '_> {
    pub fn world_aabb(&self, root: Entity) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut any = false;
        for entity in std::iter::once(root).chain(self.children.iter_descendants(root)) {
            let Ok((aabb, global)) = self.aabbs.get(entity) else {
                continue;
            };
            let center = Vec3::from(aabb.center);
            let half = Vec3::from(aabb.half_extents);
            let affine = global.affine();
            for i in 0..8 {
                let corner = center
                    + half
                        * Vec3::new(
                            if i & 1 == 0 { -1.0 } else { 1.0 },
                            if i & 2 == 0 { -1.0 } else { 1.0 },
                            if i & 4 == 0 { -1.0 } else { 1.0 },
                        );
                let p = affine.transform_point3(corner);
                min = min.min(p);
                max = max.max(p);
            }
            any = true;
        }
        any.then_some((min, max))
    }

    /// How far the prop's origin sits above its lowest point, so it can rest on surfaces.
    pub fn bottom_offset(&self, root: Entity) -> f32 {
        match (self.world_aabb(root), self.globals.get(root)) {
            (Some((min, _)), Ok(global)) => global.translation().y - min.y,
            _ => 0.0,
        }
    }
}

fn snap(mut p: Vec3, settings: &MapEditorSettings) -> Vec3 {
    if settings.snap_enabled && settings.grid_size > 0.0 {
        let g = settings.grid_size;
        p.x = (p.x / g).round() * g;
        p.z = (p.z / g).round() * g;
    }
    p
}

fn snapshot<'a>(props: impl Iterator<Item = (&'a Prop, &'a Transform)>) -> Vec<PropData> {
    props.map(|(prop, t)| PropData::from_prop(prop, t)).collect()
}

// ---------------------------------------------------------------------------------------------
// Input

fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<EditorState>,
    settings: Res<MapEditorSettings>,
    mut actions: MessageWriter<EditorAction>,
) {
    if state.wants_keyboard || state.looking {
        return;
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let step = settings.grid_size.max(0.05);

    if keys.just_pressed(KeyCode::Escape) {
        actions.write(if state.placing.is_some() || state.picking {
            EditorAction::CancelPlace
        } else {
            EditorAction::Select(None)
        });
    }
    if ctrl {
        if keys.just_pressed(KeyCode::KeyZ) {
            actions.write(if shift { EditorAction::Redo } else { EditorAction::Undo });
        }
        if keys.just_pressed(KeyCode::KeyY) {
            actions.write(EditorAction::Redo);
        }
        if keys.just_pressed(KeyCode::KeyD) {
            actions.write(EditorAction::Duplicate);
        }
        if keys.just_pressed(KeyCode::KeyS) {
            actions.write(EditorAction::Save(state.map_name.clone()));
        }
        return;
    }
    if keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace, KeyCode::KeyX]) {
        actions.write(EditorAction::Delete);
    }
    if keys.just_pressed(KeyCode::KeyR) && state.placing.is_none() {
        let deg = if shift { -settings.rotate_step_deg } else { settings.rotate_step_deg };
        actions.write(EditorAction::RotateSelected(deg));
    }
    if keys.just_pressed(KeyCode::KeyF) {
        actions.write(EditorAction::Focus);
    }
    if keys.just_pressed(KeyCode::End) {
        actions.write(EditorAction::DropToGround);
    }
    let nudges = [
        (KeyCode::PageUp, Vec3::Y),
        (KeyCode::PageDown, Vec3::NEG_Y),
        (KeyCode::ArrowLeft, Vec3::NEG_X),
        (KeyCode::ArrowRight, Vec3::X),
        (KeyCode::ArrowUp, Vec3::NEG_Z),
        (KeyCode::ArrowDown, Vec3::Z),
    ];
    for (key, dir) in nudges {
        if keys.just_pressed(key) {
            actions.write(EditorAction::Nudge(dir * step));
        }
    }
}

/// Dropping a .glb/.gltf from Explorer/Finder onto the game window imports it.
fn file_drop(mut drops: MessageReader<FileDragAndDrop>, mut actions: MessageWriter<EditorAction>) {
    for drop in drops.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = drop {
            actions.write(EditorAction::Import(path_buf.clone()));
        }
    }
}

fn forward_load_map(mut loads: MessageReader<LoadMap>, mut actions: MessageWriter<EditorAction>) {
    for LoadMap(name) in loads.read() {
        actions.write(EditorAction::Load(name.clone()));
    }
}

// ---------------------------------------------------------------------------------------------
// Placing new props

#[allow(clippy::too_many_arguments)]
fn placement(
    mut state: ResMut<EditorState>,
    settings: Res<MapEditorSettings>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut spawner: PropSpawner,
    mut picker: Picker,
    bounds: Bounds,
    mut ghosts: Query<(&mut Transform, &mut Visibility), With<Ghost>>,
    props: Query<(&Prop, &Transform), Without<Ghost>>,
) {
    state.click_consumed = false;
    let over_ui = state.pointer_over_ui;
    let looking = state.looking;
    let wants_keyboard = state.wants_keyboard;
    let Some(placing) = state.placing.as_mut() else {
        return;
    };
    let ghost = *placing
        .ghost
        .get_or_insert_with(|| spawner.spawn(&placing.data, true));

    if keys.just_pressed(KeyCode::KeyR) && !wants_keyboard && !looking {
        let shift = keys.pressed(KeyCode::ShiftLeft);
        let step = settings.rotate_step_deg.to_radians();
        placing.yaw += if shift { -step } else { step };
    }

    let bottom = bounds.bottom_offset(ghost);
    let target = picker.cursor_target(Some(ghost));
    let base_rotation = placing.data.transform().rotation;
    if let Ok((mut transform, mut visibility)) = ghosts.get_mut(ghost) {
        if let Some(point) = target {
            transform.translation = snap(point, &settings) + Vec3::Y * bottom;
            transform.rotation = Quat::from_rotation_y(placing.yaw) * base_rotation;
        }
        let want = if over_ui || target.is_none() { Visibility::Hidden } else { Visibility::Inherited };
        if *visibility != want {
            *visibility = want;
        }
    }

    let commit = if placing.from_drag {
        if !buttons.pressed(MouseButton::Left) {
            Some(!over_ui && target.is_some())
        } else {
            None
        }
    } else if buttons.just_pressed(MouseButton::Left) && !over_ui && !looking {
        Some(true)
    } else {
        None
    };
    let Some(place) = commit else {
        return;
    };
    state.click_consumed = true;

    let mut finished = true;
    if place && let Ok((transform, _)) = ghosts.get(ghost) {
        let before = snapshot(props.iter());
        let placing = state.placing.as_mut().unwrap();
        let mut data = placing.data.clone();
        data.set_transform(transform);
        let entity = spawner.spawn(&data, false);
        let keep_going = !placing.from_drag && keys.pressed(KeyCode::ShiftLeft);
        state.undo.push(before);
        state.redo.clear();
        state.selected = Some(entity);
        state.status = if keep_going {
            format!("Placed {} — keep clicking to place more (release Shift to stop)", data.name)
        } else {
            format!("Placed {}", data.name)
        };
        finished = !keep_going;
    }
    if finished {
        spawner.commands.entity(ghost).despawn();
        state.placing = None;
    }
}

// ---------------------------------------------------------------------------------------------
// Selecting and moving existing props

#[allow(clippy::too_many_arguments)]
fn select_and_drag(
    mut state: ResMut<EditorState>,
    settings: Res<MapEditorSettings>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut picker: Picker,
    bounds: Bounds,
    mut props: Query<(&Prop, &mut Transform), Without<Ghost>>,
    mut actions: MessageWriter<EditorAction>,
) {
    if state.placing.is_some() || state.click_consumed {
        state.hovered = None;
        return;
    }
    let cursor = picker.cursor();

    // Eyedropper: highlight game objects (not props) and copy the one that's clicked.
    if state.picking {
        let hit = if !state.pointer_over_ui && !state.looking {
            picker.cursor_ray().and_then(|ray| picker.cast(ray, None, false))
        } else {
            None
        };
        state.hovered = hit.as_ref().filter(|h| h.prop.is_none()).map(|h| h.entity);
        if buttons.just_pressed(MouseButton::Left) && !state.pointer_over_ui && !state.looking {
            match hit {
                Some(hit) if hit.prop.is_none() => {
                    actions.write(EditorAction::PickFrom(hit.entity));
                }
                Some(_) => state.status = "That's already a map prop: select it and Duplicate instead".into(),
                None => state.status = "Nothing there: click an object in the game world".into(),
            }
        }
        return;
    }

    // Hover highlight.
    state.hovered = if state.drag.is_none() && !state.pointer_over_ui && !state.looking {
        picker
            .cursor_ray()
            .and_then(|ray| picker.cast(ray, None, true))
            .and_then(|hit| hit.prop)
    } else {
        None
    };

    // G grabs the selected prop: it follows the mouse until you click.
    if keys.just_pressed(KeyCode::KeyG)
        && !state.wants_keyboard
        && !state.looking
        && state.drag.is_none()
        && let Some(entity) = state.selected
        && let Ok((_, transform)) = props.get(entity)
    {
        let before = snapshot(props.iter());
        state.undo.push(before);
        state.redo.clear();
        state.drag = Some(DragMove {
            entity,
            press_pos: cursor.unwrap_or_default(),
            start: *transform,
            bottom: bounds.bottom_offset(entity),
            active: true,
            grab: true,
        });
        state.status = "Moving — click to drop, Esc to cancel".into();
        return;
    }

    // Click: select whatever prop is under the cursor (or nothing) and get ready to drag it.
    if state.drag.is_none()
        && buttons.just_pressed(MouseButton::Left)
        && !state.pointer_over_ui
        && !state.looking
    {
        state.selected = state.hovered;
        if let (Some(entity), Some(press_pos)) = (state.hovered, cursor)
            && let Ok((_, transform)) = props.get(entity)
        {
            state.drag = Some(DragMove {
                entity,
                press_pos,
                start: *transform,
                bottom: bounds.bottom_offset(entity),
                active: false,
                grab: false,
            });
        }
        return;
    }

    let Some(drag) = state.drag.as_mut() else {
        return;
    };
    let entity = drag.entity;

    // Finish or cancel.
    if drag.grab {
        if keys.just_pressed(KeyCode::Escape) {
            let start = drag.start;
            if let Ok((_, mut t)) = props.get_mut(entity) {
                *t = start;
            }
            state.undo.pop();
            state.drag = None;
            state.status = "Move cancelled".into();
            return;
        }
        if buttons.just_pressed(MouseButton::Left) {
            state.drag = None;
            state.status = "Moved".into();
            return;
        }
    } else if !buttons.pressed(MouseButton::Left) {
        if drag.active {
            state.status = "Moved".into();
        }
        state.drag = None;
        return;
    }

    // Start dragging once the mouse has moved a few pixels.
    if !drag.active {
        if cursor.is_some_and(|c| c.distance(drag.press_pos) > 4.0) {
            drag.active = true;
            let before = snapshot(props.iter());
            state.undo.push(before);
            state.redo.clear();
        } else {
            return;
        }
    }

    let bottom = state.drag.as_ref().map_or(0.0, |d| d.bottom);
    if let Some(point) = picker.cursor_target(Some(entity))
        && let Ok((_, mut transform)) = props.get_mut(entity)
    {
        transform.translation = snap(point, &settings) + Vec3::Y * bottom;
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    settings: Res<MapEditorSettings>,
    bounds: Bounds,
    camera: Query<&GlobalTransform, With<EditorCamera>>,
) {
    if settings.show_grid
        && let Ok(cam) = camera.single()
    {
        let g = settings.grid_size.max(0.1);
        let c = cam.translation();
        let center = Vec3::new((c.x / g).round() * g, 0.002, (c.z / g).round() * g);
        gizmos.grid(
            Isometry3d::new(center, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            UVec2::splat(60),
            Vec2::splat(g),
            Color::srgba(1.0, 1.0, 1.0, 0.08),
        );
    }

    let mut draw_box = |entity: Entity, color: Color| {
        if let Some((min, max)) = bounds.world_aabb(entity) {
            let size = (max - min).max(Vec3::splat(0.01));
            gizmos.cube(
                Transform::from_translation((min + max) / 2.0).with_scale(size * 1.02),
                color,
            );
        }
    };
    if let Some(hovered) = state.hovered
        && state.selected != Some(hovered)
    {
        draw_box(hovered, Color::srgba(1.0, 1.0, 1.0, 0.5));
    }
    if let Some(selected) = state.selected {
        draw_box(selected, Color::srgb(1.0, 0.65, 0.1));
    }
    if let Some(ghost) = state.placing.as_ref().and_then(|p| p.ghost) {
        draw_box(ghost, Color::srgb(0.3, 0.8, 1.0));
    }
}

fn leave_editor(mut state: ResMut<EditorState>, mut commands: Commands) {
    if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
        commands.entity(ghost).despawn();
    }
    state.drag = None;
    state.hovered = None;
    state.pointer_over_ui = false;
    state.wants_keyboard = false;
}

// ---------------------------------------------------------------------------------------------
// Actions

fn startup_library(
    config: Res<StartupConfig>,
    mut state: ResMut<EditorState>,
    mut actions: MessageWriter<EditorAction>,
) {
    if let Some(map) = &config.startup_map {
        state.map_name = map.clone();
        actions.write(EditorAction::Load(map.clone()));
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_actions(
    mut actions: MessageReader<EditorAction>,
    mut state: ResMut<EditorState>,
    settings: Res<MapEditorSettings>,
    mut spawner: PropSpawner,
    mut props: Query<(Entity, &mut Prop, &mut Transform), Without<Ghost>>,
    prop_materials: Query<&PropMaterial>,
    mut camera: Query<&mut Transform, (With<EditorCamera>, Without<Prop>, Without<Mesh3d>)>,
    mut picker: Picker,
    bounds: Bounds,
    mut pending: Local<std::collections::VecDeque<EditorAction>>,
    mut library: ResMut<Library>,
    eyedropper: Eyedropper,
) {
    pending.extend(actions.read().cloned());
    while let Some(action) = pending.pop_front() {
        // Spawns and despawns only land when commands are applied, so after one of these, wait for
        // the next frame before handling more: every undo snapshot then sees the previous change.
        let structural = matches!(
            action,
            EditorAction::Spawn(_)
                | EditorAction::Delete
                | EditorAction::Duplicate
                | EditorAction::Undo
                | EditorAction::Redo
                | EditorAction::SetCollision(..)
                | EditorAction::Load(_)
                | EditorAction::NewMap
        );
        let state = &mut *state;
        let current = |props: &Query<(Entity, &mut Prop, &mut Transform), Without<Ghost>>| {
            snapshot(props.iter().map(|(_, p, t)| (p, t)))
        };
        match action.clone() {
            EditorAction::Select(entity) => state.selected = entity,

            EditorAction::Spawn(data) => {
                state.undo.push(current(&props));
                state.redo.clear();
                state.selected = Some(spawner.spawn(&data, false));
            }

            EditorAction::BeginPlace { data, from_drag } => {
                if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
                    spawner.commands.entity(ghost).despawn();
                }
                state.status = if from_drag {
                    format!("Drop {} into the world", data.name)
                } else {
                    format!("Click in the world to place {} (Shift: place many, R: rotate, Esc: cancel)", data.name)
                };
                state.placing = Some(Placing { data, ghost: None, from_drag, yaw: 0.0 });
                state.selected = None;
            }

            EditorAction::CancelPlace => {
                if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
                    spawner.commands.entity(ghost).despawn();
                }
                state.picking = false;
                state.status = "Cancelled".into();
            }

            EditorAction::StartPicking => {
                if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
                    spawner.commands.entity(ghost).despawn();
                }
                state.picking = true;
                state.selected = None;
                state.status = "Click any object in the game world to make it placeable (Esc: cancel)".into();
            }

            EditorAction::PickFrom(entity) => {
                state.picking = false;
                match eyedropper.pick(entity) {
                    Ok(entry) => {
                        state.status = format!("Picked {} — click in the world to place copies", entry.label);
                        let data = entry.data.clone();
                        library.picked.retain(|e| e.data.source != entry.data.source);
                        library.picked.insert(0, entry);
                        if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
                            spawner.commands.entity(ghost).despawn();
                        }
                        state.placing = Some(Placing { data, ghost: None, from_drag: false, yaw: 0.0 });
                    }
                    Err(e) => state.status = e,
                }
            }

            EditorAction::Delete => {
                if let Some(entity) = state.selected.take()
                    && props.contains(entity)
                {
                    state.undo.push(current(&props));
                    state.redo.clear();
                    spawner.commands.entity(entity).despawn();
                    state.status = "Deleted".into();
                }
            }

            EditorAction::Duplicate => {
                if let Some(entity) = state.selected
                    && let Ok((_, prop, transform)) = props.get(entity)
                {
                    let mut data = PropData::from_prop(&prop, &transform);
                    data.translation[0] += settings.grid_size.max(1.0);
                    state.undo.push(current(&props));
                    state.redo.clear();
                    state.selected = Some(spawner.spawn(&data, false));
                    state.status = format!("Duplicated {}", data.name);
                }
            }

            EditorAction::Focus => {
                if let Some(entity) = state.selected
                    && let Some((min, max)) = bounds.world_aabb(entity)
                    && let Ok(mut cam) = camera.single_mut()
                {
                    let center = (min + max) / 2.0;
                    let radius = (max - min).length() / 2.0;
                    let forward = *cam.forward();
                    cam.translation = center - forward * (radius * 2.5).max(3.0);
                }
            }

            EditorAction::DropToGround => {
                if let Some(entity) = state.selected
                    && let Some((min, max)) = bounds.world_aabb(entity)
                {
                    let origin = Vec3::new((min.x + max.x) / 2.0, min.y + 0.01, (min.z + max.z) / 2.0);
                    let ray = Ray3d::new(origin, Dir3::NEG_Y);
                    let ground = picker.cast(ray, Some(entity), false).map(|h| h.point.y).unwrap_or(0.0);
                    state.undo.push(current(&props));
                    state.redo.clear();
                    if let Ok((_, _, mut t)) = props.get_mut(entity) {
                        t.translation.y -= min.y - ground;
                    }
                }
            }

            EditorAction::RotateSelected(deg) => {
                if let Some(entity) = state.selected
                    && props.contains(entity)
                {
                    state.undo.push(current(&props));
                    state.redo.clear();
                    if let Ok((_, _, mut t)) = props.get_mut(entity) {
                        t.rotation = Quat::from_rotation_y(deg.to_radians()) * t.rotation;
                    }
                }
            }

            EditorAction::Nudge(offset) => {
                if let Some(entity) = state.selected
                    && props.contains(entity)
                {
                    state.undo.push(current(&props));
                    state.redo.clear();
                    if let Ok((_, _, mut t)) = props.get_mut(entity) {
                        t.translation += offset;
                    }
                }
            }

            EditorAction::PushUndo => {
                state.undo.push(current(&props));
                state.redo.clear();
            }

            EditorAction::Undo | EditorAction::Redo => {
                let undo = matches!(action, EditorAction::Undo);
                let (from, to) = if undo {
                    (&mut state.undo, &mut state.redo)
                } else {
                    (&mut state.redo, &mut state.undo)
                };
                let Some(target) = from.pop() else {
                    state.status = if undo { "Nothing to undo" } else { "Nothing to redo" }.into();
                    continue;
                };
                to.push(current(&props));
                restore(&mut spawner, &props, &target);
                state.selected = None;
                state.status = if undo { "Undo" } else { "Redo" }.into();
            }

            EditorAction::SetTransform(entity, transform) => {
                if let Ok((_, _, mut t)) = props.get_mut(entity) {
                    *t = transform;
                }
            }

            EditorAction::SetColor(entity, color) => {
                if let Ok((_, mut prop, _)) = props.get_mut(entity) {
                    prop.color = color;
                }
                if let Ok(PropMaterial(handle)) = prop_materials.get(entity)
                    && let Some(mut material) = spawner.materials.get_mut(handle)
                {
                    let [r, g, b, a] = color;
                    material.base_color = Color::srgba(r, g, b, a);
                    material.alpha_mode = if a < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque };
                }
            }

            EditorAction::SetCollision(entity, kind) => {
                // Colliders are built at spawn, so rebuild the prop.
                if let Ok((_, prop, transform)) = props.get(entity) {
                    let mut data = PropData::from_prop(&prop, &transform);
                    data.collision = kind;
                    spawner.commands.entity(entity).despawn();
                    let new = spawner.spawn(&data, false);
                    if state.selected == Some(entity) {
                        state.selected = Some(new);
                    }
                }
            }

            EditorAction::Rename(entity, name) => {
                if let Ok((_, mut prop, _)) = props.get_mut(entity) {
                    spawner.commands.entity(entity).insert(Name::new(format!("Prop: {name}")));
                    prop.name = name;
                }
            }

            EditorAction::Save(name) => {
                let name = sanitize(&name);
                let map = MapFile { version: 1, props: current(&props) };
                let path = maps_dir().join(format!("{name}.map.json"));
                let result = std::fs::create_dir_all(maps_dir())
                    .and_then(|_| {
                        serde_json::to_string_pretty(&map)
                            .map_err(std::io::Error::other)
                    })
                    .and_then(|json| std::fs::write(&path, json));
                state.status = match result {
                    Ok(()) => format!("Saved {} props to {}", map.props.len(), path.display()),
                    Err(e) => format!("Save failed: {e}"),
                };
                state.map_name = name;
                library.request_rescan();
            }

            EditorAction::Load(name) => {
                let name = sanitize(&name);
                let path = maps_dir().join(format!("{name}.map.json"));
                if !path.exists() {
                    state.map_name = name.clone();
                    state.status = format!("Map \"{name}\" is new: build it, then Ctrl+S to save");
                    continue;
                }
                let loaded = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str::<MapFile>(&s).map_err(|e| e.to_string()));
                match loaded {
                    Ok(map) => {
                        restore(&mut spawner, &props, &map.props);
                        state.undo.clear();
                        state.redo.clear();
                        state.selected = None;
                        state.status = format!("Loaded {name} ({} props)", map.props.len());
                        state.map_name = name;
                    }
                    Err(e) => {
                        state.status = format!("Could not load {}: {e}", path.display());
                        warn!("{}", state.status);
                    }
                }
            }

            EditorAction::NewMap => {
                state.undo.push(current(&props));
                state.redo.clear();
                restore(&mut spawner, &props, &[]);
                state.selected = None;
                state.map_name = "untitled".into();
                state.status = "New map (Ctrl+Z brings the old one back)".into();
            }

            EditorAction::RefreshLibrary => {
                library.request_rescan();
                state.status = "Refreshed the asset list".into();
            }

            EditorAction::Import(path) => match import_model(&path) {
                Ok(asset_path) => {
                    library.request_rescan();
                    let data = PropData::new(PropSource::Model(asset_path.clone()));
                    if let Some(ghost) = state.placing.take().and_then(|p| p.ghost) {
                        spawner.commands.entity(ghost).despawn();
                    }
                    state.placing = Some(Placing { data, ghost: None, from_drag: false, yaw: 0.0 });
                    state.status = format!("Imported {asset_path} — click in the world to place it");
                }
                Err(e) => state.status = format!("Import failed: {e}"),
            },
        }
        if structural {
            break;
        }
    }
}

/// Replace every prop in the world with `target`.
fn restore(
    spawner: &mut PropSpawner,
    props: &Query<(Entity, &mut Prop, &mut Transform), Without<Ghost>>,
    target: &[PropData],
) {
    for (entity, ..) in props.iter() {
        spawner.commands.entity(entity).despawn();
    }
    for data in target {
        spawner.spawn(data, false);
    }
}

fn sanitize(name: &str) -> String {
    let clean: String = name
        .trim()
        .trim_end_matches(".map.json")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if clean.is_empty() { "untitled".into() } else { clean }
}

fn is_model(path: &Path) -> bool {
    is_gltf_path(&path.to_string_lossy())
}

/// Make a dropped file loadable by the AssetServer: use it in place if it's already under
/// `assets/`, otherwise copy it (and a .gltf's neighbouring buffers/textures) to `assets/imported/`.
fn import_model(path: &Path) -> Result<String, String> {
    if !is_model(path) {
        return Err(format!("{} is not a .glb or .gltf file", path.display()));
    }
    let root = assets_root();
    let canonical_root = root.canonicalize().unwrap_or(root.clone());
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if let Ok(rel) = canonical.strip_prefix(&canonical_root) {
        return Ok(rel.to_string_lossy().replace('\\', "/"));
    }

    let file_name = path.file_name().ok_or("bad file name")?;
    let dest_dir = root.join("imported");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    std::fs::copy(path, dest_dir.join(file_name)).map_err(|e| e.to_string())?;

    let is_gltf = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gltf"));
    if is_gltf && let Some(src_dir) = path.parent() {
        for entry in std::fs::read_dir(src_dir).into_iter().flatten().flatten() {
            let p = entry.path();
            let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            if matches!(ext.as_deref(), Some("bin" | "png" | "jpg" | "jpeg" | "ktx2" | "webp"))
                && let Some(name) = p.file_name()
            {
                let _ = std::fs::copy(&p, dest_dir.join(name));
            }
        }
    }
    Ok(format!("imported/{}", file_name.to_string_lossy()))
}
