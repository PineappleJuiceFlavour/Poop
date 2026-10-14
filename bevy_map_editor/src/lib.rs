//! # bevy_map_editor
//!
//! A drop-in, in-game map editor for any Bevy 3D game.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_map_editor::MapEditorPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(MapEditorPlugin::default())
//!     .run();
//! ```
//!
//! Press **F1** in game to open the editor. The game's camera is switched off and a free
//! camera takes over from the same spot; press F1 again and the original camera comes back.
//! Drag props from the asset browser into the world, move them, rotate them, save the map.
//! Saved maps load automatically the next time the game starts (see [`MapEditorPlugin::startup_map`]).

mod camera;
mod editor;
mod library;
mod props;
mod ui;

use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

pub use camera::EditorCamera;
pub use editor::{EditorAction, EditorState, LoadMap};
pub use library::{Library, LibraryEntry, LibraryGroup, MapEditorAppExt, PrefabRegistry};
pub use props::{CollisionKind, MapFile, MeshPart, Primitive, Prop, PropData, PropSource, PropSpawned};

/// Add this to your app. Add it after `DefaultPlugins` (and after `EguiPlugin` if your game uses egui).
#[derive(Clone)]
pub struct MapEditorPlugin {
    /// Key that opens and closes the editor.
    pub toggle_key: KeyCode,
    /// Map (file name inside `assets/maps/`) to load when the game starts, e.g. `"level1"`.
    /// `None` starts with an empty map.
    pub startup_map: Option<String>,
    /// Freeze game time (`Time<Virtual>`) while the editor is open, so the game and its physics stop.
    pub pause_game: bool,
    /// Start the game inside the editor.
    pub start_in_editor: bool,
    /// File extensions in the game's assets folder that load as scenes, shown under "Game files".
    /// glTF is always included; add the formats your game has loaders for (e.g. `"p3d"`).
    pub model_extensions: Vec<String>,
    /// The game's assets folder, if it isn't the default `assets/` (match your `AssetPlugin::file_path`).
    pub assets_dir: Option<PathBuf>,
}

impl Default for MapEditorPlugin {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F1,
            startup_map: None,
            pause_game: true,
            start_in_editor: false,
            model_extensions: Vec::new(),
            assets_dir: None,
        }
    }
}

/// Whether the editor is open. Use the run conditions [`editor_open`] / [`editor_closed`]
/// to stop your gameplay input while the editor has the mouse and keyboard.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorMode {
    #[default]
    Playing,
    Editing,
}

/// Run condition: the map editor is open.
pub fn editor_open(mode: Res<State<EditorMode>>) -> bool {
    *mode.get() == EditorMode::Editing
}

/// Run condition: the map editor is closed (the game is being played).
pub fn editor_closed(mode: Res<State<EditorMode>>) -> bool {
    *mode.get() == EditorMode::Playing
}

/// Runtime settings, editable from the editor toolbar.
#[derive(Resource, Clone)]
pub struct MapEditorSettings {
    pub toggle_key: KeyCode,
    pub pause_game: bool,
    pub snap_enabled: bool,
    pub grid_size: f32,
    pub rotate_step_deg: f32,
    pub camera_speed: f32,
    pub look_sensitivity: f32,
    pub show_grid: bool,
    pub show_colliders_hint: bool,
    /// Lower-case extensions (without the dot) scanned for in the assets folder.
    pub model_extensions: Vec<String>,
}

impl Default for MapEditorSettings {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F1,
            pause_game: true,
            snap_enabled: false,
            grid_size: 1.0,
            rotate_step_deg: 15.0,
            camera_speed: 10.0,
            look_sensitivity: 0.003,
            show_grid: true,
            show_colliders_hint: true,
            model_extensions: vec!["glb".into(), "gltf".into()],
        }
    }
}

/// The schedule the editor UI runs in (the editor camera's own egui context).
#[derive(bevy::ecs::schedule::ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapEditorUiPass;

static ASSETS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Folder that Bevy's `AssetServer` reads from (`<project>/assets` when run through cargo,
/// or [`MapEditorPlugin::assets_dir`]).
pub fn assets_root() -> PathBuf {
    let base = bevy::asset::io::file::FileAssetReader::get_base_path();
    match ASSETS_DIR.get() {
        Some(dir) if dir.is_absolute() => dir.clone(),
        Some(dir) => base.join(dir),
        None => base.join("assets"),
    }
}

/// Folder maps are saved to.
pub fn maps_dir() -> PathBuf {
    assets_root().join("maps")
}

#[derive(Resource)]
pub(crate) struct StartupConfig {
    startup_map: Option<String>,
    start_in_editor: bool,
}

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        // bevy_egui gives the game's first camera a primary context and runs this schedule for it.
        // A game without egui UI of its own never creates it, which makes that pass fail.
        if app.get_schedule(EguiPrimaryContextPass).is_none() {
            app.init_schedule(EguiPrimaryContextPass);
        }

        if let Some(dir) = &self.assets_dir {
            let _ = ASSETS_DIR.set(dir.clone());
        }
        let mut settings = MapEditorSettings {
            toggle_key: self.toggle_key,
            pause_game: self.pause_game,
            ..default()
        };
        for ext in &self.model_extensions {
            let ext = ext.trim_start_matches('.').to_ascii_lowercase();
            if !settings.model_extensions.contains(&ext) {
                settings.model_extensions.push(ext);
            }
        }

        app.init_state::<EditorMode>()
            .insert_resource(settings)
            .insert_resource(StartupConfig {
                startup_map: self.startup_map.clone(),
                start_in_editor: self.start_in_editor,
            })
            .add_plugins((
                props::PropsPlugin,
                library::LibraryPlugin,
                camera::EditorCameraPlugin,
                editor::EditorLogicPlugin,
                ui::EditorUiPlugin,
            ));
    }
}
