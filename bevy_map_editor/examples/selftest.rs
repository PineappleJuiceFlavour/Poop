//! Automated end-to-end check of the editor, no mouse needed:
//!
//!     cargo run --example selftest -- <screenshot dir>
//!
//! Opens and closes the editor, checks the camera / cursor / time handoff, spawns props,
//! saves, undoes, reloads, then drops physics balls onto the props to prove they collide.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_map_editor::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "map editor selftest".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(MapEditorPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, script)
        .run();
}

#[derive(Component)]
struct GameCam;
#[derive(Component)]
struct Ball(&'static str, f32);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Confined;
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(20.0, 40.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(60.0, 1.0, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.32, 0.45, 0.3))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(60.0, 1.0, 60.0),
    ));
    commands.spawn((GameCam, Camera3d::default(), Transform::from_xyz(4.0, 7.0, 14.0).looking_at(Vec3::new(0.0, 1.0, -2.0), Vec3::Y)));
}

fn prop(source: PropSource, pos: Vec3) -> PropData {
    let mut d = PropData::new(source);
    d.translation = pos.to_array();
    d
}

fn check(ok: bool, what: &str, fails: &mut u32) {
    println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
    if !ok {
        *fails += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn script(
    mut frame: Local<u32>,
    mut fails: Local<u32>,
    mut commands: Commands,
    mut next: ResMut<NextState<EditorMode>>,
    mut actions: MessageWriter<EditorAction>,
    game_cam: Single<&Camera, (With<GameCam>, Without<EditorCamera>)>,
    editor_cam: Single<&Camera, With<EditorCamera>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    time: Res<Time<Virtual>>,
    props: Query<&Prop>,
    balls: Query<(&Ball, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    let f = *frame;
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let shot = |commands: &mut Commands, name: &str| {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("{dir}/{name}.png")));
    };
    let fails = &mut *fails;
    match f {
        60 => next.set(EditorMode::Editing),
        70 => {
            check(editor_cam.is_active && !game_cam.is_active, "editor camera took over, game camera off", fails);
            check(cursor.visible && cursor.grab_mode == CursorGrabMode::None, "cursor freed in editor", fails);
            check(time.is_paused(), "game time paused in editor", fails);
            actions.write(EditorAction::NewMap);
            actions.write(EditorAction::Spawn(prop(PropSource::Primitive(Primitive::Cube), Vec3::new(0.0, 3.0, 0.0))));
            actions.write(EditorAction::Spawn(prop(PropSource::Primitive(Primitive::Stairs), Vec3::new(4.0, 0.5, 0.0))));
            actions.write(EditorAction::Spawn(prop(PropSource::Primitive(Primitive::Ramp), Vec3::new(-4.0, 0.5, 3.0))));
            actions.write(EditorAction::Spawn(prop(PropSource::Model("props/crate.glb".into()), Vec3::new(-4.0, 0.0, 0.0))));
            actions.write(EditorAction::Spawn(prop(PropSource::Model("props/stone_arch.glb".into()), Vec3::new(0.0, 0.0, -5.0))));
        }
        100 => {
            check(props.iter().count() == 5, "5 props spawned", fails);
            actions.write(EditorAction::Save("selftest".into()));
            shot(&mut commands, "1_editor");
        }
        110 => {
            actions.write(EditorAction::Undo);
        }
        115 => {
            check(props.iter().count() == 4, &format!("undo removed last prop ({} left)", props.iter().count()), fails);
            actions.write(EditorAction::Redo);
        }
        120 => {
            check(props.iter().count() == 5, "redo restored it", fails);
            actions.write(EditorAction::NewMap);
        }
        125 => {
            check(props.iter().count() == 0, "new map cleared props", fails);
            actions.write(EditorAction::Load("selftest".into()));
        }
        130 => {
            check(props.iter().count() == 5, "loaded map from disk has 5 props", fails);
            next.set(EditorMode::Playing);
        }
        140 => {
            check(!editor_cam.is_active && game_cam.is_active, "game camera restored", fails);
            check(!cursor.visible && cursor.grab_mode == CursorGrabMode::Confined, "game cursor mode restored", fails);
            check(!time.is_paused(), "game time running again", fails);
            // Drop balls: on the floating cube, the crate model, the stairs' top step, and through the arch's gap.
            let mesh = meshes.add(Sphere::new(0.3));
            let mat = materials.add(Color::srgb(1.0, 0.3, 0.2));
            for (name, pos, expect) in [
                ("cube (primitive collider)", Vec3::new(0.0, 8.0, 0.0), 3.8),
                ("crate.glb (mesh collider)", Vec3::new(-4.0, 6.0, 0.0), 1.3),
                ("stairs top step", Vec3::new(4.0, 6.0, -0.75), 1.3),
                ("stone_arch.glb top beam", Vec3::new(0.0, 6.0, -5.0), 4.1),
            ] {
                commands.spawn((
                    Ball(name, expect),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(pos),
                    RigidBody::Dynamic,
                    Collider::sphere(0.3),
                ));
            }
            // Roll one through the arch's doorway: an exact mesh collider leaves the opening passable.
            commands.spawn((
                Ball("roller", 0.0),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(0.0, 0.35, -1.0),
                RigidBody::Dynamic,
                Collider::sphere(0.3),
                LinearVelocity(Vec3::new(0.0, 0.0, -6.0)),
            ));
        }
        400 => {
            for (ball, t) in &balls {
                if ball.0 == "roller" {
                    check(t.translation.z < -5.5, &format!("ball rolled through the arch doorway (z={:.2})", t.translation.z), fails);
                    continue;
                }
                let ok = (t.translation.y - ball.1).abs() < 0.2;
                check(ok, &format!("ball on {} rests at y={:.2} (expected {:.2})", ball.0, t.translation.y, ball.1), fails);
            }
            shot(&mut commands, "2_game");
        }
        420 => {
            println!("SELFTEST DONE: {} failure(s)", *fails);
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
