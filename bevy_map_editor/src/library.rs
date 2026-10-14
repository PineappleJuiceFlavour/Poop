//! The asset browser's catalogue: everything the host game has to offer.
//!
//! - **Game prefabs**: objects the game registers in code ([`MapEditorAppExt::register_prefab`]).
//! - **Game files**: scene files in the game's assets folder.
//! - **Loaded by the game**: scenes and meshes the running game has loaded, in any format it has a
//!   loader for, including every separate mesh inside multi-mesh glTF levels and kits.
//! - **Picked**: objects copied from the game world with the eyedropper.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::props::{MeshPart, Prop, PropData, PropSource, is_gltf_path, path_stem};
use crate::{EditorMode, MapEditorSettings, assets_root, maps_dir};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryGroup {
    Prefabs,
    Files,
    Loaded,
    Picked,
}

impl LibraryGroup {
    pub fn title(self) -> &'static str {
        match self {
            LibraryGroup::Prefabs => "Game prefabs",
            LibraryGroup::Files => "Game files",
            LibraryGroup::Loaded => "Loaded by the game",
            LibraryGroup::Picked => "Picked from the world",
        }
    }
}

/// One placeable thing in the asset browser.
#[derive(Clone, Debug)]
pub struct LibraryEntry {
    pub group: LibraryGroup,
    pub label: String,
    pub hint: String,
    /// The prop that gets placed (source, default scale, collision…).
    pub data: PropData,
}

/// What the asset browser shows. Rebuilt when the editor opens, every second while it is open
/// (so newly loaded game assets appear), and on Refresh.
#[derive(Resource, Default)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
    pub picked: Vec<LibraryEntry>,
    pub maps: Vec<String>,
    pub(crate) rescan: bool,
}

impl Library {
    pub fn group(&self, group: LibraryGroup) -> impl Iterator<Item = &LibraryEntry> {
        let picked = (group == LibraryGroup::Picked).then_some(self.picked.iter());
        self.entries
            .iter()
            .filter(move |e| e.group == group)
            .chain(picked.into_iter().flatten())
    }

    /// Scan files and maps again on the next update.
    pub fn request_rescan(&mut self) {
        self.rescan = true;
    }
}

pub type PrefabSpawner = Arc<dyn Fn(&mut EntityCommands, &AssetServer) + Send + Sync>;

/// Objects the game offers to the editor. The spawn function receives the prop's entity (which
/// already has its transform) plus the `AssetServer`, and inserts whatever the object needs:
/// meshes, scenes, gameplay components, its own colliders.
#[derive(Resource, Default, Clone)]
pub struct PrefabRegistry {
    entries: Vec<(String, PrefabSpawner)>,
}

impl PrefabRegistry {
    pub fn register(&mut self, name: impl Into<String>, spawn: impl Fn(&mut EntityCommands, &AssetServer) + Send + Sync + 'static) {
        let name = name.into();
        self.entries.retain(|(n, _)| *n != name);
        self.entries.push((name, Arc::new(spawn)));
    }

    pub fn get(&self, name: &str) -> Option<&PrefabSpawner> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, f)| f)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(n, _)| n.as_str())
    }
}

pub trait MapEditorAppExt {
    /// Make one of your game's objects placeable in the editor:
    ///
    /// ```ignore
    /// app.register_prefab("Health pack", |e, assets| {
    ///     e.insert((HealthPack(25), WorldAssetRoot(assets.load("pickups/health.glb#Scene0"))));
    /// });
    /// ```
    ///
    /// Maps store only the name, so the object is rebuilt by your code every time a map loads.
    /// If the spawn function inserts a `Collider` on the entity, the editor won't add its own.
    fn register_prefab(
        &mut self,
        name: impl Into<String>,
        spawn: impl Fn(&mut EntityCommands, &AssetServer) + Send + Sync + 'static,
    ) -> &mut Self;
}

impl MapEditorAppExt for App {
    fn register_prefab(
        &mut self,
        name: impl Into<String>,
        spawn: impl Fn(&mut EntityCommands, &AssetServer) + Send + Sync + 'static,
    ) -> &mut Self {
        self.init_resource::<PrefabRegistry>();
        self.world_mut().resource_mut::<PrefabRegistry>().register(name, spawn);
        self
    }
}

pub(crate) struct LibraryPlugin;

