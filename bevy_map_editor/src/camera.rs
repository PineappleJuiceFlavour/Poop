//! Editor free camera, and handing control between it and the game's own camera.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_egui::EguiSchedule;

use crate::editor::EditorState;
use crate::{EditorMode, MapEditorSettings, MapEditorUiPass, StartupConfig, editor_open};

/// The editor's free camera. It only renders while the editor is open.
#[derive(Component, Default)]
pub struct EditorCamera {
    pub yaw: f32,
    pub pitch: f32,
}

/// What the game looked like before the editor opened, so it can be put back exactly.
#[derive(Resource, Default)]
struct Handoff {
    game_cameras: Vec<Entity>,
    cursor: Option<(bool, CursorGrabMode)>,
    time_was_paused: bool,
}

pub(crate) struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Handoff>()
            .add_systems(Startup, (spawn_editor_camera, start_in_editor))
            .add_systems(Update, toggle_editor)
            .add_systems(OnEnter(EditorMode::Editing), enter_editor)
            .add_systems(OnExit(EditorMode::Editing), exit_editor)
            .add_systems(Update, fly_camera.run_if(editor_open));
    }
}

fn spawn_editor_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Map Editor Camera"),
        EditorCamera::default(),
        Camera3d::default(),
        Camera {
            // Draw on top of anything else, but only while the editor is open.
            order: 1000,
            is_active: false,
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        // The editor UI lives in its own egui context on this camera, so it never clashes
        // with any egui UI the game already has.
        EguiSchedule::new(MapEditorUiPass),
    ));
}

fn start_in_editor(config: Res<StartupConfig>, mut next: ResMut<NextState<EditorMode>>) {
    if config.start_in_editor {
        next.set(EditorMode::Editing);
    }
}

fn toggle_editor(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<MapEditorSettings>,
    state: Res<EditorState>,
    mode: Res<State<EditorMode>>,
    mut next: ResMut<NextState<EditorMode>>,
) {
    if !keys.just_pressed(settings.toggle_key) || state.wants_keyboard {
        return;
    }
    next.set(match mode.get() {
        EditorMode::Playing => EditorMode::Editing,
        EditorMode::Editing => EditorMode::Playing,
    });
}

#[allow(clippy::type_complexity)]
fn enter_editor(
    mut handoff: ResMut<Handoff>,
    mut game_cams: Query<
        (Entity, &mut Camera, &GlobalTransform, &Transform, Option<&Projection>, Has<Camera3d>),
        Without<EditorCamera>,
    >,
    editor_cam: Single<
        (&mut Camera, &mut Transform, &mut Projection, &mut EditorCamera),
        With<EditorCamera>,
    >,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
    settings: Res<MapEditorSettings>,
) {
    let (mut ed_camera, mut ed_transform, mut ed_projection, mut ed) = editor_cam.into_inner();

    // Start the free camera exactly where the game's main 3D camera is.
    let source = game_cams
        .iter()
        .filter(|(_, cam, _, _, _, is_3d)| cam.is_active && *is_3d)
        .max_by_key(|(_, cam, ..)| cam.order)
        .map(|(_, _, global, local, projection, _)| {
            let transform = if *global == GlobalTransform::IDENTITY {
                *local
            } else {
                global.compute_transform()
            };
            (transform, projection.cloned())
        });
    if let Some((transform, projection)) = source {
        ed_transform.translation = transform.translation;
        ed_transform.rotation = transform.rotation;
        ed_transform.scale = Vec3::ONE;
        if let Some(projection) = projection {
            *ed_projection = projection;
        }
    }
    let (yaw, pitch, _) = ed_transform.rotation.to_euler(EulerRot::YXZ);
    ed.yaw = yaw;
    ed.pitch = pitch;

    // Switch every game camera off and remember which ones were on.
    handoff.game_cameras.clear();
    for (entity, mut cam, ..) in &mut game_cams {
        if cam.is_active {
            cam.is_active = false;
            handoff.game_cameras.push(entity);
        }
    }
    ed_camera.is_active = true;

    if let Ok(mut cursor) = cursor.single_mut() {
        handoff.cursor = Some((cursor.visible, cursor.grab_mode));
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }

    handoff.time_was_paused = time.is_paused();
    if settings.pause_game {
        time.pause();
    }
}

fn exit_editor(
    mut handoff: ResMut<Handoff>,
    mut game_cams: Query<&mut Camera, Without<EditorCamera>>,
    mut editor_cam: Single<&mut Camera, With<EditorCamera>>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
    mut state: ResMut<EditorState>,
) {
    editor_cam.is_active = false;
    for entity in handoff.game_cameras.drain(..) {
        if let Ok(mut cam) = game_cams.get_mut(entity) {
            cam.is_active = true;
        }
    }
    if let (Ok(mut cursor), Some((visible, grab))) = (cursor.single_mut(), handoff.cursor.take()) {
        cursor.visible = visible;
        cursor.grab_mode = grab;
    }
    if !handoff.time_was_paused {
        time.unpause();
    }
    state.looking = false;
}

#[allow(clippy::too_many_arguments)]
fn fly_camera(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time<Real>>,
    mut settings: ResMut<MapEditorSettings>,
    mut state: ResMut<EditorState>,
    camera: Single<(&mut Transform, &mut EditorCamera)>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let (mut transform, mut cam) = camera.into_inner();
    let dt = time.delta_secs();

    // Hold right mouse to look around and fly (WASD + Q/E), like most 3D editors.
    if buttons.just_pressed(MouseButton::Right) && !state.pointer_over_ui {
        state.looking = true;
    }
    if !buttons.pressed(MouseButton::Right) {
        state.looking = false;
    }
    if let Ok(mut cursor) = cursor.single_mut() {
        let (visible, grab) = if state.looking {
            (false, CursorGrabMode::Confined)
        } else {
            (true, CursorGrabMode::None)
        };
        if cursor.visible != visible {
            cursor.visible = visible;
        }
        if cursor.grab_mode != grab {
            cursor.grab_mode = grab;
        }
    }

    let scroll_lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };

    if state.looking {
        cam.yaw -= motion.delta.x * settings.look_sensitivity;
        cam.pitch = (cam.pitch - motion.delta.y * settings.look_sensitivity).clamp(-1.54, 1.54);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);

        let mut dir = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            dir += *transform.forward();
        }
        if keys.pressed(KeyCode::KeyS) {
            dir -= *transform.forward();
        }
        if keys.pressed(KeyCode::KeyD) {
            dir += *transform.right();
        }
        if keys.pressed(KeyCode::KeyA) {
            dir -= *transform.right();
        }
        if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Space) {
            dir += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::ControlLeft) {
            dir -= Vec3::Y;
        }
        let boost = if keys.pressed(KeyCode::ShiftLeft) { 4.0 } else { 1.0 };
        transform.translation += dir.normalize_or_zero() * settings.camera_speed * boost * dt;

        // Scroll while flying changes fly speed.
        if scroll_lines != 0.0 {
            settings.camera_speed = (settings.camera_speed * 1.15f32.powf(scroll_lines)).clamp(0.5, 500.0);
        }
    } else if !state.pointer_over_ui {
        // Scroll to dolly, middle mouse to pan.
        if scroll_lines != 0.0 {
            let forward = *transform.forward();
            transform.translation += forward * scroll_lines * settings.camera_speed * 0.15;
        }
        if buttons.pressed(MouseButton::Middle) {
            let right = *transform.right();
            let up = *transform.up();
            let k = settings.camera_speed * 0.004;
            transform.translation += (-right * motion.delta.x + up * motion.delta.y) * k;
        }
    }
}
