//! A tiny third-person game with the map editor plugged in.
//!
//!     cargo run --example demo_game
//!
//! Play: WASD move, Space jump, mouse look. Press F1 for the map editor, F1 again to play
//! on the map you built: everything you place is solid.

use avian3d::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_map_editor::{MapEditorPlugin, editor_closed};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Demo game — press F1 for the map editor".into(),
                resolution: (1600, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        // The one line that adds the editor:
        .add_plugins(MapEditorPlugin {
            startup_map: Some("demo".into()),
            ..default()
        })
        .add_systems(Startup, setup)
        // Gameplay input only runs while the editor is closed.
        .add_systems(Update, (look, move_player).chain().run_if(editor_closed))
        .add_systems(PostUpdate, follow_camera.before(TransformSystems::Propagate))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct GameCamera {
    yaw: f32,
    pitch: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // The game captures the mouse; the editor frees it and gives it back afterwards.
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Confined;

    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(120.0, 1.0, 120.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.32, 0.45, 0.3))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(120.0, 1.0, 120.0),
    ));

    // A bit of hand-built level that's part of the game itself (not editor props).
    for (i, x) in [-10.0f32, 10.0].into_iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 2.0 + i as f32 * 2.0, 4.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.7, 0.4, 0.35))),
            Transform::from_xyz(x, 1.0 + i as f32, -12.0),
            RigidBody::Static,
            Collider::cuboid(4.0, 2.0 + i as f32 * 2.0, 4.0),
        ));
    }

    // Player.
    commands.spawn((
        Player,
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.95))),
        Transform::from_xyz(0.0, 2.0, 6.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.0),
        LockedAxes::ROTATION_LOCKED,
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
    ));

    // The game's own camera (a third-person follow cam).
    let camera = commands
        .spawn((
            GameCamera { yaw: 0.0, pitch: -0.35 },
            Camera3d::default(),
            Transform::from_xyz(0.0, 4.0, 14.0).looking_at(Vec3::new(0.0, 1.0, 6.0), Vec3::Y),
        ))
        .id();

    commands.spawn((
        UiTargetCamera(camera),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(10),
            ..default()
        },
        Text::new("WASD move | Space jump | Mouse look | F1 map editor"),
    ));
}

fn look(motion: Res<AccumulatedMouseMotion>, mut cam: Single<&mut GameCamera>) {
    cam.yaw -= motion.delta.x * 0.003;
    cam.pitch = (cam.pitch - motion.delta.y * 0.003).clamp(-1.2, 0.6);
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    cam: Single<&GameCamera>,
    player: Single<(Entity, &Transform, &mut LinearVelocity), With<Player>>,
    spatial: SpatialQuery,
) {
    let (entity, transform, mut velocity) = player.into_inner();
    let forward = Quat::from_rotation_y(cam.yaw) * Vec3::NEG_Z;
    let right = Quat::from_rotation_y(cam.yaw) * Vec3::X;
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= right;
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) { 10.0 } else { 6.0 };
    let horizontal = dir.normalize_or_zero() * speed;
    velocity.x = horizontal.x;
    velocity.z = horizontal.z;

    let grounded = spatial
        .cast_ray(
            transform.translation,
            Dir3::NEG_Y,
            1.0,
            true,
            &SpatialQueryFilter::from_excluded_entities([entity]),
        )
        .is_some();
    if grounded && keys.just_pressed(KeyCode::Space) {
        velocity.y = 7.0;
    }
}

fn follow_camera(
    player: Single<&Transform, (With<Player>, Without<GameCamera>)>,
    camera: Single<(&mut Transform, &GameCamera)>,
) {
    let (mut transform, cam) = camera.into_inner();
    let target = player.translation + Vec3::Y * 1.0;
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    transform.translation = target + rotation * Vec3::new(0.0, 0.0, 8.0);
    transform.look_at(target, Vec3::Y);
}
