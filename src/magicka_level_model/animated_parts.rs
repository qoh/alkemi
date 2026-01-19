use crate::magicka_level_model::{Spawner, spawn_locator, visual_effects::spawn_visual_effect};

use super::{d3dx, map_mat4, map_quat, map_vec3, xna_geom};
use bevy::{
    animation::{AnimationTarget, AnimationTargetId, animated_field},
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    render::render_resource::Face,
};
use remagic::xnb_readers::magicka_content::{AnimatedLevelPart, AnimationChannel};
use std::collections::HashMap;
use typed_path::PlatformPath;

pub(crate) fn debug_animated_parts(
    parts: Query<
        (
            &GlobalTransform,
            Has<Mesh3d>,
            Has<MeshMaterial3d<StandardMaterial>>,
        ),
        With<AnimatedPart>,
    >,
    mut gizmos: Gizmos,
) {
    for (transform, has_mesh, has_material) in parts {
        if !has_mesh || !has_material {
            gizmos.axes(*transform, 1.);
        }
    }
}

#[derive(Component)]
pub struct AnimatedPart {}

pub(crate) fn spawn_animated_part(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    animated_part: &AnimatedLevelPart,
    shared_resources: &impl remagic::SharedResources,
    content_path: &PlatformPath,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    mut animation_clips: Mut<Assets<AnimationClip>>,
    mut animation_graphs: Mut<Assets<AnimationGraph>>,
    assets: &AssetServer,
    light_entities: &mut HashMap<&String, Entity>,
    nav_mesh_params: &mut crate::magicka_level_model::nav_mesh::SpawnParams,
    nav_mesh_setup: &crate::magicka_level_model::nav_mesh::NavMeshSetup,
) {
    let transform = if let Some((_, pose1)) = animated_part.animation.keyframes.first() {
        Transform {
            translation: map_vec3(pose1.translation),
            rotation: map_quat(pose1.orientation),
            scale: map_vec3(pose1.scale),
        }
    } else {
        Transform::default()
    };

    debug!(
        "unhandled animated level part {:?} affect_shields={:?}",
        &animated_part.name, animated_part.affect_shields
    );
    for (mesh_name, (visible, cast_shadows)) in &animated_part.mesh_settings {
        if !visible {
            warn!(
                "unhandled animated level part {:?} setting for mesh {mesh_name:?}: visible=false",
                &animated_part.name
            );
        }
        if !cast_shadows {
            warn!(
                "unhandled animated level part {:?} setting for mesh {mesh_name:?}: cast_shadows=false",
                &animated_part.name
            );
        }
    }
    let mut entity_commands = parent.spawn((
        Name::new(animated_part.name.to_owned()),
        transform,
        Visibility::default(),
        AnimatedPart {},
    ));

    if let Some(remagic::xnb_readers::magicka_content::AnimatedLevelPartCollision {
        material,
        vertices: Some(vertices),
        triangle_vertex_indices: indices,
    }) = &animated_part.collision
    {
        use avian3d::prelude::*;
        entity_commands.insert((
            RigidBody::Kinematic,
            super::collision::to_collider_raw(vertices.0.as_slice(), indices),
            CollisionLayers::new(super::collision::Layers::Level, LayerMask::ALL),
        ));
    }

    for (light_name, matrix) in &animated_part.lights {
        let Some(light_entity) = light_entities.remove(light_name) else {
            warn!(
                "Animated level part {:?} could not find light {light_name:?}, or multiple parts are referring to it.",
                animated_part.name
            );
            continue;
        };
        let matrix = map_mat4(*matrix);
        // TODO: safe check, must be affine
        let transform = Transform::from_matrix(matrix);
        entity_commands.add_child(light_entity);
        entity_commands
            .commands_mut()
            .entity(light_entity)
            .insert(transform);
    }

    entity_commands.with_children(|parent| {
        for liquid in &animated_part.liquids {
            super::liquid::spawn_liquid(
                Spawner::Parent(parent),
                liquid,
                content_path,
                meshes,
                materials,
                assets,
            );
        }
        for visual_effect in &animated_part.effects {
            spawn_visual_effect(Spawner::Parent(parent), visual_effect, assets);
        }
        for (name, locator) in &animated_part.locators {
            spawn_locator(Spawner::Parent(parent), name.to_owned(), locator);
        }

        if let Some(nav_mesh) = &animated_part.nav_mesh {
            super::nav_mesh::spawn(parent, nav_mesh, nav_mesh_params, nav_mesh_setup);
        }

        for child in &animated_part.children {
            spawn_animated_part(
                parent,
                child,
                shared_resources,
                content_path,
                meshes,
                materials,
                animation_clips.reborrow(),
                animation_graphs.reborrow(),
                assets,
                light_entities,
                nav_mesh_params,
                nav_mesh_setup,
            );
        }
    });

    setup_animation(
        animated_part,
        entity_commands.reborrow(),
        animation_clips,
        animation_graphs,
    );

    let entity = entity_commands.id();
    // TODO move this above children?
    if let Some(model) = animated_part.model.as_ref() {
        let model_entity = spawn_xna_model(
            model,
            shared_resources,
            content_path,
            parent.commands(),
            meshes,
            materials,
            assets,
        );
        parent.commands().entity(entity).add_child(model_entity);
    }
}

