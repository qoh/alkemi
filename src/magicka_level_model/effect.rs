use bevy::{pbr::ExtendedMaterial, prelude::*, render::render_resource::Face};
use remagic::xnb_readers::{
    magicka_effect::{AdditiveEffect, DeferredEffect, Effect},
    skinning::SkinnedModelBasicEffect,
};
use typed_path::{PlatformPath, PlatformPathBuf};

use crate::magicka_level_model::map_vec3;

#[derive(Debug, Clone, Copy)]
pub enum VertexColorState {
    Enabled,
    Disabled,
}

pub(crate) fn translate_effect(
    effect: Option<&Effect>,
    content_path: &PlatformPath,
    invert_winding: bool,
    assets: &AssetServer,
) -> (Option<StandardMaterial>, VertexColorState) {
    let Some(effect) = effect else {
        warn!("null effect");
        return (None, VertexColorState::Disabled);
    };

    // XXX: This should just be the arg passed around instead of invert_winding
    let back_face = if invert_winding {
        Face::Back
    } else {
        Face::Front
    };

    match effect {
        Effect::Deferred(effect) => {
            translate_effect_deferred(effect, content_path, back_face, assets)
        }
        Effect::Additive(effect) => translate_effect_additive(effect, content_path, assets),
        _ => {
            warn!("unhandled effect {:#?}", effect);
            (None, VertexColorState::Disabled)
        }
    }
}

pub(crate) fn translate_effect_deferred(
    effect: &DeferredEffect,
    content_path: &PlatformPath,
    back_face: Face,
    assets: &AssetServer,
) -> (Option<StandardMaterial>, VertexColorState) {
    let base_color_texture = if !effect.Layer0.DiffuseTexture0.path.is_empty() {
        Some(assets.load_override(find_image(
            effect.Layer0.DiffuseTexture0.path.as_str(),
            content_path,
        )))
    } else {
        None
    };
    let normal_map_texture = if !effect.Layer0.NormalTexture0.path.is_empty() {
        Some(assets.load_with_settings_override(
            find_image(effect.Layer0.NormalTexture0.path.as_str(), content_path),
            |s: &mut crate::magicka_assets::image::MagickaTexture2dLoaderSettings| {
                s.is_srgb = false;
            },
        ))
    } else {
        None
    };
    if effect.Alpha < 1. {
        warn!("unsupported: effect alpha {} < 1", effect.Alpha);
    }
    let material = StandardMaterial {
        base_color: Color::WHITE.with_alpha(effect.Alpha),
        cull_mode: if !effect.Layer0.AlphaMask0Enabled {
            None
        } else {
            Some(back_face)
        },
        base_color_texture,
        emissive: LinearRgba::new(
            effect.Layer0.EmissiveAmount0,
            effect.Layer0.EmissiveAmount0,
            effect.Layer0.EmissiveAmount0,
            1.,
        ),
        // emissive_exposure_weight: todo!(),
        // emissive_channel: todo!(),
        // emissive_texture: todo!(),
        // perceptual_roughness: todo!(),
        // metallic: todo!(),
        // metallic_roughness_channel: todo!(),
        // metallic_roughness_texture: todo!(),
        reflectance: effect.Layer0.Reflectiveness0,
        // reflectance: todo!(),
        // specular_tint: todo!(),
        // diffuse_transmission: todo!(),
        // specular_transmission: todo!(),
        // thickness: todo!(),
        // ior: todo!(),
        // attenuation_distance: todo!(),
        // attenuation_color: todo!(),
        // normal_map_channel: todo!(),
        normal_map_texture,
        flip_normal_map_y: true,
        // occlusion_channel: todo!(),
        // occlusion_texture: todo!(),
        // clearcoat: todo!(),
        // clearcoat_perceptual_roughness: todo!(),
        // anisotropy_strength: todo!(),
        // anisotropy_rotation: todo!(),
        // double_sided: todo!(),
        double_sided: !effect.Layer0.AlphaMask0Enabled,
        // unlit: todo!(),
        // fog_enabled: todo!(),
        alpha_mode: if !effect.Layer0.AlphaMask0Enabled
        /* || !effect.Layer1.as_ref().is_some_and(|l| l.AlphaMask0Enabled)*/
        {
            AlphaMode::Mask(0.001)
        } else {
            AlphaMode::Opaque
        },
        // depth_bias: todo!(),
        // depth_map: todo!(),
        // parallax_depth_scale: todo!(),
        // parallax_mapping_method: todo!(),
        // max_parallax_layer_count: todo!(),
        // lightmap_exposure: todo!(),
        // opaque_render_method: todo!(),
        // deferred_lighting_pass_id: todo!(),
        // uv_transform: todo!(),
        ..Default::default()
    };
    (
        Some(material),
        if false
        /* effect.VertexColorEnabled */
        {
            // This introduces artifacts, it is unclear when it should be used
            VertexColorState::Enabled
        } else {
            VertexColorState::Disabled
        },
    )
}

