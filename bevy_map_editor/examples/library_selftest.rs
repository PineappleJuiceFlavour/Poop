//! Automated check of every way the editor takes assets from the host game:
//!
//!     cargo run --example library_selftest -- <screenshot dir>
//!
//! - **Game files**: glTF files in the assets folder are listed.
//! - **Loaded by the game**: each mesh of a multi-mesh glTF kit the game loaded, and meshes from a
//!   custom format (`.tri`, read by the game's own `AssetLoader`, standing in for a ported game).
//! - **Eyedropper**: objects in the game world are copied into placeable props.
//! - **Prefabs**: objects the game registers in code.
//!
//! Every copy is placed, saved, reloaded from disk, and checked for collision.

use avian3d::prelude::*;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::world_serialization::WorldAssetRoot;
use bevy_map_editor::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "map editor library selftest".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .register_asset_loader(TriLoader)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(MapEditorPlugin::default())
        // A prefab: game code builds it, the map only stores its name.
        .register_prefab("Supply crate", |e, assets| {
            e.insert((SupplyCrate, WorldAssetRoot(assets.load("props/crate.glb#Scene0"))));
        })
        .add_systems(Startup, setup)
        .add_systems(Update, script)
        .run();
}

/// A gameplay component the prefab adds, to prove the game's own code ran.
#[derive(Component)]
struct SupplyCrate;

#[derive(Component)]
struct GameCam;
#[derive(Component)]
struct Ground;
#[derive(Component)]
struct Spire;
#[derive(Component)]
struct Kit;

/// The "ported game's" mesh format: a header line, `#` comments, then one triangle
/// (three x y z points) per line.
#[derive(Default, TypePath)]
struct TriLoader;

impl AssetLoader for TriLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn Reader, _: &(), _: &mut LoadContext<'_>) -> Result<Mesh, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = String::from_utf8_lossy(&bytes);
        let bad = |msg: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string());
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("tri 1") {
            return Err(bad("not a tri file"));
        }
        let (mut positions, mut normals) = (Vec::new(), Vec::new());
        for line in lines.map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let n: Vec<f32> = line.split_whitespace().map(|v| v.parse().map_err(|_| bad("bad number"))).collect::<Result<_, _>>()?;
            if n.len() != 9 {
                return Err(bad("a triangle needs 9 numbers"));
            }
            let p = [Vec3::new(n[0], n[1], n[2]), Vec3::new(n[3], n[4], n[5]), Vec3::new(n[6], n[7], n[8])];
            let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            for v in p {
                positions.push(v.to_array());
                normals.push(normal.to_array());
            }
        }
        let count = positions.len() as u32;
        Ok(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_indices(Indices::U32((0..count).collect())))
    }

    fn extensions(&self) -> &[&str] {
        &["tri"]
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(20.0, 40.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Generated in code: there's no asset file for the eyedropper to point to.
    commands.spawn((
        Ground,
        Mesh3d(meshes.add(Cuboid::new(60.0, 1.0, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.32, 0.45, 0.3))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(60.0, 1.0, 60.0),
    ));
    // The game's level: a glTF kit with three separate meshes...
    commands.spawn((Kit, WorldAssetRoot(asset_server.load("levels/town_kit.glb#Scene0")), Transform::from_xyz(-6.0, 0.0, -4.0)));
    // ...and a mesh in the game's own format.
    commands.spawn((
        Spire,
        Mesh3d(asset_server.load("levels/spire.tri")),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.6, 0.2))),
        Transform::from_xyz(6.0, 0.0, -4.0),
    ));
    commands.spawn((GameCam, Camera3d::default(), Transform::from_xyz(0.0, 8.0, 14.0).looking_at(Vec3::new(0.0, 1.0, -2.0), Vec3::Y)));
}

fn check(ok: bool, what: &str, fails: &mut u32) {
    println!("[{}] {what}", if ok { "PASS" } else { "FAIL" });
    if !ok {
        *fails += 1;
    }
}

fn find<'a>(library: &'a Library, group: LibraryGroup, pred: impl Fn(&LibraryEntry) -> bool) -> Option<&'a LibraryEntry> {
    library.group(group).find(|e| pred(e))
}

fn mesh_paths(source: &PropSource) -> Vec<String> {
    match source {
        PropSource::Parts(parts) => parts.iter().map(|p| p.mesh.clone()).collect(),
        _ => Vec::new(),
    }
}

