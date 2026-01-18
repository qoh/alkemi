use num_enum::TryFromPrimitive;
use winnow::{
    Parser, Result,
    binary::length_repeat,
    combinator::{cond, repeat, seq},
    error::ContextError,
};

use crate::{
    xnb::{
        SharedResourceReference, Stream, TypeReaderMeta, object, quicklist, shared_resource_ref,
        types::*,
    },
    xnb_readers::{
        magicka_content::{AnimationChannel, Pose, animation_channel},
        xna_mesh::{Model, model},
        xna_tex::{Texture2d, texture_2d},
    },
};

#[derive(Debug)]
pub struct SkinnedModel {
    pub model: Option<Model>,
    pub bones: Vec<Option<SharedResourceReference<SkinnedModelBone>>>,
    pub animations: Vec<Option<SharedResourceReference<AnimationClip>>>,
    // bones
    // animations
}

impl TypeReaderMeta for SkinnedModel {
    const NAME: &'static str =
        "XNAnimation.Pipeline.SkinnedModelReader, XNAnimation, Version=0.7.0.0, Culture=neutral";

    const VERSION: i32 = 0;
}

pub fn skinned_model(input: &mut Stream) -> Result<SkinnedModel> {
    seq!(SkinnedModel {
        model: object(model),
        bones: quicklist(shared_resource_ref),
        animations: quicklist(shared_resource_ref),
    })
    .parse_next(input)
    // let model = object(model).parse_next(input)?;
    // let bones = quicklist(shared_resource_ref);
    // let animations = quicklist(shared_resource_ref);
    // // bones
    // // animations
    // Ok(SkinnedModel { model })
}

#[derive(Debug)]
pub struct SkinnedModelBone {
    pub index: u16,
    pub name: String,
    pub bind_pose: Pose,
    pub inverse_bind_pose_transform: Matrix,
    pub parent_bone: Option<SharedResourceReference<SkinnedModelBone>>,
    pub child_bones: Vec<Option<SharedResourceReference<SkinnedModelBone>>>,
}
impl TypeReaderMeta for SkinnedModelBone {
    const NAME: &'static str = "XNAnimation.Pipeline.SkinnedModelBoneReader, XNAnimation, Version=0.7.0.0, Culture=neutral";
    const VERSION: i32 = 0;
}
impl SkinnedModelBone {
    pub fn parse(input: &mut Stream) -> Result<Self> {
        seq!(SkinnedModelBone {
            index: u16,
            name: string.map(ToOwned::to_owned),
            bind_pose: seq!(Pose {
                translation: vec3,
                orientation: quat,
                scale: vec3,
            }),
            inverse_bind_pose_transform: matrix,
            parent_bone: shared_resource_ref,
            child_bones: quicklist(shared_resource_ref),
        })
        .parse_next(input)
    }
}

#[derive(Debug)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<(String, AnimationChannel)>,
}
impl TypeReaderMeta for AnimationClip {
    const NAME: &'static str =
        "XNAnimation.Pipeline.AnimationClipReader, XNAnimation, Version=0.7.0.0, Culture=neutral";
    const VERSION: i32 = 0;
}
impl AnimationClip {
    pub fn parse(input: &mut Stream) -> Result<Self> {
        seq!(AnimationClip {
            name: string.map(ToOwned::to_owned),
            duration: f32,
            channels: quicklist((string.map(ToOwned::to_owned), animation_channel)),
        })
        .parse_next(input)
    }
}

#[derive(Debug)]
pub struct SkinnedModelBasicEffect {
    pub technique: Technique,
    pub emissive_amount: f32,
    pub diffuse_color: Vector3,
    pub specular_amount: f32,
    pub specular_power: f32,
    pub diffuse_1_alpha: f32,
    pub use_soft_light_blend: bool,
    pub diffuse_map_0_enabled: bool,
    pub diffuse_map_1_enabled: bool,
    pub specular_map_enabled: bool,
    pub damage_map_0_enabled: bool,
    pub damage_map_1_enabled: bool,
    pub normal_map_enabled: bool,
    // HACK: This is supposed to be embedded Texture2D objects,
    // but for some reason the type ID says there are external references instead?
    // pub diffuse_map_0: Option<Texture2d>,
    // pub diffuse_map_1: Option<Texture2d>,
    // pub specular_map: Option<Texture2d>,
    // pub damage_map_0: Option<Texture2d>,
    // pub damage_map_1: Option<Texture2d>,
    // pub normal_map: Option<Texture2d>,
    pub diffuse_map_0: Option<ExternalReference<Texture2d>>,
    pub diffuse_map_1: Option<ExternalReference<Texture2d>>,
    pub specular_map: Option<ExternalReference<Texture2d>>,
    pub damage_map_0: Option<ExternalReference<Texture2d>>,
    pub damage_map_1: Option<ExternalReference<Texture2d>>,
    pub normal_map: Option<ExternalReference<Texture2d>>,
}

impl TypeReaderMeta for SkinnedModelBasicEffect {
    const NAME: &'static str = "XNAnimation.Pipeline.SkinnedModelBasicEffectReader, XNAnimation, Version=0.7.0.0, Culture=neutral";
    const VERSION: i32 = 0;
}

impl SkinnedModelBasicEffect {
    pub(crate) fn parse(input: &mut Stream) -> Result<Self> {
        seq!(Self {
            technique: u8.try_map(Technique::try_from),
            emissive_amount: f32,
            diffuse_color: vec3,
            specular_amount: f32,
            specular_power: f32,
            diffuse_1_alpha: f32,
            use_soft_light_blend: bool,
            diffuse_map_0_enabled: bool,
            diffuse_map_1_enabled: bool,
            specular_map_enabled: bool,
            damage_map_0_enabled: bool,
            damage_map_1_enabled: bool,
            normal_map_enabled: bool,
            // HACK: This is supposed to be embedded Texture2D objects,
            // but for some reason the type ID says there are external references instead?
            // diffuse_map_0: object(texture_2d),
            // diffuse_map_1: object(texture_2d),
            // specular_map: object(texture_2d),
            // damage_map_0: object(texture_2d),
            // damage_map_1: object(texture_2d),
            // normal_map: object(texture_2d),
            diffuse_map_0: object(external_ref),
            diffuse_map_1: object(external_ref),
            specular_map: object(external_ref),
            damage_map_0: object(external_ref),
            damage_map_1: object(external_ref),
            normal_map: object(external_ref),
        })
        .parse_next(input)
    }
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum Technique {
    Default,
    AlphaBlended,
    Additive,
    Depth,
    Shadow,
    NonDeffered,
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_template() {
        let bytes = std::fs::read(
            "/data/SteamLibrary/steamapps/common/Magicka/Content/Models/Characters/Wizard/avatar_purple_mesh_0.xnb",
        )
        .unwrap();
        let template = crate::parse_skinned_model(&bytes)
            .map_err(|e| e.into_inner())
            .unwrap();
        dbg!(template.inner());
    }
}
