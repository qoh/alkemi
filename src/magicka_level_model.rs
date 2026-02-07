pub(crate) mod animated_parts;
mod bitree;
mod collision;
mod d3dx;
mod effect;
pub(crate) mod light;
mod liquid;
mod nav_mesh;
mod visual_effects;
mod xna_geom;

// TODO: Move the definition of Layers out of magicka_level_model
pub use collision::Layers;

use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};
use std::collections::HashMap;
use typed_path::PlatformPath;

use crate::magicka_assets::content_root;

pub fn plugin(app: &mut App) {
    app.add_plugins((nav_mesh::plugin, light::plugin));
    app.add_plugins(MaterialPlugin::<effect::CharacterMaterial>::default());
    app.register_asset_reflect::<effect::CharacterMaterial>();

    app.add_systems(
        PostUpdate,
        (
            animated_parts::debug_animated_parts,
            visual_effects::debug_visual_effects,
            visual_effects::assign_visual_effect_scene,
            debug_trigger_areas,
            debug_locators,
        ),
    );
    app.add_observer(trigger_area_enter);
    app.add_observer(trigger_area_leave);
}

pub fn spawn_level(
    InRef(level_path): InRef<PlatformPath>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    assets: Res<AssetServer>,
    mut nav_mesh_params: nav_mesh::SpawnParams,
) -> Result<Entity> {
    // let content_root = NativePathBuf::from(crate::magicka_assets::content_root());
    let content_path = level_path;
    let level_path = content_root().join_checked(content_path).unwrap();
    let level_bytes = std::fs::read(level_path.as_ref() as &std::ffi::OsStr)?;
    let level_asset = remagic::parse_level(&level_bytes).map_err(|e| {
        panic!("parsing failed: {}", e.inner());
    })?;
    let level_data = level_asset
        .inner()
        .as_ref()
        .ok_or_else(|| todo!("null level"))?;

    let mut root = commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
    ));

    root.with_children(|parent| {
        let nav_mesh_setup = nav_mesh::setup_for_level(parent);

        if let Some(model) = &level_data.model {
            let mut parent = parent.spawn((
                Name::new("BiTreeModel"),
                Transform::default(),
                Visibility::default(),
            ));
            parent.with_children(|parent| {
                bitree::spawn_bitree_model(
                    meshes.as_mut(),
                    materials.as_mut(),
                    &assets,
                    parent,
                    model,
                    content_path,
                );
            });
        }

        // This map lets animated parts find the lights to take ownership of
        let mut light_entities = HashMap::new();
        for light in &level_data.lights {
            let entity = light::spawn_light(parent, light);
            let existing = light_entities.insert(&light.name, entity);
            if existing.is_some() {
                warn!("Level has multiple lights named {:?}", light.name);
            }
        }

        for animated_part in &level_data.animated_parts {
            animated_parts::spawn_animated_part(
                parent,
                animated_part,
                &level_asset,
                content_path,
                &mut meshes,
                &mut materials,
                animation_clips.reborrow(),
                animation_graphs.reborrow(),
                &assets,
                &mut light_entities,
                &mut nav_mesh_params,
                &nav_mesh_setup,
            );
        }

        for visual_effect in &level_data.visual_effects {
            visual_effects::spawn_visual_effect(Spawner::Parent(parent), visual_effect, &assets);
        }

        for physics_entity in &level_data.physics_entities {
            warn!("unhandled level {physics_entity:#?}");
        }
        for liquid in &level_data.waters {
            liquid::spawn_liquid(
                Spawner::Parent(parent),
                liquid,
                content_path,
                &mut meshes,
                &mut materials,
                &assets,
            );
        }
        for force_field in &level_data.force_fields {
            warn!("unhandled level {force_field:#?}");
        }

        // Collision
        parent
            .spawn((
                Name::new("Level Collision"),
                avian3d::prelude::RigidBody::Static,
            ))
            .with_children(|parent| {
                for layer in &level_data.collision {
                    if let Some(collider) = collision::to_collider(layer) {
                        parent.spawn((
                            collider,
                            avian3d::prelude::CollisionLayers::new(
                                collision::Layers::Level,
                                avian3d::prelude::LayerMask::ALL,
                            ),
                        ));
                    }
                }
            });

        if let Some(camera_mesh) = &level_data.camera_mesh {
            let collider = collision::to_collider(camera_mesh);
            if let Some(collider) = collider {
                // Just used to store the Collider for direct raycasting.
                parent.spawn((Name::new("Camera Mesh"), CameraMesh { collider }));
            }
        }

        nav_mesh::spawn(
            parent,
            &level_data.nav_mesh,
            &mut nav_mesh_params,
            &nav_mesh_setup,
        );

        // Spawn in areas used for trigger checks
        // Also spawn an extra global area that triggers use to check if entities exist anywhere
        parent.spawn((Name::new("any"), TriggerArea::default(), TriggerAreaGlobal));
        for (name, trigger_area) in &level_data.trigger_areas {
            let position = map_vec3(trigger_area.position);
            let side_lengths = map_vec3(trigger_area.side_lengths);
            let orientation = map_quat(trigger_area.orientation);
            let center = (Transform::from_rotation(orientation) * (side_lengths * 0.5)) + position;
            use avian3d::prelude::PhysicsLayer;
            parent.spawn((
                Name::new(name.to_owned()),
                TriggerArea::default(),
                avian3d::prelude::CollisionEventsEnabled,
                Transform {
                    translation: center,
                    rotation: orientation,
                    scale: side_lengths,
                },
                avian3d::prelude::Collider::cuboid(1., 1., 1.),
                avian3d::prelude::CollisionLayers::new(
                    collision::Layers::Trigger,
                    avian3d::prelude::LayerMask::ALL
                        & !avian3d::prelude::LayerMask::from(
                            collision::Layers::Level.to_bits()
                                | collision::Layers::Trigger.to_bits(),
                        ),
                ),
                avian3d::prelude::Sensor,
            ));
        }
        for (name, locator) in &level_data.locators {
            spawn_locator(Spawner::Parent(parent), name.to_owned(), locator);
        }
    });

    let instance_id = root.id();

    Ok(instance_id)
}