fn placed(entry: &LibraryEntry, pos: Vec3) -> PropData {
    let mut data = entry.data.clone();
    data.translation = pos.to_array();
    data
}

#[allow(clippy::too_many_arguments)]
fn script(
    mut frame: Local<u32>,
    mut fails: Local<u32>,
    mut picked_error: Local<Option<String>>,
    mut commands: Commands,
    mut next: ResMut<NextState<EditorMode>>,
    mut actions: MessageWriter<EditorAction>,
    library: Res<Library>,
    state: Res<EditorState>,
    world: (Single<Entity, With<Kit>>, Single<Entity, With<Spire>>, Single<Entity, With<Ground>>),
    queries: (
        Query<&Children>,
        Query<&Mesh3d>,
        Query<(Entity, &Prop)>,
        Query<(), With<Collider>>,
        Query<(), With<SupplyCrate>>,
        Query<&MeshMaterial3d<StandardMaterial>>,
    ),
    asset_server: Res<AssetServer>,
    mut exit: MessageWriter<AppExit>,
) {
    let (kit, spire, ground) = world;
    let (children, mesh_entities, props, colliders, crates, materials) = queries;
    *frame += 1;
    let f = *frame;
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let fails = &mut *fails;
    let has_collider = |root: Entity| {
        std::iter::once(root).chain(children.iter_descendants(root)).any(|e| colliders.contains(e))
    };
    match f {
        // Give the game time to load its level before the editor opens.
        120 => next.set(EditorMode::Editing),
        140 => {
            actions.write(EditorAction::NewMap);
            check(
                find(&library, LibraryGroup::Files, |e| e.data.source == PropSource::Model("props/crate.glb".into())).is_some(),
                "Game files: props/crate.glb listed",
                fails,
            );
            check(
                find(&library, LibraryGroup::Files, |e| e.data.source == PropSource::Model("levels/town_kit.glb".into())).is_some(),
                "Game files: levels/town_kit.glb listed",
                fails,
            );
            check(
                !library.group(LibraryGroup::Files).any(|e| e.hint.contains("maps/")),
                "Game files: the maps folder is skipped",
                fails,
            );
            for name in ["lamp_post", "bench", "barrel"] {
                let entry = find(&library, LibraryGroup::Loaded, |e| e.label == format!("town_kit › {name}"));
                let ok = entry.is_some_and(|e| {
                    let paths = mesh_paths(&e.data.source);
                    !paths.is_empty() && paths.iter().all(|p| p.starts_with("levels/town_kit.glb#Mesh"))
                });
                check(ok, &format!("Loaded: kit mesh \"{name}\" listed as its own prop"), fails);
            }
            let spire_entry = find(&library, LibraryGroup::Loaded, |e| mesh_paths(&e.data.source) == ["levels/spire.tri"]);
            check(spire_entry.is_some(), "Loaded: mesh from the game's custom .tri format listed", fails);
            check(
                spire_entry.is_some_and(|e| matches!(&e.data.source, PropSource::Parts(p) if p[0].material.is_none())),
                "Loaded: code-made material on the .tri mesh falls back to the prop color",
                fails,
            );
            check(
                find(&library, LibraryGroup::Prefabs, |e| e.data.source == PropSource::Prefab("Supply crate".into())).is_some(),
                "Prefabs: \"Supply crate\" listed",
                fails,
            );
        }
        150 => {
            // Eyedropper on the bench inside the kit scene.
            let bench = children
                .iter_descendants(*kit)
                .filter(|e| mesh_entities.contains(*e))
                .find(|e| {
                    mesh_entities
                        .get(*e)
                        .ok()
                        .and_then(|m| asset_server.get_path(m.0.id()))
                        .is_some_and(|p| p.to_string().starts_with("levels/town_kit.glb#Mesh1/"))
                });
            check(bench.is_some(), "kit scene spawned its meshes", fails);
            if let Some(bench) = bench {
                actions.write(EditorAction::StartPicking);
                actions.write(EditorAction::PickFrom(bench));
            }
        }
        155 => {
            let entry = library.picked.first();
            let paths = entry.map(|e| mesh_paths(&e.data.source)).unwrap_or_default();
            check(
                paths.len() == 2 && paths.iter().all(|p| p.starts_with("levels/town_kit.glb#Mesh1/")),
                &format!("Eyedropper: bench copied with both its material parts ({paths:?})"),
                fails,
            );
            check(state.is_placing(), "Eyedropper: picking starts placing the copy", fails);
            check(
                entry.is_some_and(|e| matches!(&e.data.source, PropSource::Parts(p) if p.iter().all(|p| p.material.is_some()))),
                "Eyedropper: bench keeps the game's materials",
                fails,
            );
            actions.write(EditorAction::CancelPlace);
            actions.write(EditorAction::PickFrom(*spire));
        }
        160 => {
            check(
                library.picked.first().is_some_and(|e| mesh_paths(&e.data.source) == ["levels/spire.tri"]),
                "Eyedropper: custom-format mesh copied",
                fails,
            );
            actions.write(EditorAction::CancelPlace);
            actions.write(EditorAction::PickFrom(*ground));
        }
        165 => {
            *picked_error = Some(state.status.clone());
            check(
                state.status.contains("register_prefab"),
                "Eyedropper: code-generated mesh explains how to make it placeable",
                fails,
            );
            check(library.picked.len() == 2, "Picked group holds the two copies", fails);
            actions.write(EditorAction::CancelPlace);
            // Place one of everything.
            let lamp = find(&library, LibraryGroup::Loaded, |e| e.label == "town_kit › lamp_post").cloned();
            let tri = find(&library, LibraryGroup::Loaded, |e| mesh_paths(&e.data.source) == ["levels/spire.tri"]).cloned();
            let prefab = find(&library, LibraryGroup::Prefabs, |_| true).cloned();
            let kit_file = find(&library, LibraryGroup::Files, |e| e.data.source == PropSource::Model("levels/town_kit.glb".into())).cloned();
            let bench = library.picked.iter().find(|e| mesh_paths(&e.data.source).len() == 2).cloned();
            for (entry, pos) in [
                (lamp, Vec3::new(-3.0, 0.0, 3.0)),
                (tri, Vec3::new(3.0, 0.0, 3.0)),
                (prefab, Vec3::new(0.0, 0.0, 5.0)),
                (kit_file, Vec3::new(0.0, 0.0, 0.0)),
                (bench, Vec3::new(-3.0, 0.0, 6.0)),
            ] {
                if let Some(entry) = entry {
                    actions.write(EditorAction::Spawn(placed(&entry, pos)));
                }
            }
        }
        200 => {
            check(props.iter().count() == 5, &format!("5 props placed from game assets ({})", props.iter().count()), fails);
            check(crates.iter().count() == 1, "prefab ran the game's spawn code", fails);
            actions.write(EditorAction::Save("library_selftest".into()));
            actions.write(EditorAction::NewMap);
        }
        210 => {
            check(props.iter().count() == 0, "map cleared", fails);
            actions.write(EditorAction::Load("library_selftest".into()));
        }
        240 => {
            check(props.iter().count() == 5, "all 5 reloaded from the saved map", fails);
            check(crates.iter().count() == 1, "prefab rebuilt by game code on load", fails);
            let text = std::fs::read_to_string(maps_dir().join("library_selftest.map.json")).unwrap_or_default();
            check(
                text.contains("levels/spire.tri") && text.contains("Supply crate") && text.contains("town_kit.glb#Mesh"),
                "map file stores asset paths and the prefab name",
                fails,
            );
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("{dir}/library_editor.png")));
            next.set(EditorMode::Playing);
        }
        400 => {
            for (entity, prop) in &props {
                check(has_collider(entity), &format!("{} is solid", prop.name), fails);
            }
            // Every saved material path must load again (no "Material not found").
            let mut bad = Vec::new();
            for (entity, _) in &props {
                for e in children.iter_descendants(entity) {
                    if let Ok(m) = materials.get(e)
                        && let bevy::asset::LoadState::Failed(err) = asset_server.load_state(&m.0)
                    {
                        bad.push(err.to_string());
                    }
                }
            }
            check(bad.is_empty(), &format!("every reloaded material resolved {bad:?}"), fails);
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("{dir}/library_game.png")));
        }
        420 => {
            let _ = std::fs::remove_file(maps_dir().join("library_selftest.map.json"));
            println!("LIBRARY SELFTEST DONE: {} failure(s)", *fails);
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
