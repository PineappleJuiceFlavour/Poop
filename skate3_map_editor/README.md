# Map editor for Skate 3 Rust Engine

Adds the map editor to [Skate 3 Rust Engine](https://github.com/SK8-ENGINE/skate-3-rust-engine)
(experimental build 49, commit `2e16697`). Press **F1** while skating: the game freezes, a free
camera takes over, and you can drag props into the map. Press **F1** again and skate on them.
Everything you place is solid, and rails, ledges and copied map objects are grindable.

## Install

The game has no way to load an editor into the downloaded `skate3rust.exe`, so this builds the game
from its source code with the editor added. You need, once:

- [Git](https://git-scm.com/download/win)
- [Rust](https://rustup.rs) with the default MSVC toolchain (the Visual Studio C++ build tools that
  rustup offers to install)
- [LLVM](https://github.com/llvm/llvm-project/releases) in its default location
  (`winget install LLVM.LLVM` works)

Then:

1. Copy this `skate3_map_editor` folder **into your game folder**, the one with `skate3rust.exe`
   and `data` in it.
2. Run `skate3_map_editor\INSTALL.bat`. It downloads the game's source next to it, adds the editor
   and builds. The first build takes a while.
3. Play with `skate-3-rust-engine\PLAY.bat`. It uses the Skate 3 data your game already converted.

Your normal `skate3rust.exe` is untouched and still updates as usual.

The build itself happens in `%LOCALAPPDATA%\sk8t` (around 10 GB), because Windows build tools fail
on paths over 260 characters and game folders in Downloads are deep. You can delete that folder
after a build; the next build just takes longer.

## Using it

| | |
|---|---|
| F1 | open / close the editor |
| Hold right mouse | look around; WASD fly, Q/E down/up, Shift fast, wheel changes speed |
| Drag a tile from **Assets** | drop it on the map where the cursor is |
| Left click / drag a prop | select / slide it over the ground |
| R / Shift+R | rotate |
| Del, Ctrl+D | delete, duplicate |
| Ctrl+Z / Ctrl+Y | undo / redo |
| Ctrl+S | save |

- **Each map has its own layout.** It loads when you go to that map, and Ctrl+S saves it to
  `editor_maps\<map name>.map.json` in your game data folder.
- **Copy the map's own objects:** click **🎯 Pick from game**, then click a bench, rail, ledge, bin,
  sign or ramp in the map. The editor cuts that object out of the map (with its loose parts, like a
  bench's legs) and you can place as many copies as you like. A copy takes the original's look, its
  collision surfaces and its **grind lines**, so it should grind like the original. Copies look exactly
  like the original on the map they came from, and plain grey on other maps. They're saved in
  `editor_maps\pieces\` and listed under **Game objects** next time.
  The ground, walls and buildings are too big to copy.
- **Built-in props:** a grindable **Rail** and **Ledge**, plus ramps, stairs, boxes, walls and floor
  tiles.
- **Your own models:** drop a `.glb`/`.gltf` file onto the game window. It's copied into the
  `imported` folder and listed under **Game files**. 3D models made for the game's mods work too.
- **Models from your game discs:** drop a disc image (`.iso`, `.xiso`, `.gcm`) of a game you own
  onto the window while the editor is open. It reads Xbox, Xbox 360, PS2, PSP, PC (ISO 9660) and
  GameCube discs, searches every file, and extracts each model it can use: `.glb`/`.gltf` files,
  `.obj` files (converted), and `.glb` models stored inside the game's archives. They go to
  `imported\<disc name>\` and show up under **Game files**. Progress shows in the status bar.
  Most games store their models in their own formats, so many discs give few or no models;
  `imported\<disc name>\_disc_report.txt` lists what the disc holds.
- **Collision** uses the game's own physics, the same system mod objects use, so the skater
  treats props like real level geometry. Each prop's **Collision** setting in the inspector:
  **Solid (exact)** keeps holes and doorways open, **Convex hull** wraps the whole model (cheaper,
  but fills gaps), **None** makes it decoration only.

## Limits

- Grind lines exist only on the Rail, the Ledge and copies of the map's own objects. Imported
  models and the other built-ins are solid but not grindable.
- Copies get their grind lines and map collision when you go back to skating (F1), not while the
  editor is open.
- A copied piece's material only exists on the map it came from; on other maps it's plain grey.
- While the editor is open the game is paused. It isn't made for online sessions: edit offline.

## What it changes in the game

`skate3-map-editor.patch` is a normal git patch against commit `2e16697`:

- `crates/skate-map-editor/`: the editor, ported to the game's Bevy 0.18.1
- `crates/skate-game/src/modding/editor_solids.rs`: makes props solid: one static body per prop in
  the mod physics world (which the board's collision already queries). Primitives use their exact
  convex shapes; models use the same convex decomposition mods get for `Shape::Model`
- `crates/skate-game/src/map_pieces.rs`: copies map objects (looks, collision triangles with their
  surface codes, grind lines) and adds every placed copy, Rail and Ledge to the map's own static
  world and grind lines while playing
- `crates/skate-map-editor/src/disc.rs`: reads disc images and extracts their models
- `crates/skate-game/src/map_editor.rs`: adds the editor, loads layouts per map, renders the editor
  view through the game's own pipeline (offscreen target, retail tone mapping)
- Small edits: `camera.rs` and the pause menu stand by while the editor is open; `app.rs`,
  `main.rs`, `modding/mod.rs` and the `Cargo.toml`s register it

If you share a build with this in it, the game's GPL-3.0 license requires sharing its source,
for example this patch.
