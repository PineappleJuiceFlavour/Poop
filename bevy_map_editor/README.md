# bevy_map_editor

A drop-in, in-game map editor for Bevy 3D games. Press **F1** while playing: the game freezes, a free
camera takes over from exactly where the game camera was, and an editor overlay appears. Drag props
into the world, move and rotate them, and save. Press **F1** again and you're back in the game on the
game's own camera, and every prop you placed is solid.

Built for **Bevy 0.19** (bevy_egui 0.42, avian3d 0.7).

## Add it to a game

```toml
# Cargo.toml
[dependencies]
bevy_map_editor = { path = "../bevy_map_editor" }
```

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(PhysicsPlugins::default())        // avian3d, for collision
    .add_plugins(MapEditorPlugin {
        startup_map: Some("level1".into()),        // loads assets/maps/level1.map.json at startup
        ..default()
    })
    // optional: keep gameplay input from reacting while you edit
    .add_systems(Update, player_input.run_if(editor_closed))
    .run();
```

Add the plugin **after** `DefaultPlugins`, and after `EguiPlugin` if your game uses egui itself.

## Controls

| | |
|---|---|
| F1 | open / close the editor |
| Hold right mouse | look around; WASD fly, Q/E down/up, Shift fast, wheel changes speed |
| Mouse wheel / middle drag | dolly / pan |
| Drag a tile from **Assets** | place it where you drop it (rests on whatever surface is under the cursor) |
| Click a tile | then click in the world to place; hold Shift to place many; R rotates the preview |
| Left click / drag a prop | select / slide it over surfaces |
| G | grab the selected prop (click to drop, Esc to cancel) |
| R / Shift+R | rotate by the rotation step |
| Arrows, PgUp/PgDn | nudge by the grid size |
| End | drop to the ground |
| F | focus camera on selection |
| Del, Ctrl+D | delete, duplicate |
| Ctrl+Z / Ctrl+Y | undo / redo |
| Ctrl+S | save |

You can also drop `.glb`/`.gltf` files from Explorer onto the game window. They're copied into
`assets/imported/` and ready to place.

## Using the game's own assets

The **Assets** panel fills itself from the game the editor is plugged into. Nothing needs copying:

| Group | What's in it |
|---|---|
| **Game prefabs** | Objects the game registers in code with `register_prefab` (below). |
| **Game files** | Every scene file in the game's assets folder: `.glb`/`.gltf`, plus any extension listed in `MapEditorPlugin::model_extensions` that the game has a scene loader for. |
| **Loaded by the game** | Whatever the running game has loaded: every scene, every separate mesh inside multi-mesh glTF levels and kits (with their materials), and meshes from any custom loader the game registered (a ported game's own formats). Refreshes every second while the editor is open. |
| **Picked from the world** | Click **🎯 Pick**, then click any object in the game world to copy it into a placeable prop. |

Maps store asset paths (`levels/city.glb#Mesh12/Primitive0`, `levels/spire.tri`), so a saved map
loads the game's real assets again next time.

Objects built in code (a `meshes.add(Cuboid…)` mesh has no file to point to) can be made placeable
as prefabs. The spawn function gets the prop's entity and the `AssetServer`; the map only stores
the name, so your code rebuilds it every time a map loads:

```rust
use bevy_map_editor::MapEditorAppExt;

app.register_prefab("Health pack", |e, assets| {
    e.insert((HealthPack(25), WorldAssetRoot(assets.load("pickups/health.glb#Scene0"))));
});
```

If the prefab inserts its own `Collider`, the editor won't add one.

Game keeps its assets somewhere other than `assets/`, or loads scenes in its own format?

```rust
MapEditorPlugin {
    assets_dir: Some("data".into()),            // match AssetPlugin::file_path
    model_extensions: vec!["scn.ron".into()],   // extra scene formats to list under Game files
    ..default()
}
```

## Props and collision

- **Primitives** (no assets needed): cube, sphere, cylinder, capsule, floor tile, wall, ramp, stairs.
  Each gets its exact matching collider.
- **Models, game meshes, picked objects, prefabs:** collision is either
  **Solid (exact)** (triangle mesh: doorways and arches stay walkable) or **Convex hull**.
- Placed props are static rigid bodies (`RigidBody::Static`).
- Not on avian? Disable the default `avian` feature and give props colliders yourself:

```rust
app.add_observer(|ev: On<PropSpawned>, props: Query<&Prop>, mut commands: Commands| {
    // e.g. insert your rapier collider on ev.entity
});
```

## Maps

Maps are plain JSON in `assets/maps/<name>.map.json` (name, source, transform, color, collision per
prop), so they diff well in git. Load one from game code with
`commands.write_message(LoadMap("level2".into()))`, or send any `EditorAction` (Spawn, Load, Save, …).

## What "any game" means here

This works with any game built on **Bevy** (3D), which covers most Rust games and ports. It isn't
injected into compiled executables: you add one line to the game's source. Other Rust engines
(Fyrox, macroquad, a custom wgpu renderer) have different ECS/rendering APIs. The map format and
editor logic would carry over, but they'd need their own front end.

## Examples

```bash
cargo run --example demo_game                 # third-person game with the editor plugged in
cargo run --example selftest -- <shot dir>    # automated end-to-end check (editor, cameras, undo, save/load, collision + screenshots)
cargo run --example library_selftest -- <dir> # checks taking assets from the game: files, loaded meshes, custom formats, eyedropper, prefabs
```

On Windows, if a build fails with `LNK1104` in a deeply nested folder, the path is over 260
characters. Set `CARGO_TARGET_DIR` to a short path.
