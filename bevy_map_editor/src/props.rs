//! Placeable props: what they are, how they're saved, and how they're spawned with collision.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::gltf::GltfAssetLabel;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use serde::{Deserialize, Serialize};

/// Built-in shapes that need no asset files.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Primitive {
    Cube,
    Sphere,
    Cylinder,
    Capsule,
    Floor,
    Wall,
    Ramp,
    Stairs,
}

impl Primitive {
    pub const ALL: [Primitive; 8] = [
        Primitive::Cube,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Capsule,
        Primitive::Floor,
        Primitive::Wall,
        Primitive::Ramp,
        Primitive::Stairs,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Primitive::Cube => "Cube",
            Primitive::Sphere => "Sphere",
            Primitive::Cylinder => "Cylinder",
            Primitive::Capsule => "Capsule",
            Primitive::Floor => "Floor tile",
            Primitive::Wall => "Wall",
            Primitive::Ramp => "Ramp",
            Primitive::Stairs => "Stairs",
        }
    }

    pub fn default_color(self) -> [f32; 4] {
        match self {
            Primitive::Floor => [0.55, 0.55, 0.58, 1.0],
            Primitive::Wall => [0.78, 0.74, 0.68, 1.0],
            Primitive::Ramp | Primitive::Stairs => [0.62, 0.66, 0.72, 1.0],
            _ => [0.85, 0.85, 0.85, 1.0],
        }
    }

    fn stair_steps() -> impl Iterator<Item = (Vec3, Vec3)> {
        // Four solid steps, 2m wide, rising 1m over 2m towards -Z. (center, size)
        (0..4).map(|i| {
            let h = 0.25 * (i as f32 + 1.0);
            let center = Vec3::new(0.0, -0.5 + h / 2.0, 0.75 - 0.5 * i as f32);
            (center, Vec3::new(2.0, h, 0.5))
        })
    }

    fn ramp_points() -> [Vec3; 6] {
        [
            Vec3::new(-1.0, -0.5, 1.0),  // A
            Vec3::new(1.0, -0.5, 1.0),   // B
            Vec3::new(1.0, -0.5, -1.0),  // C
            Vec3::new(-1.0, -0.5, -1.0), // D
            Vec3::new(-1.0, 0.5, -1.0),  // E
            Vec3::new(1.0, 0.5, -1.0),   // F
        ]
    }
}

/// Where a prop comes from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PropSource {
    Primitive(Primitive),
    /// A scene asset path: a glTF/GLB file (`"props/crate.glb"`, first scene), a labelled scene
    /// (`"levels/city.glb#Scene2"`), or any format your game has a scene loader for.
    Model(String),
    /// Meshes the game loaded, by asset path (any format the game can load).
    Parts(Vec<MeshPart>),
    /// An object your game registered with [`crate::MapEditorAppExt::register_prefab`].
    Prefab(String),
}

/// One mesh of a [`PropSource::Parts`] prop.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MeshPart {
    /// Mesh asset path, e.g. `"levels/city.glb#Mesh12/Primitive0"`.
    pub mesh: String,
    /// `StandardMaterial` asset path, e.g. `"levels/city.glb#Material3/std"`. `None` uses the prop color.
    #[serde(default)]
    pub material: Option<String>,
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default = "identity_rotation")]
    pub rotation: [f32; 4],
    #[serde(default = "one_scale")]
    pub scale: [f32; 3],
}

fn identity_rotation() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn one_scale() -> [f32; 3] {
    [1.0; 3]
}

impl MeshPart {
    pub fn new(mesh: String, material: Option<String>, offset: Transform) -> Self {
        Self {
            mesh,
            material,
            translation: offset.translation.to_array(),
            rotation: offset.rotation.to_array(),
            scale: offset.scale.to_array(),
        }
    }

    fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_array(self.rotation).normalize(),
            scale: Vec3::from_array(self.scale),
        }
    }
}

