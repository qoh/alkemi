// Could consider using this for waves:
// https://github.com/Neopallium/bevy_water

use crate::magicka_level_model::effect::{VertexColorState, find_image};

use super::{Spawner, effect, xna_geom};
use bevy::{prelude::*, render::render_resource::Face};
use remagic::xnb_readers::magicka_effect::{DeferredLiquidEffect, LavaEffect};
use typed_path::PlatformPath;

#[derive(Component, Reflect)]
pub struct Liquid {
    pub freezable: bool,
}

pub(crate) fn spawn_liquid(
    Spawner::Parent(parent): Spawner,
    liquid: &remagic::xnb_readers::magicka_content::Liquid,
    content_path: &PlatformPath,
    mut meshes: &mut Assets<Mesh>,
    mut materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) -> Entity {
    let liquid_component = Liquid {
        freezable: liquid.freezable,
    };

    let (
        Some(vertices),
        Some(indices),
        Some(vertex_declaration),
        Ok(vertex_stride),
        Ok(num_vertices),
        Ok(primitive_count),
    ) = (
        liquid.vertices.as_ref(),
        liquid.indices.as_ref(),
        liquid.vertex_declaration.as_ref(),
        usize::try_from(liquid.vertex_stride),
        usize::try_from(liquid.num_vertices),
        usize::try_from(liquid.primitive_count),
    )
    else {
        error!("liquid has missing or invalid mesh data");
        return parent.spawn((Name::new("Liquid"), liquid_component)).id();
    };

    let stream = 0;
    let mut mesh = xna_geom::init_mesh_from_xna_vertices(
        vertex_declaration,
        vertices,
        vertex_stride,
        num_vertices,
        stream,
    );
    let invert_winding = xna_geom::assign_mesh_indices(&mut mesh, 0, 0, primitive_count, indices);
    // XXX: This should just be the arg passed around instead of invert_winding
    let back_face = if invert_winding {
        Face::Back
    } else {
        Face::Front
    };

    let (material, vertex_color_state) = match &liquid.effect {
        remagic::xnb_readers::magicka_content::LiquidEffect::DeferredLiquid(effect) => {
            translate_effect_liquid(effect, content_path, back_face, assets)
        }
        remagic::xnb_readers::magicka_content::LiquidEffect::Lava(effect) => {
            translate_effect_lava(effect, content_path, back_face, assets)
        }
    };
    if matches!(vertex_color_state, effect::VertexColorState::Disabled) {
        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
    }

    info!("liquid has unhandled collision={:?}", liquid.collision);
    if liquid.auto_freeze {
        info!("liquid has unhandled auto_freeze=true");
    }

    parent
        .spawn((
            Name::new("Liquid"),
            liquid_component,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
        ))
        .id()
}

fn translate_effect_liquid(
    effect: &DeferredLiquidEffect,
    content_path: &PlatformPath,
    back_face: Face,
    assets: &AssetServer,
) -> (StandardMaterial, VertexColorState) {
    let DeferredLiquidEffect {
        ref reflection_map,
        wave_height,
        wave_speed0,
        wave_speed1,
        water_reflectiveness,
        bottom_color,
        deep_bottom_color,
        water_emissive_amount,
        water_spec_amount,
        water_spec_power,
        ref bottom_texture,
        ref water_normal_map,
        // TODO: The material to use when frozen
        ice_reflectiveness: _,
        ice_color: _,
        ice_emissive_amount: _,
        ice_spec_amount: _,
        ice_spec_power: _,
        ice_diffuse_map: _,
        ice_normal_map: _,
    } = *effect;

    // TODO: bottom_color looks right on wc_s3 (tutorial), but deep_bottom_color looks right on wc_s4 (exiting castle)
    // let diffuse_color = (bottom_color.0, bottom_color.1, bottom_color.2);
    // let diffuse_color = (
    //     bottom_color.0 * deep_bottom_color.0,
    //     bottom_color.1 * deep_bottom_color.1,
    //     bottom_color.2 * deep_bottom_color.2,
    // );
    let diffuse_color = (
        bottom_color.0 * 0.5 + deep_bottom_color.0 * 0.5,
        bottom_color.1 * 0.5 + deep_bottom_color.1 * 0.5,
        bottom_color.2 * 0.5 + deep_bottom_color.2 * 0.5,
    );
    // This is the same logic as in effect::blend_base_emissive
    let base_color_scale = (1. - water_emissive_amount).max(0.);

    let material = StandardMaterial {
        base_color: Color::linear_rgb(
            diffuse_color.0 * base_color_scale,
            diffuse_color.1 * base_color_scale,
            diffuse_color.2 * base_color_scale,
        ),
        // base_color_channel: todo!(),
        // base_color_texture: todo!(),
        emissive: LinearRgba::new(
            diffuse_color.0 * water_emissive_amount,
            diffuse_color.1 * water_emissive_amount,
            diffuse_color.2 * water_emissive_amount,
            1.,
        ),
        // emissive_exposure_weight: todo!(),
        // emissive_channel: todo!(),
        // emissive_texture: todo!(),
        // perceptual_roughness: todo!(),
        // metallic: todo!(),
        // metallic_roughness_channel: todo!(),
        // metallic_roughness_texture: todo!(),
        reflectance: water_reflectiveness,
        // specular_tint: todo!(),
        // diffuse_transmission: todo!(),
        // specular_transmission: todo!(),
        // thickness: todo!(),
        // ior: todo!(),
        // attenuation_distance: todo!(),
        // attenuation_color: todo!(),
        // normal_map_channel: todo!(),
        // normal_map_texture: todo!(),
        normal_map_texture: if !water_normal_map.path.is_empty() {
            Some(assets.load_with_settings_override(
                find_image(water_normal_map.path.as_str(), content_path),
                |s: &mut crate::magicka_assets::image::MagickaTexture2dLoaderSettings| {
                    s.is_srgb = false;
                },
            ))
        } else {
            None
        },
        // flip_normal_map_y: todo!(),
        // occlusion_channel: todo!(),
        // occlusion_texture: todo!(),
        // clearcoat: todo!(),
        // clearcoat_perceptual_roughness: todo!(),
        // anisotropy_strength: todo!(),
        // anisotropy_rotation: todo!(),
        // double_sided: todo!(),
        // cull_mode: todo!(),
        // unlit: todo!(),
        // fog_enabled: todo!(),
        // alpha_mode: todo!(),
        // depth_bias: todo!(),
        // depth_map: todo!(),
        // parallax_depth_scale: todo!(),
        // parallax_mapping_method: todo!(),
        // max_parallax_layer_count: todo!(),
        // lightmap_exposure: todo!(),
        // opaque_render_method: todo!(),
        // deferred_lighting_pass_id: todo!(),
        // uv_transform: todo!(),
        ..default()
    };
    (material, effect::VertexColorState::Disabled)
}

fn translate_effect_lava(
    effect: &LavaEffect,
    _content_path: &PlatformPath,
    back_face: Face,
    _assets: &AssetServer,
) -> (StandardMaterial, VertexColorState) {
    warn!("unhandled liquid effect {effect:#?}");
    let material = StandardMaterial {
        base_color: Color::linear_rgba(1., 0.1, 0.1, 1.),
        cull_mode: None, // XXX: Is this needed?
        ..default()
    };
    (material, effect::VertexColorState::Disabled)
}