fn setup_animation(
    part: &AnimatedLevelPart,
    mut part_commands: EntityCommands<'_>,
    mut animation_clips: Mut<Assets<AnimationClip>>,
    mut animation_graphs: Mut<Assets<AnimationGraph>>,
) {
    let target_id = AnimationTargetId::from_name(&Name::new("root"));
    let mut clip = AnimationClip::default();
    add_channel_to_clip(&mut clip, &part.animation, target_id);
    clip.set_duration(part.animation_duration);
    let clip = animation_clips.add(clip);
    let (graph, node_index) = AnimationGraph::from_clip(clip);
    let graph = animation_graphs.add(graph);
    let mut player = AnimationPlayer::default();
    player.play(node_index).repeat();
    part_commands.insert((
        player,
        AnimationTarget {
            id: target_id,
            player: part_commands.id(),
        },
        AnimationGraphHandle(graph),
    ));
}

// TODO: Move this to magicka_assets
pub(crate) fn add_channel_to_clip(
    clip: &mut AnimationClip,
    channel: &AnimationChannel,
    target_id: AnimationTargetId,
) {
    let translation_keyframes = channel
        .keyframes
        .iter()
        .map(|(t, p)| (*t, map_vec3(p.translation)));
    if let Ok(curve) = AnimatableKeyframeCurve::new(translation_keyframes) {
        clip.add_curve_to_target(
            target_id,
            AnimatableCurve::new(animated_field!(Transform::translation), curve),
        );
    }

    let rotation_keyframes = channel
        .keyframes
        .iter()
        .map(|(t, p)| (*t, map_quat(p.orientation)));
    if let Ok(curve) = AnimatableKeyframeCurve::new(rotation_keyframes) {
        clip.add_curve_to_target(
            target_id,
            AnimatableCurve::new(animated_field!(Transform::rotation), curve),
        );
    }

    let scale_keyframes = channel
        .keyframes
        .iter()
        .map(|(t, p)| (*t, map_vec3(p.scale)));
    if let Ok(curve) = AnimatableKeyframeCurve::new(scale_keyframes) {
        clip.add_curve_to_target(
            target_id,
            AnimatableCurve::new(animated_field!(Transform::scale), curve),
        );
    }
}