/// File name without folders, extension or `#label`.
pub(crate) fn path_stem(path: &str) -> String {
    let file = path.split('#').next().unwrap_or(path);
    std::path::Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

pub(crate) fn is_gltf_path(path: &str) -> bool {
    let file = path.split('#').next().unwrap_or(path).to_ascii_lowercase();
    file.ends_with(".glb") || file.ends_with(".gltf")
}

impl PropSource {
    pub fn label(&self) -> String {
        match self {
            PropSource::Primitive(p) => p.label().to_string(),
            PropSource::Model(path) => path_stem(path),
            PropSource::Parts(parts) => parts.first().map(|p| path_stem(&p.mesh)).unwrap_or_default(),
            PropSource::Prefab(name) => name.clone(),
        }
    }
}

/// How a prop collides.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CollisionKind {
    /// No collision: decoration only.
    None,
    /// Exact shape. Primitives use their matching collider; models use a triangle mesh.
    #[default]
    Solid,
    /// Convex hull around the model. Cheaper than `Solid`, but fills in holes and doorways.
    ConvexHull,
}

impl CollisionKind {
    pub fn label(self) -> &'static str {
        match self {
            CollisionKind::None => "None",
            CollisionKind::Solid => "Solid (exact)",
            CollisionKind::ConvexHull => "Convex hull",
        }
    }
}

/// Marks an entity placed by the map editor. Everything with this component is saved in the map.
#[derive(Component, Clone, Debug)]
pub struct Prop {
    pub name: String,
    pub source: PropSource,
    pub color: [f32; 4],
    pub collision: CollisionKind,
}

/// One prop as stored in a map file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PropData {
    pub name: String,
    pub source: PropSource,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    #[serde(default = "white")]
    pub color: [f32; 4],
    #[serde(default)]
    pub collision: CollisionKind,
}

fn white() -> [f32; 4] {
    [1.0; 4]
}

impl PropData {
    pub fn new(source: PropSource) -> Self {
        let color = match &source {
            PropSource::Primitive(p) => p.default_color(),
            _ => white(),
        };
        Self {
            name: source.label(),
            source,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            color,
            collision: CollisionKind::Solid,
        }
    }

    pub fn from_prop(prop: &Prop, transform: &Transform) -> Self {
        Self {
            name: prop.name.clone(),
            source: prop.source.clone(),
            translation: transform.translation.to_array(),
            rotation: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
            color: prop.color,
            collision: prop.collision,
        }
    }

    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_array(self.rotation).normalize(),
            scale: Vec3::from_array(self.scale),
        }
    }

    pub fn set_transform(&mut self, t: &Transform) {
        self.translation = t.translation.to_array();
        self.rotation = t.rotation.to_array();
        self.scale = t.scale.to_array();
    }

    fn to_prop(&self) -> Prop {
        Prop {
            name: self.name.clone(),
            source: self.source.clone(),
            color: self.color,
            collision: self.collision,
        }
    }
}

/// A saved map (`assets/maps/<name>.map.json`).
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct MapFile {
    pub version: u32,
    pub props: Vec<PropData>,
}

/// Triggered on every prop the editor spawns (placed, duplicated, undone or loaded from a map).
///
/// If your game uses a physics engine other than avian3d, observe this to give props colliders:
/// `app.add_observer(|ev: On<PropSpawned>, props: Query<&Prop>, mut commands: Commands| { ... })`
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct PropSpawned {
    pub entity: Entity,
}

/// The preview that follows the cursor while placing. Not saved, no collision.
#[derive(Component)]
pub(crate) struct Ghost;