enum Spawner<'w, /*'s,*/ 'r> {
    // Root(Commands<'w, 's>),
    Parent(&'r mut RelatedSpawnerCommands<'w, ChildOf>),
}

fn debug_trigger_areas(q: Query<&GlobalTransform, With<TriggerArea>>, mut gizmos: Gizmos) {
    for transform in q {
        // gizmos.cuboid(*transform, Color::srgb(0., 1., 0.));
    }
}

fn debug_locators(q: Query<(&GlobalTransform, &Locator)>, mut gizmos: Gizmos) {
    for (transform, locator) in q {
        gizmos.axes(*transform, 0.5);
        // Almost everything has 2., so if it differs that's interesting
        if locator.radius != 2. {
            gizmos.sphere(
                transform.to_isometry(),
                locator.radius,
                Color::srgb(0., 0., 1.),
            );
        }
    }
}

/// The [`avian3d::prelude::Collider`] corresponding to the level's camera mesh.
#[derive(Component, Debug, Default)]
pub struct CameraMesh {
    pub collider: avian3d::prelude::Collider,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct TriggerArea {
    // Values are the character type
    characters: std::collections::HashMap<Entity, String>,
}

impl TriggerArea {
    pub fn num_characters(&self) -> usize {
        self.characters.len()
    }
    pub fn num_characters_of_type(&self, type_name: &str) -> usize {
        self.characters
            .iter()
            .filter(|(_, v)| v.eq_ignore_ascii_case(type_name))
            .count()
    }
}

/// This trigger area counts entities/characters globally instead of in a physical region.
#[derive(Component, Debug, Default)]
pub struct TriggerAreaGlobal; // TODO: Implement

pub fn trigger_area_enter(
    event: On<avian3d::prelude::CollisionStart>,
    mut areas: Query<&mut TriggerArea>,
    characters: Query<&crate::character::Character, Without<TriggerArea>>,
) -> Result {
    let entity1 = event.body1.unwrap_or(event.collider1);
    let entity2 = event.body2.unwrap_or(event.collider2);
    let (area_entity, character_entity) = if areas.contains(entity1) {
        (entity1, entity2)
    } else if areas.contains(entity2) {
        (entity2, entity1)
    } else {
        return Ok(());
    };
    let mut area = areas.get_mut(area_entity)?;
    if !characters.contains(character_entity) {
        return Ok(());
    }

    let character = characters.get(character_entity)?;
    let character_type = character.type_name.clone();
    area.characters.insert(character_entity, character_type);

    Ok(())
}
pub fn trigger_area_leave(
    event: On<avian3d::prelude::CollisionEnd>,
    mut areas: Query<&mut TriggerArea>,
) -> Result {
    let entity1 = event.body1.unwrap_or(event.collider1);
    let entity2 = event.body2.unwrap_or(event.collider2);
    let (area_entity, character_entity) = if areas.contains(entity1) {
        (entity1, entity2)
    } else if areas.contains(entity2) {
        (entity2, entity1)
    } else {
        return Ok(());
    };
    let mut area = areas.get_mut(area_entity)?;

    if area.characters.contains_key(&character_entity) {
        area.characters.remove(&character_entity);
    }

    Ok(())
}

#[derive(Component, Reflect)]
pub struct Locator {
    pub radius: f32,
}
fn spawn_locator(
    Spawner::Parent(parent): Spawner,
    name: String,
    locator: &remagic::xnb_readers::magicka_content::Locator,
) -> Entity {
    let matrix = map_mat4(locator.transform);
    // TODO: safe check, must be affine
    let transform = Transform::from_matrix(matrix);
    parent
        .spawn((
            Name::new(name.to_owned()),
            Locator {
                radius: locator.radius,
            },
            transform,
        ))
        .id()
}

fn map_vec3(magicka: remagic::xnb::types::Vector3) -> Vec3 {
    vec3(magicka.0, magicka.1, magicka.2)
}

fn map_quat(magicka: remagic::xnb::types::Quaternion) -> Quat {
    quat(magicka.0, magicka.1, magicka.2, magicka.3)
}

pub(crate) fn map_mat4(magicka: remagic::xnb::types::Matrix) -> Mat4 {
    Mat4 {
        x_axis: vec4(magicka.0, magicka.1, magicka.2, magicka.3),
        y_axis: vec4(magicka.4, magicka.5, magicka.6, magicka.7),
        z_axis: vec4(magicka.8, magicka.9, magicka.10, magicka.11),
        w_axis: vec4(magicka.12, magicka.13, magicka.14, magicka.15),
    }
}