impl Plugin for LibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Library>()
            .init_resource::<PrefabRegistry>()
            .add_systems(Startup, |mut library: ResMut<Library>| library.rescan = true)
            .add_systems(OnEnter(EditorMode::Editing), |mut library: ResMut<Library>| {
                library.rescan = true
            })
            .add_systems(Update, update_library);
    }
}

fn update_library(
    mut library: ResMut<Library>,
    settings: Res<MapEditorSettings>,
    mode: Res<State<EditorMode>>,
    time: Res<Time<Real>>,
    mut last: Local<f32>,
    mut files: Local<Vec<LibraryEntry>>,
    live: LiveAssets,
) {
    let now = time.elapsed_secs();
    let due = *mode.get() == EditorMode::Editing && now - *last > 1.0;
    if !library.rescan && !due {
        return;
    }
    *last = now;
    if library.rescan {
        library.rescan = false;
        *files = scan_files(&settings.model_extensions);
        library.maps = scan_maps();
    }

    let file_paths: HashSet<&str> = files
        .iter()
        .filter_map(|e| match &e.data.source {
            PropSource::Model(path) => Some(path.as_str()),
            _ => None,
        })
        .collect();
    let mut entries = live.prefab_entries();
    entries.extend(files.iter().cloned());
    entries.extend(live.loaded_entries(&file_paths));
    library.entries = entries;
}

fn entry(group: LibraryGroup, label: String, hint: String, source: PropSource) -> LibraryEntry {
    let mut data = PropData::new(source);
    data.name = label.clone();
    LibraryEntry { group, label, hint, data }
}

// ---------------------------------------------------------------------------------------------
// Files on disk

pub(crate) fn scan_files(extensions: &[String]) -> Vec<LibraryEntry> {
    let root = assets_root();
    let mut paths = Vec::new();
    collect_files(&root, &root, 0, extensions, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let hint = format!("assets/{path}");
            entry(LibraryGroup::Files, path_stem(&path), hint, PropSource::Model(path))
        })
        .collect()
}

fn collect_files(root: &Path, dir: &Path, depth: usize, extensions: &[String], out: &mut Vec<String>) {
    if depth > 8 || out.len() > 5000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth == 0 && path.file_name().is_some_and(|n| n == "maps") {
                continue;
            }
            collect_files(root, &path, depth + 1, extensions, out);
        } else if has_extension(&path, extensions)
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn has_extension(path: &Path, extensions: &[String]) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    // Matches multi-part extensions like "scn.ron" too.
    extensions.iter().any(|ext| name.ends_with(&format!(".{ext}")))
}

pub(crate) fn scan_maps() -> Vec<String> {
    let mut maps: Vec<String> = std::fs::read_dir(maps_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".map.json").map(str::to_owned)
        })
        .collect();
    maps.sort();
    maps
}

// ---------------------------------------------------------------------------------------------
// Whatever the running game has loaded

const MAX_LOADED: usize = 5000;

#[derive(SystemParam)]
pub(crate) struct LiveAssets<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    gltfs: Option<Res<'w, Assets<Gltf>>>,
    gltf_meshes: Option<Res<'w, Assets<GltfMesh>>>,
    scenes: Option<Res<'w, Assets<WorldAsset>>>,
    meshes: Res<'w, Assets<Mesh>>,
    users: Query<
        'w,
        's,
        (&'static Mesh3d, &'static MeshMaterial3d<StandardMaterial>, Option<&'static ChildOf>),
        Without<Prop>,
    >,
    names: Query<'w, 's, &'static Name>,
    prefabs: Res<'w, PrefabRegistry>,
}

impl LiveAssets<'_, '_> {
    fn prefab_entries(&self) -> Vec<LibraryEntry> {
        self.prefabs
            .names()
            .map(|name| {
                entry(
                    LibraryGroup::Prefabs,
                    name.to_string(),
                    "Registered by the game".into(),
                    PropSource::Prefab(name.to_string()),
                )
            })
            .collect()
    }