// TODO: move to more appropriate module
pub(crate) fn spawn_xna_model(
    xna_model: &remagic::xnb_readers::xna_mesh::Model,
    shared_resources: &impl remagic::SharedResources,
    content_path: &PlatformPath,
    commands: Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) -> Entity {
    spawn_xna_model_detailed(
        xna_model,
        shared_resources,
        content_path,
        commands,
        meshes,
        materials,
        assets,
        None,
    )
}
/// Returns (root entity, Vec<bone entity>)
pub(crate) fn spawn_xna_model_detailed(
    xna_model: &remagic::xnb_readers::xna_mesh::Model,
    shared_resources: &impl remagic::SharedResources,
    content_path: &PlatformPath,
    mut commands: Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    skinned_mesh: Option<&bevy::mesh::skinning::SkinnedMesh>,
) -> Entity {
    let root_entity = commands
        .spawn((Transform::default(), Visibility::default()))
        .id();
    let bones_with_entities: Vec<_> = xna_model
        .bones
        .iter()
        .map(|bone| {
            let mut bone_entity = commands.spawn((
                ChildOf(root_entity),
                Transform::default(),
                Visibility::default(),
            ));
            if let Some(name) = bone.name.as_ref() {
                bone_entity.insert(Name::new(name.0.to_owned()));
            }
            (bone, bone_entity.id())
        })
        .collect();
    for (bone, bone_entity) in &bones_with_entities {
        let Some(parent_index) = bone.parent else {
            continue;
        };
        let Some((_, parent_bone_entity)) = usize::try_from(parent_index)
            .ok()
            .and_then(|i| bones_with_entities.get(i))
        else {
            warn!(
                "bone {:?} parent bone index {parent_index} out of range, only {} bones",
                &bone.name,
                bones_with_entities.len()
            );
            continue;
        };
        commands
            .entity(*bone_entity)
            .insert(ChildOf(*parent_bone_entity));
    }
    for xna_mesh in &xna_model.meshes {
        let Some(vertex_buffer) = &xna_mesh.vertex_buffer else {
            warn!("no vertex buffer for mesh");
            continue;
        };
        let Some(index_buffer) = &xna_mesh.index_buffer else {
            warn!("no index buffer for mesh");
            continue;
        };
        let mesh_parent_entity = {
            if let Some(parent_bone_index) = xna_mesh.parent_bone {
                if let Some((_, parent_bone_entity)) = usize::try_from(parent_bone_index)
                    .ok()
                    .and_then(|i| bones_with_entities.get(i))
                {
                    *parent_bone_entity
                } else {
                    warn!(
                        "mesh {:?} parent bone index {parent_bone_index} out of range, only {} bones",
                        &xna_mesh.name,
                        bones_with_entities.len()
                    );
                    root_entity
                }
            } else {
                root_entity
            }
        };
        let mut mesh_entity = commands.spawn((
            ChildOf(mesh_parent_entity),
            (Transform::default(), Visibility::default()),
        ));
        if let Some(name) = xna_mesh.name.as_ref() {
            mesh_entity.insert(Name::new(name.0.to_owned()));
        }
        let mesh_entity = mesh_entity.id();
        for mesh_part in &xna_mesh.parts {
            let Some(vertex_declaration) = usize::try_from(mesh_part.vertex_declaration_index)
                .ok()
                .and_then(|i| xna_model.vertex_declarations.get(i))
                .and_then(|o| o.as_ref())
            else {
                warn!("no vertex declaration for mesh part");
                continue;
            };

            let stream = 0;
            let vertex_stride = d3dx::vertex_size(&vertex_declaration.elements, 0);

            let incorrect_vertex_count =
                vertex_buffer.data.len().checked_div(vertex_stride).unwrap();

            // The MinIndex and NumVertices values are really just hints to help Direct3D optimize memory access during software vertex processing, and could simply be set to include the entire vertex buffer at the price of performance.
            // - https://learn.microsoft.com/en-us/windows/win32/direct3d9/rendering-from-vertex-and-index-buffers#scenario-3-drawing-one-triangle-with-indexing
            // dbg!((mesh_part.num_vertices, incorrect_vertex_count)); // TODO respect

            if mesh_part.stream_offset != 0 {
                // TODO: handle this
                error!(
                    "mesh part has non-zero stream_offset {}, don't know what to do with that",
                    mesh_part.stream_offset
                );
                continue;
            }

            let mut mesh = xna_geom::init_mesh_from_xna_vertices(
                vertex_declaration,
                vertex_buffer,
                vertex_stride,
                incorrect_vertex_count,
                stream,
            );

            // BaseVertexIndex is a value that's effectively added to every VB Index stored in the index buffer
            // - https://learn.microsoft.com/en-us/windows/win32/direct3d9/rendering-from-vertex-and-index-buffers#scenario-4-drawing-one-triangle-with-offset-indexing
            let base_vertex: u32 = mesh_part.base_vertex.try_into().unwrap();
            let start_index: usize = mesh_part.start_index.try_into().unwrap();
            let primitive_count: usize = mesh_part.primitive_count.try_into().unwrap();
            let invert_winding = xna_geom::assign_mesh_indices(
                &mut mesh,
                base_vertex,
                start_index,
                primitive_count,
                index_buffer,
            );
            // XXX: This should just be the arg passed around instead of invert_winding
            let back_face = if invert_winding {
                Face::Back
            } else {
                Face::Front
            };

            enum MaterialType {
                Standard(StandardMaterial),
                Character(super::effect::CharacterMaterial),
            }

            let (maybe_material, vertex_color_state): (Option<MaterialType>, _) = if let Some(
                effect_ref,
            ) =
                &mesh_part.effect
            {
                let effect = shared_resources.shared_resource_any(effect_ref);
                match effect {
                    Ok(Some(e)) => {
                        if let Some(e) =
                            e.downcast_ref::<remagic::xnb_readers::magicka_effect::DeferredEffect>()
                        {
                            let (mat, col) = super::effect::translate_effect_deferred(e, content_path, back_face, assets);
                            (mat.map(MaterialType::Standard), col)
                        } else if let Some(e) =
                            e.downcast_ref::<remagic::xnb_readers::skinning::SkinnedModelBasicEffect>()
                        {
                            let (mat, col) = super::effect::translate_effect_skinned_model_basic(e, content_path, back_face, assets);
                            (mat.map(MaterialType::Character), col)
                        } else {
                            warn!("mesh shared resource effect unsupported: {:?}", e.type_id());
                            (None, super::effect::VertexColorState::Disabled)
                        }
                    }
                    Ok(None) => {
                        warn!("mesh shared resource effect is null");
                        (None, super::effect::VertexColorState::Disabled)
                    }
                    Err(e) => {
                        error!("failed to access mesh shared resource effect: {:?}", e);
                        // This is here since SkinnedModels don't read to shared resources yet
                        let material = StandardMaterial {
                            base_color: Color::srgb(0., 1., 0.),
                            cull_mode: Some(if invert_winding {
                                Face::Back
                            } else {
                                Face::Front
                            }),
                            ..Default::default()
                        };
                        (
                            Some(MaterialType::Standard(material)),
                            super::effect::VertexColorState::Disabled,
                        )
                    }
                }
            } else {
                (None, super::effect::VertexColorState::Disabled)
            };

            if matches!(
                vertex_color_state,
                super::effect::VertexColorState::Disabled
            ) {
                mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
            }

            if skinned_mesh.is_none() {
                // Bevy assumes there is a SkinnedMesh if the mesh has joint attributes
                // See https://github.com/bevyengine/bevy/issues/22469
                mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
            }
            let mut mesh_part_commands = commands.spawn((
                ChildOf(mesh_entity),
                Name::new("MeshPart"),
                Mesh3d(meshes.add(mesh)),
            ));
            if let Some(skinned_mesh) = skinned_mesh {
                mesh_part_commands.insert(skinned_mesh.clone());
            }
            match maybe_material {
                Some(MaterialType::Standard(m)) => {
                    mesh_part_commands.insert(MeshMaterial3d(materials.add(m)));
                }
                Some(MaterialType::Character(m)) => {
                    mesh_part_commands.insert(MeshMaterial3d(assets.add(m)));
                }
                None => {}
            }
        }
    }
    root_entity
}
