use std::collections::HashMap;

use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    prelude::*,
};
use remagic::SharedResources;
use typed_path::PlatformPath;

use crate::magicka_level_model::map_mat4;

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct AnimationLibrary {
    pub animations: HashMap<String, AnimationNodeIndex>,
}

pub fn load_skinned_model(
    model: &remagic::xnb_readers::skinning::SkinnedModel,
    shared_resources: &impl SharedResources,
    content_path: &PlatformPath,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) -> Scene {
    let mut world = World::default();

    // Load all the animaitons
    let mut anim_clips = HashMap::with_capacity(model.animations.len());
    let mut anim_graph = AnimationGraph::new();
    let blend_node = anim_graph.add_blend(1., anim_graph.root);
    for source_clip in &model.animations {
        let source_clip = shared_resources
            .shared_resource(source_clip.as_ref().unwrap())
            .unwrap()
            .unwrap();
        let mut clip = AnimationClip::default();
        for (target_name, channel) in &source_clip.channels {
            let target_id = AnimationTargetId::from_name(&Name::new(target_name.clone()));
            crate::magicka_level_model::animated_parts::add_channel_to_clip(
                &mut clip, channel, target_id,
            );
        }
        clip.set_duration(source_clip.duration);
        let clip = assets.add(clip);
        let clip_node = anim_graph.add_clip(clip, 1., blend_node);
        anim_clips.insert(source_clip.name.clone(), clip_node);
    }

    let root = world
        .spawn((
            Transform::default(),
            Visibility::default(),
            AnimationPlayer::default(),
            AnimationGraphHandle(assets.add(anim_graph)),
            AnimationLibrary {
                animations: anim_clips,
            },
        ))
        .id();

    // Bones, first pass: Create them without a parent
    // Also separately populate these vecs for the SkinnedMesh
    let mut bone_inverse_bind_poses = Vec::with_capacity(model.bones.len());
    let mut bone_joint_entities = Vec::with_capacity(model.bones.len());
    let bone_entities: Vec<_> = model
        .bones
        .iter()
        .map(|bone_ref| {
            let bone = shared_resources
                .shared_resource(bone_ref.as_ref().unwrap())
                .unwrap()
                .unwrap();
            let name = Name::new(bone.name.clone());
            let anim_target_id = AnimationTargetId::from_name(&name);
            let entity = world
                .spawn((
                    name,
                    Transform::default(),
                    Visibility::default(),
                    anim_target_id,
                    AnimatedBy(root),
                ))
                .id();
            bone_inverse_bind_poses.push(map_mat4(bone.inverse_bind_pose_transform));
            bone_joint_entities.push(entity);
            (bone_ref, bone, entity)
        })
        .collect();

    // Bones, second pass: Parent entities to parent bones (or the root)
    for (_bone_ref, bone_data, bone_entity) in &bone_entities {
        let parent_entity = if let Some(parent_ref) = &bone_data.parent_bone {
            let (_, _, parent_bone_entity) = bone_entities
                .iter()
                .find(|(check_ref, _, _)| {
                    check_ref
                        .as_ref()
                        .is_some_and(|check_ref| check_ref == parent_ref)
                })
                .unwrap();
            *parent_bone_entity
        } else {
            root
        };
        world.entity_mut(parent_entity).add_child(*bone_entity);
    }

    // Finally load in the meshes, with skinning set up targeting the bones
    if let Some(xna_model) = &model.model {
        let inverse_bindposes =
            assets.add(SkinnedMeshInverseBindposes::from(bone_inverse_bind_poses));
        let skinned_mesh = SkinnedMesh {
            inverse_bindposes,
            joints: bone_joint_entities,
        };
        let model_entity = crate::magicka_level_model::animated_parts::spawn_xna_model_detailed(
            xna_model,
            shared_resources,
            content_path,
            world.commands(),
            meshes,
            materials,
            assets,
            Some(&skinned_mesh),
        );
        world.flush();
        world.entity_mut(root).add_child(model_entity);
    }

    Scene::new(world)
}
