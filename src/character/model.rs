use bevy::prelude::*;
use remagic::xnb_readers::magicka_character::CharacterTemplate;
use std::ffi::OsStr;
use typed_path::{PlatformPath, PlatformPathBuf};

use crate::magicka_assets::skinned_model::Bone;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ModelCache>();
    app.add_systems(PostUpdate, copy_skinnedmesh_from_source);
    app.register_type::<Bone>();
}

// HACK: Workaround for AssetServer not permitting insert of loaded assets with path
#[derive(Resource, Default, Debug)]
pub struct ModelCache {
    by_path_in_content: std::collections::HashMap<PlatformPathBuf, Handle<Scene>>,
}

#[derive(Debug)]
pub(super) struct AttachedModel {
    pub skeleton: Entity,
}

pub(super) fn attach_model(
    content_path: &PlatformPath,
    template: &CharacterTemplate,
    model_index: usize,
    mut player: EntityCommands,
    relative_transform: Transform,
    mut cache: Mut<ModelCache>,
    mut meshes: Mut<Assets<Mesh>>,
    mut materials: Mut<Assets<StandardMaterial>>,
    assets: &AssetServer,
) -> AttachedModel {
    let player_entity = player.id();

    let CharacterTemplate {
        length,
        radius,
        skinned_models,
        ..
    } = template;

    let full_height = length + radius * 2.;

    // Attach invisible shared "skeleton" model that actually plays all the animations
    let skeleton_scene = get_or_load_model(
        content_path,
        skinned_models.1.path.as_str(),
        cache.reborrow(),
        meshes.reborrow(),
        materials.reborrow(),
        assets,
    );
    let skeleton_ent = player
        .commands_mut()
        .spawn((
            ChildOf(player_entity),
            Transform::from_translation(Vec3::Y * -0.5 * full_height) * relative_transform,
            SceneRoot(skeleton_scene),
            Visibility::Hidden,
        ))
        .id();

    // Attach visible model that reflects the animation of the invisible one
    let visual_scene = get_or_load_model(
        content_path,
        skinned_models.0[model_index].0.path.as_str(),
        cache.reborrow(),
        meshes.reborrow(),
        materials.reborrow(),
        assets,
    );
    player.with_child((SceneRoot(visual_scene), CopiesSkinnedMeshFrom(skeleton_ent)));

    AttachedModel {
        skeleton: skeleton_ent,
    }
}

fn get_or_load_model(
    content_path: &PlatformPath,
    relative_path: &str,
    mut cache: Mut<ModelCache>,
    mut meshes: Mut<Assets<Mesh>>,
    mut materials: Mut<Assets<StandardMaterial>>,
    assets: &AssetServer,
) -> Handle<Scene> {
    let relative_path = typed_path::WindowsPathBuf::from(relative_path);
    let model_content_path = content_path.parent().unwrap().join(
        relative_path
            .with_platform_encoding_checked()
            .unwrap()
            .as_bytes(),
    );
    if let Some(handle) = cache.by_path_in_content.get(model_content_path.as_path()) {
        return (handle as &Handle<Scene>).clone();
    }

    let handle = load_model(
        &mut meshes,
        &mut materials,
        assets,
        model_content_path.as_path(),
    );
    cache
        .by_path_in_content
        .insert(model_content_path, handle.clone());
    handle
}

fn load_model(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    model_content_path: &PlatformPath,
) -> Handle<Scene> {
    debug!(
        "Loading character model {:?}",
        model_content_path.to_string_lossy()
    );

    let mut model_path = crate::magicka_assets::content_root()
        .join_checked(model_content_path)
        .unwrap();
    model_path.set_extension("xnb");

    let model_path = std::path::Path::new(model_path.as_ref() as &OsStr);
    let bytes = crate::magicka_assets::read_ignore_path_ascii_case(model_path).unwrap();
    let xnb_asset = remagic::parse_skinned_model(&bytes)
        .map_err(|e| e.into_inner())
        .unwrap();
    let skinned_mesh = xnb_asset.inner().as_ref().unwrap();

    assets.add(crate::magicka_assets::skinned_model::load_skinned_model(
        skinned_mesh,
        &xnb_asset,
        model_content_path,
        meshes,
        materials,
        assets,
    ))
}

/// All descendant SkinnedMesh components will be replaced once with
/// the first descendant SkinnedMesh component on the target entity.
#[derive(Component, Debug)]
#[relationship(relationship_target = CopiesSkinnedMeshTo)]
struct CopiesSkinnedMeshFrom(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = CopiesSkinnedMeshFrom)]
struct CopiesSkinnedMeshTo(Vec<Entity>);

fn copy_skinnedmesh_from_source(
    sources: Query<Option<&bevy::scene::SceneInstance>, With<CopiesSkinnedMeshTo>>,
    targets: Query<(
        Entity,
        &CopiesSkinnedMeshFrom,
        Option<&bevy::scene::SceneInstance>,
    )>,
    mut skinned_meshes: Query<&mut bevy::mesh::skinning::SkinnedMesh>,
    children: Query<&Children>,
    scene_spawner: Res<SceneSpawner>,
    mut commands: Commands,
) {
    for (target, target_source, target_scene) in targets {
        // If the target is an instanced scene, wait for it to become ready
        if let Some(instance) = target_scene
            && !scene_spawner.instance_is_ready(**instance)
        {
            continue;
        }

        let source = target_source.0;
        let source_scene = sources.get(source).unwrap();
        // If the source is an instanced scene, wait for it to become ready
        if let Some(instance) = source_scene
            && !scene_spawner.instance_is_ready(**instance)
        {
            continue;
        }

        // Grab the first available SkinnedMesh in tree order
        let mut source_skinned_mesh = None;
        for child in children.iter_descendants(source) {
            if let Ok(skinned_mesh) = skinned_meshes.get(child) {
                source_skinned_mesh = Some(skinned_mesh.clone());
                break;
            }
        }

        // If there was none, sad
        let Some(source_skinned_mesh) = source_skinned_mesh else {
            warn_once!("No skinned mesh config to copy from source");
            // Give up
            commands.entity(target).remove::<CopiesSkinnedMeshFrom>();
            continue;
        };

        // Replace all target SkinnedMeshes with it
        for child in children.iter_descendants(target) {
            if let Ok(mut skinned_mesh) = skinned_meshes.get_mut(child) {
                *skinned_mesh = source_skinned_mesh.clone();
            }
        }

        // Success, don't check anymore
        commands.entity(target).remove::<CopiesSkinnedMeshFrom>();
    }
}