pub(crate) fn translate_effect_additive(
    effect: &AdditiveEffect,
    content_path: &PlatformPath,
    assets: &AssetServer,
) -> (Option<StandardMaterial>, VertexColorState) {
    let material = StandardMaterial {
        base_color: Srgba::from_vec3(map_vec3(effect.ColorTint)).into(),
        base_color_texture: effect
            .TextureEnabled
            .then(|| assets.load_override(find_image(&effect.Texture.path, content_path))),
        alpha_mode: AlphaMode::Add,
        ..default()
    };
    let vertex_color = if effect.VertexColorEnabled {
        VertexColorState::Enabled
    } else {
        VertexColorState::Disabled
    };
    (Some(material), vertex_color)
}

pub type CharacterMaterial = ExtendedMaterial<StandardMaterial, CharacterExtension>;

pub(crate) fn translate_effect_skinned_model_basic(
    effect: &SkinnedModelBasicEffect,
    content_path: &PlatformPath,
    back_face: Face,
    assets: &AssetServer,
) -> (Option<CharacterMaterial>, VertexColorState) {
    use remagic::xnb_readers::skinning::Technique;
    match effect.technique {
        Technique::Default => {
            let SkinnedModelBasicEffect {
                technique: _,
                emissive_amount,
                diffuse_color,
                specular_amount,
                specular_power,
                diffuse_1_alpha,
                use_soft_light_blend,
                diffuse_map_0_enabled,
                diffuse_map_1_enabled,
                specular_map_enabled,
                damage_map_0_enabled,
                damage_map_1_enabled,
                normal_map_enabled,
                ref diffuse_map_0,
                ref diffuse_map_1,
                ref specular_map,
                ref damage_map_0,
                ref damage_map_1,
                ref normal_map,
            } = *effect;
            let (base_color, emissive) = blend_base_emissive(diffuse_color, 0.);
            let base = StandardMaterial {
                base_color,
                base_color_texture: if diffuse_map_0_enabled && let Some(tex_ref) = diffuse_map_0 {
                    Some(assets.load_override(find_image(tex_ref.path.as_str(), content_path)))
                } else {
                    None
                },
                emissive,
                normal_map_texture: if normal_map_enabled && let Some(tex_ref) = normal_map {
                    Some(assets.load_with_settings_override(
                        find_image(tex_ref.path.as_str(), content_path),
                        |s: &mut crate::magicka_assets::image::MagickaTexture2dLoaderSettings| {
                            s.is_srgb = false;
                        },
                    ))
                } else {
                    None
                },
                ..default()
            };
            let material = CharacterMaterial {
                base,
                extension: CharacterExtension {
                    color: LinearRgba::rgb(240. / 255., 20. / 255., 20. / 255.),
                    ..default()
                },
            };
            (Some(material), VertexColorState::Enabled)
        }
        // Technique::AlphaBlended => todo!(),
        // Technique::Additive => todo!(),
        // Technique::Depth => todo!(),
        // Technique::Shadow => todo!(),
        // Technique::NonDeffered => todo!(),
        tech => {
            warn!("unimplemented skinned model effect technique {:?}", tech);
            (
                None,
                if false {
                    VertexColorState::Enabled
                } else {
                    VertexColorState::Disabled
                },
            )
        }
    }
}

fn blend_base_emissive(
    diffuse_color: remagic::xnb::types::Vector3,
    emissiveness: f32,
) -> (Color, LinearRgba) {
    let base_color_scale = (1. - emissiveness).max(0.);
    let base_color = Color::linear_rgb(
        diffuse_color.0 * base_color_scale,
        diffuse_color.1 * base_color_scale,
        diffuse_color.2 * base_color_scale,
    );
    let emissive = LinearRgba::new(
        diffuse_color.0 * emissiveness,
        diffuse_color.1 * emissiveness,
        diffuse_color.2 * emissiveness,
        1.,
    );
    (base_color, emissive)
}

pub(crate) fn find_image(relative_path: &str, from_path: &PlatformPath) -> String {
    use std::ffi::OsStr;
    use typed_path::Utf8WindowsPath;

    let relative_path = Utf8WindowsPath::new(relative_path);
    let image_content_path = from_path
        .parent()
        .unwrap()
        .join(relative_path.with_platform_encoding().as_bytes_path());
    let mut absolute_path = crate::magicka_assets::content_root()
        .join_checked(&image_content_path)
        .unwrap();
    // TODO panic free
    assert_eq!(None, absolute_path.extension());
    assert!(absolute_path.set_extension("xnb"));
    if !matches!(std::fs::exists(absolute_path.as_ref() as &OsStr), Ok(true)) {
        let found_path = crate::magicka_assets::find_path_ignore_ascii_case(std::path::Path::new(
            absolute_path.as_ref() as &OsStr,
        ))
        .unwrap();
        absolute_path = PlatformPath::new(found_path.as_os_str().as_encoded_bytes()).to_owned();
    }
    let os_str = std::convert::AsRef::<OsStr>::as_ref(&absolute_path);
    os_str.to_str().unwrap().to_owned()
}

use bevy::{pbr::MaterialExtension, render::render_resource::AsBindGroup, shader::ShaderRef};

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct CharacterExtension {
    #[uniform(100)]
    color: LinearRgba, // HACK: Alpha is unused
    // WebGL2 support: structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_8b: u32,
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_12b: u32,
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_16b: u32,
}

const SHADER_ASSET_PATH: &str = "character_material.wgsl";

impl MaterialExtension for CharacterExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