    fn loaded_entries(&self, files: &HashSet<&str>) -> Vec<LibraryEntry> {
        let mut out = Vec::new();
        let server = &self.asset_server;

        // Scenes (any loader). The first scene of a file already listed under "Game files" is skipped.
        if let Some(scenes) = &self.scenes {
            for (id, _) in scenes.iter() {
                let Some(path) = server.get_path(id) else {
                    continue;
                };
                let path = path.to_string();
                let (base, label) = path.split_once('#').unwrap_or((path.as_str(), ""));
                if files.contains(base) && (label.is_empty() || label == "Scene0") {
                    continue;
                }
                let name = if label.is_empty() { path_stem(base) } else { format!("{} › {label}", path_stem(base)) };
                out.push(entry(LibraryGroup::Loaded, name, format!("Scene: {path}"), PropSource::Model(path.clone())));
            }
        }

        // Every mesh of glTF files that hold several (levels, kits), each with its own materials.
        let mut covered: HashSet<String> = HashSet::new();
        if let (Some(gltfs), Some(gltf_meshes)) = (&self.gltfs, &self.gltf_meshes) {
            for (id, gltf) in gltfs.iter() {
                if gltf.meshes.len() < 2 {
                    continue;
                }
                let Some(path) = server.get_path(id) else {
                    continue;
                };
                let base = path.to_string();
                let base = base.split('#').next().unwrap_or_default().to_string();
                covered.insert(base.clone());
                for (i, handle) in gltf.meshes.iter().enumerate() {
                    let Some(mesh) = gltf_meshes.get(handle) else {
                        continue;
                    };
                    let parts: Option<Vec<MeshPart>> = mesh
                        .primitives
                        .iter()
                        .map(|primitive| {
                            let mesh_path = server.get_path(primitive.mesh.id())?.to_string();
                            let material = match &primitive.material {
                                Some(m) => format!("{}/std", server.get_path(m.id())?),
                                None => format!("{base}#DefaultMaterial/std"),
                            };
                            Some(MeshPart::new(mesh_path, Some(material), Transform::IDENTITY))
                        })
                        .collect();
                    let Some(parts) = parts.filter(|p| !p.is_empty()) else {
                        continue;
                    };
                    let name = if mesh.name.is_empty() { format!("Mesh{i}") } else { mesh.name.clone() };
                    let hint = format!("Mesh \"{name}\" in {base}");
                    out.push(entry(
                        LibraryGroup::Loaded,
                        format!("{} › {name}", path_stem(&base)),
                        hint,
                        PropSource::Parts(parts),
                    ));
                    if out.len() >= MAX_LOADED {
                        return out;
                    }
                }
            }
        }

        // The material each mesh is drawn with, and the name of the node drawing it.
        let mut materials: HashMap<AssetId<Mesh>, String> = HashMap::new();
        let mut node_names: HashMap<AssetId<Mesh>, String> = HashMap::new();
        for (mesh, material, parent) in &self.users {
            if let Some(path) = server.get_path(material.0.id()) {
                materials.insert(mesh.0.id(), path.to_string());
            }
            if let Some(name) = parent.and_then(|p| self.names.get(p.parent()).ok()) {
                node_names.insert(mesh.0.id(), name.as_str().to_string());
            }
        }

        // Games usually load just `level.glb#Scene0`, which lets Bevy drop the `Gltf` asset used
        // above. Its meshes are still in memory though: group them by glTF mesh (`#MeshN`).
        let mut gltf_groups: BTreeMap<String, BTreeMap<String, Vec<(String, AssetId<Mesh>)>>> = BTreeMap::new();
        for (id, _) in self.meshes.iter() {
            let Some(path) = server.get_path(id) else {
                continue;
            };
            let path = path.to_string();
            let Some((base, label)) = path.split_once('#') else {
                continue;
            };
            if !is_gltf_path(base) || covered.contains(base) {
                continue;
            }
            let Some(group) = label.split('/').next().filter(|g| g.starts_with("Mesh")) else {
                continue;
            };
            gltf_groups
                .entry(base.to_string())
                .or_default()
                .entry(group.to_string())
                .or_default()
                .push((path.clone(), id));
        }
        for (base, groups) in gltf_groups {
            // A single-mesh file is already placeable whole under "Game files".
            if groups.len() < 2 && files.contains(base.as_str()) {
                continue;
            }
            for (group, mut primitives) in groups {
                primitives.sort();
                let name = primitives
                    .iter()
                    .find_map(|(_, id)| node_names.get(id).cloned())
                    .unwrap_or_else(|| group.clone());
                let parts = primitives
                    .iter()
                    .map(|(path, id)| MeshPart::new(path.clone(), materials.get(id).cloned(), Transform::IDENTITY))
                    .collect();
                out.push(entry(
                    LibraryGroup::Loaded,
                    format!("{} › {name}", path_stem(&base)),
                    format!("Mesh \"{name}\" in {base}"),
                    PropSource::Parts(parts),
                ));
                if out.len() >= MAX_LOADED {
                    return out;
                }
            }
        }

        // Meshes from any other loader (custom formats of ported games), with the material the
        // game draws them with.
        for (id, _) in self.meshes.iter() {
            let Some(path) = server.get_path(id) else {
                continue;
            };
            let path = path.to_string();
            if is_gltf_path(&path) {
                continue;
            }
            let part = MeshPart::new(path.clone(), materials.get(&id).cloned(), Transform::IDENTITY);
            let label = match path.split_once('#') {
                Some((base, label)) => format!("{} › {label}", path_stem(base)),
                None => path_stem(&path),
            };
            out.push(entry(LibraryGroup::Loaded, label, format!("Mesh: {path}"), PropSource::Parts(vec![part])));
            if out.len() >= MAX_LOADED {
                break;
            }
        }

        out.sort_by(|a, b| a.label.cmp(&b.label));
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Eyedropper: turn an object in the game world into something placeable

#[derive(SystemParam)]
pub(crate) struct Eyedropper<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    parts: Query<
        'w,
        's,
        (&'static Mesh3d, Option<&'static MeshMaterial3d<StandardMaterial>>, &'static Transform),
        Without<Prop>,
    >,
    names: Query<'w, 's, &'static Name>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    scenes: Query<'w, 's, &'static WorldAssetRoot>,
}

impl Eyedropper<'_, '_> {
    pub fn pick(&self, entity: Entity) -> Result<LibraryEntry, String> {
        let Ok((_, _, own_transform)) = self.parts.get(entity) else {
            return Err("That object has no mesh to copy".into());
        };
        let parent = self.parents.get(entity).ok().map(|c| c.parent());

        // glTF splits a mesh into one entity per material, all sharing the node's transform:
        // take those siblings together so the copy looks the same.
        let group: Vec<Entity> = parent
            .and_then(|p| self.children.get(p).ok())
            .map(|siblings| {
                siblings
                    .iter()
                    .filter(|s| self.parts.get(*s).is_ok_and(|(_, _, t)| t == own_transform))
                    .collect()
            })
            .unwrap_or_else(|| vec![entity]);

        let parts: Option<Vec<MeshPart>> = group
            .iter()
            .map(|e| {
                let (mesh, material, _) = self.parts.get(*e).ok()?;
                let mesh_path = self.asset_server.get_path(mesh.0.id())?.to_string();
                let material = material
                    .and_then(|m| self.asset_server.get_path(m.0.id()))
                    .map(|p| p.to_string());
                Some(MeshPart::new(mesh_path, material, Transform::IDENTITY))
            })
            .collect();

        let name_of = |e: Entity| self.names.get(e).ok().map(|n| n.as_str().to_string());
        let (source, name) = match parts {
            Some(parts) if !parts.is_empty() => {
                let name = parent
                    .and_then(name_of)
                    .or_else(|| name_of(entity))
                    .unwrap_or_else(|| path_stem(&parts[0].mesh));
                (PropSource::Parts(parts), name)
            }
            _ => {
                // The mesh has no file of its own; fall back to the scene it belongs to.
                let scene = std::iter::once(entity)
                    .chain(self.parents.iter_ancestors(entity))
                    .find_map(|e| self.scenes.get(e).ok())
                    .and_then(|root| self.asset_server.get_path(root.0.id()));
                match scene {
                    Some(path) => {
                        let path = path.to_string();
                        let name = path_stem(&path);
                        (PropSource::Model(path), name)
                    }
                    None => {
                        return Err("That object's mesh was generated in code, so there's no asset to point \
                                    to. Register it with app.register_prefab(..) to make it placeable."
                            .into());
                    }
                }
            }
        };

        let hint = match &source {
            PropSource::Parts(parts) => format!("Picked: {}", parts[0].mesh),
            PropSource::Model(path) => format!("Picked scene: {path}"),
            _ => String::new(),
        };
        let mut picked = entry(LibraryGroup::Picked, name, hint, source);
        if let Ok(global) = self.globals.get(entity) {
            picked.data.scale = global.compute_transform().scale.to_array();
        }
        Ok(picked)
    }
}