/// The material of a primitive prop, so its color can be edited live.
#[derive(Component)]
pub(crate) struct PropMaterial(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub(crate) struct PrimitiveMeshes {
    shapes: HashMap<Primitive, Handle<Mesh>>,
    steps: Vec<(Vec3, Handle<Mesh>)>,
}

impl FromWorld for PrimitiveMeshes {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mut shapes = HashMap::new();
        shapes.insert(Primitive::Cube, meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
        shapes.insert(Primitive::Sphere, meshes.add(Sphere::new(0.5).mesh().uv(32, 18)));
        shapes.insert(Primitive::Cylinder, meshes.add(Cylinder::new(0.5, 1.0)));
        shapes.insert(Primitive::Capsule, meshes.add(Capsule3d::new(0.5, 1.0)));
        shapes.insert(Primitive::Floor, meshes.add(Cuboid::new(4.0, 0.1, 4.0)));
        shapes.insert(Primitive::Wall, meshes.add(Cuboid::new(4.0, 3.0, 0.2)));
        shapes.insert(Primitive::Ramp, meshes.add(ramp_mesh()));
        let steps = Primitive::stair_steps()
            .map(|(center, size)| (center, meshes.add(Cuboid::from_size(size))))
            .collect();
        Self { shapes, steps }
    }
}

fn ramp_mesh() -> Mesh {
    let [a, b, c, d, e, f] = Primitive::ramp_points();
    let tris = [
        [a, d, c],
        [a, c, b], // bottom
        [d, e, f],
        [d, f, c], // back
        [a, b, f],
        [a, f, e], // slope
        [a, e, d], // left
        [b, c, f], // right
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for [p0, p1, p2] in tris {
        let n = (p1 - p0).cross(p2 - p0).normalize();
        for p in [p0, p1, p2] {
            positions.push(p.to_array());
            normals.push(n.to_array());
            uvs.push([p.x * 0.5 + 0.5, p.z * 0.5 + 0.5]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

/// Waiting for its meshes to load so colliders can be built from them.
#[cfg(feature = "avian")]
#[derive(Component)]
pub(crate) struct NeedsColliders {
    frames: u32,
}

/// Spawns props from [`PropData`].
#[derive(SystemParam)]
pub(crate) struct PropSpawner<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
    meshes: Res<'w, PrimitiveMeshes>,
    prefabs: Res<'w, crate::library::PrefabRegistry>,
}

impl PropSpawner<'_, '_> {
    pub fn spawn(&mut self, data: &PropData, ghost: bool) -> Entity {
        let prop = data.to_prop();
        let mut entity = self.commands.spawn((
            Name::new(format!("Prop: {}", data.name)),
            prop.clone(),
            data.transform(),
            Visibility::default(),
        ));

        match &data.source {
            PropSource::Primitive(primitive) => {
                let [r, g, b, a] = data.color;
                let material = self.materials.add(StandardMaterial {
                    base_color: Color::srgba(r, g, b, a),
                    perceptual_roughness: 0.85,
                    alpha_mode: if a < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                    ..default()
                });
                entity.insert(PropMaterial(material.clone()));
                if *primitive == Primitive::Stairs {
                    let steps = self.meshes.steps.clone();
                    entity.with_children(|parent| {
                        for (center, mesh) in steps {
                            parent.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(material.clone()),
                                Transform::from_translation(center),
                            ));
                        }
                    });
                } else {
                    entity.insert((
                        Mesh3d(self.meshes.shapes[primitive].clone()),
                        MeshMaterial3d(material),
                    ));
                }
            }
            PropSource::Model(path) => {
                let scene = if is_gltf_path(path) && !path.contains('#') {
                    self.asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.clone()))
                } else {
                    self.asset_server.load(path.clone())
                };
                entity.insert(WorldAssetRoot(scene));
            }
            PropSource::Parts(parts) => {
                let [r, g, b, a] = data.color;
                let fallback = self.materials.add(StandardMaterial {
                    base_color: Color::srgba(r, g, b, a),
                    perceptual_roughness: 0.85,
                    ..default()
                });
                let parts: Vec<_> = parts
                    .iter()
                    .map(|part| {
                        let material = match &part.material {
                            Some(path) => self.asset_server.load(path.clone()),
                            None => fallback.clone(),
                        };
                        (self.asset_server.load::<Mesh>(part.mesh.clone()), material, part.transform())
                    })
                    .collect();
                if parts.iter().any(|(_, m, _)| *m == fallback) {
                    entity.insert(PropMaterial(fallback));
                }
                entity.with_children(|parent| {
                    for (mesh, material, transform) in parts {
                        parent.spawn((Mesh3d(mesh), MeshMaterial3d(material), transform));
                    }
                });
            }
            PropSource::Prefab(name) => match self.prefabs.get(name) {
                Some(spawn) => spawn(&mut entity, &self.asset_server),
                None => warn!("map editor: no prefab named {name:?} is registered"),
            },
        }

        if ghost {
            entity.insert(Ghost);
        } else {
            add_collision(&mut entity, &prop);
        }

        let id = entity.id();
        if !ghost {
            self.commands.trigger(PropSpawned { entity: id });
        }
        id
    }
}

#[cfg(feature = "avian")]
fn add_collision(entity: &mut EntityCommands, prop: &Prop) {
    use avian3d::prelude::*;

    if prop.collision == CollisionKind::None {
        return;
    }
    match &prop.source {
        PropSource::Primitive(primitive) => {
            let collider = match primitive {
                Primitive::Cube => Collider::cuboid(1.0, 1.0, 1.0),
                Primitive::Sphere => Collider::sphere(0.5),
                Primitive::Cylinder => Collider::cylinder(0.5, 1.0),
                Primitive::Capsule => Collider::capsule(0.5, 1.0),
                Primitive::Floor => Collider::cuboid(4.0, 0.1, 4.0),
                Primitive::Wall => Collider::cuboid(4.0, 3.0, 0.2),
                Primitive::Ramp => Collider::convex_hull(Primitive::ramp_points().to_vec())
                    .unwrap_or_else(|| Collider::cuboid(2.0, 1.0, 2.0)),
                Primitive::Stairs => Collider::compound(
                    Primitive::stair_steps()
                        .map(|(center, size)| {
                            (center, Quat::IDENTITY, Collider::cuboid(size.x, size.y, size.z))
                        })
                        .collect(),
                ),
            };
            entity.insert((RigidBody::Static, collider));
        }
        // Colliders for every mesh under the prop, built once they have loaded.
        PropSource::Model(_) | PropSource::Parts(_) | PropSource::Prefab(_) => {
            entity.insert((RigidBody::Static, NeedsColliders { frames: 0 }));
        }
    }
}

#[cfg(feature = "avian")]
#[allow(clippy::type_complexity)]
fn build_pending_colliders(
    mut pending: Query<(Entity, &Prop, &mut NeedsColliders, Has<avian3d::prelude::Collider>)>,
    children: Query<&Children>,
    parts: Query<(&Mesh3d, Has<avian3d::prelude::Collider>)>,
    meshes: Res<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    use avian3d::prelude::*;
    use bevy::asset::LoadState;
    use bevy::mesh::Indices;

    for (root, prop, mut needs, root_has_collider) in &mut pending {
        needs.frames += 1;
        let entities: Vec<Entity> = std::iter::once(root)
            .chain(children.iter_descendants(root))
            .filter(|e| parts.contains(*e))
            .collect();
        // A prefab that brought its own collider is left alone; something that never loads is given up on.
        let give_up = root_has_collider || needs.frames > 1200;
        if !give_up {
            // Wait until the scene has spawned and every mesh is in memory (or failed to load).
            let waiting = entities.is_empty()
                || entities.iter().any(|e| {
                    let (mesh, _) = parts.get(*e).unwrap();
                    meshes.get(&mesh.0).is_none()
                        && !matches!(asset_server.load_state(&mesh.0), LoadState::Failed(_) | LoadState::Loaded)
                });
            if waiting {
                continue;
            }
            for entity in entities {
                let (mesh, has_collider) = parts.get(entity).unwrap();
                let Some(mesh) = meshes.get(&mesh.0) else {
                    continue;
                };
                if has_collider {
                    continue;
                }
                // Triangle-mesh colliders need an index buffer; plenty of exported meshes have none.
                let indexed;
                let mesh = if mesh.indices().is_none() {
                    let mut copy = mesh.clone();
                    copy.insert_indices(Indices::U32((0..mesh.count_vertices() as u32).collect()));
                    indexed = copy;
                    &indexed
                } else {
                    mesh
                };
                let collider = match prop.collision {
                    CollisionKind::ConvexHull => Collider::convex_hull_from_mesh(mesh),
                    _ => Collider::trimesh_from_mesh(mesh),
                };
                match collider {
                    Some(collider) => {
                        commands.entity(entity).insert(collider);
                    }
                    None => warn!("map editor: could not build a collider for a mesh in {}", prop.name),
                }
            }
        }
        commands.entity(root).remove::<NeedsColliders>();
    }
}

#[cfg(not(feature = "avian"))]
fn add_collision(_entity: &mut EntityCommands, _prop: &Prop) {
    // No built-in physics backend: observe `PropSpawned` to add colliders for your engine.
}

pub(crate) struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrimitiveMeshes>();
        #[cfg(feature = "avian")]
        app.add_systems(Update, build_pending_colliders);
    }
}
