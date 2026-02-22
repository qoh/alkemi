use avian3d::prelude::*;
use bevy::{asset::AsAssetId, prelude::*};
use typed_path::PlatformPath;

use crate::{
    magicka_assets::{item::Item as ItemAsset, skinned_model::Bone},
    magicka_level_model::{Layers, animated_parts::spawn_xna_model},
};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, spawn_new_item_models);
    app.add_systems(Update, reenable_physics_for_detached_item);
    app.add_systems(
        PostUpdate,
        attach_items_to_skeleton.before(TransformSystems::Propagate),
    );
}

#[derive(Component, Debug, Reflect)]
#[require(
    RigidBody::Dynamic,
    Collider::cuboid(1., 1., 1.),
    Restitution::new(0.),
    Friction::new(1.),
    CollisionLayers::from_bits(Layers::Default.to_bits(), !Layers::Trigger.to_bits()),

    Visibility, // should be added by vfx plugin?
)]
pub struct ItemInstance(pub Handle<ItemAsset>);

impl AsAssetId for ItemInstance {
    type Asset = ItemAsset;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

#[derive(Component, Default, Debug, Reflect)]
#[require(RigidBodyDisabled, ColliderDisabled)]
pub struct AttachedItem;

/// Waiting for the skeleton to load
#[derive(Component, Debug, Reflect)]
#[require(AttachedItem, Visibility::Hidden)]
pub struct DeferredAttachedItem {
    pub bone_name: String, // not nice that this is a (cloned) string
    pub skeleton: Entity,
}

fn reenable_physics_for_detached_item(
    mut newly_detached_entities: RemovedComponents<AttachedItem>,
    items: Query<(), With<ItemInstance>>,
    mut commands: Commands,
) {
    for entity in newly_detached_entities.read() {
        if items.contains(entity) {
            commands
                .entity(entity)
                .try_remove::<(RigidBodyDisabled, ColliderDisabled)>();
        }
    }
}

type AssetHandleChanged<C> = Or<(Changed<C>, AssetChanged<C>)>;

fn spawn_new_item_models(
    changed_items: Query<(Entity, &ItemInstance), AssetHandleChanged<ItemInstance>>,
    item_assets: Res<Assets<ItemAsset>>,

    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    for (item, asset_handle) in changed_items {
        let asset = item_assets.get(asset_handle.as_asset_id());

        // TODO: Despawn previous model

        let Some(asset) = asset else {
            continue;
        };
        let Some(from_content_path) =
            crate::magicka_assets::content_path_from_handle(&asset_handle.0)
        else {
            warn!(
                "No content path to resolve item {:?} model (from {:?})",
                asset.item.name,
                asset_handle.0.path()
            );
            continue;
        };
        let spawn_result = load_and_spawn_xna_model(
            &asset.item.model.path,
            from_content_path,
            commands.reborrow(),
            meshes.reborrow(),
            materials.reborrow(),
            &assets,
        );
        let model_entity = match spawn_result {
            Ok(e) => e,
            Err(()) => {
                warn!(
                    "Failed to spawn item {:?} model {:?}",
                    asset.item.name, asset.item.model.path
                );
                continue;
            }
        };
        commands.entity(model_entity).insert((
            ChildOf(item),
            Transform::from_scale(Vec3::splat(asset.item.scale)),
            if asset.item.hide_model {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
        ));
    }
}

fn load_and_spawn_xna_model(
    relative_path: &str,
    content_path: &PlatformPath,
    commands: Commands,
    mut meshes: Mut<Assets<Mesh>>,
    mut materials: Mut<Assets<StandardMaterial>>,
    assets: &AssetServer,
) -> Result<Entity, ()> {
    let resolved_path = crate::magicka_assets::resolve_relative_path(content_path, relative_path);
    let file_path =
        std::path::Path::new(resolved_path.resolved_path.as_ref() as &std::ffi::OsStr).to_owned();
    let bytes = std::fs::read(&file_path).map_err(|e| {
        warn!("Failed to read model file {file_path:?}: {e}");
    })?;
    let xnb_asset = remagic::parse_model(&bytes).map_err(|e| {
        warn!(
            "Failed to parse model file {file_path:?}: {}",
            e.into_inner()
        );
    })?;
    let xna_model = xnb_asset.inner().as_ref().ok_or_else(|| {
        warn!("Model is null in file {file_path:?}");
    })?;
    let model_entity = spawn_xna_model(
        xna_model,
        &xnb_asset,
        &resolved_path.transitive_content_path,
        commands,
        &mut meshes,
        &mut materials,
        assets,
    );
    Ok(model_entity)
}

fn attach_items_to_skeleton(
    waiting_items: Query<(Entity, &DeferredAttachedItem)>,
    skeletons: Query<Option<&bevy::scene::SceneInstance>>,
    bones: Query<(Entity, &Name), With<Bone>>,
    children: Query<&Children>,
    scene_spawner: Res<SceneSpawner>,
    mut commands: Commands,
) {
    for (item, defer_attach) in waiting_items {
        let Ok(skeleton_scene) = skeletons.get(defer_attach.skeleton) else {
            warn!(
                "Deferred attached item {item:?} refers to un-spawned skeleton entity {:?}",
                defer_attach.skeleton
            );
            commands.entity(item).try_remove::<DeferredAttachedItem>();
            continue;
        };

        // If the target is an instanced scene, wait for it to become ready
        if let Some(instance) = skeleton_scene
            && !scene_spawner.instance_is_ready(**instance)
        {
            continue;
        }

        if skeleton_scene.is_none() {
            info_once!(
                "Deferred item attach {item:?} for bone {:?} target {:?} has no SceneInstance",
                defer_attach.bone_name,
                defer_attach.skeleton
            );
        }

        // Find a child bone with a matching name
        let bone = children
            .iter_descendants(defer_attach.skeleton)
            .find_map(|child| {
                let (bone, bone_name) = bones.get(child).ok()?;
                let matches = bone_name.eq_ignore_ascii_case(&defer_attach.bone_name);
                matches.then_some(bone)
            });

        if let Some(bone) = bone {
            // Unconditionally visible because the skeleton is usually invisible
            commands
                .entity(item)
                .insert((ChildOf(bone), Visibility::Visible));
        } else {
            warn!(
                "Can't find bone {:?} on skeleton {:?} to attach item {item:?} to",
                defer_attach.bone_name, defer_attach.skeleton,
            );
        }

        commands.entity(item).try_remove::<DeferredAttachedItem>();
    }
}
