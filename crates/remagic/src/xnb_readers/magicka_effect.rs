use crate::{
    xnb::{
        Stream, TypeReaderMeta, object,
        types::{ExternalReference, Vector2, Vector3, bool, external_ref, f32, vec2, vec3},
    },
    xnb_readers::xna_mesh::{Texture2d, TextureCube},
};
use winnow::{
    Parser as _, Result,
    combinator::{alt, cond, seq},
    error::StrContext,
};
#[derive(Debug)]
pub enum Effect {
    Deferred(DeferredEffect),
    Additive(AdditiveEffect),
    DeferredLiquid(DeferredLiquidEffect),
    Lava(LavaEffect),
}

pub(crate) fn effect(input: &mut Stream) -> Result<Option<Effect>> {
    pub enum InnerEffect {
        Deferred(Option<DeferredEffect>),
        Additive(Option<AdditiveEffect>),
        DeferredLiquid(Option<DeferredLiquidEffect>),
        Lava(Option<LavaEffect>),
    }
    alt((
        object(deferred_effect).map(InnerEffect::Deferred),
        object(additive_effect).map(InnerEffect::Additive),
        object(deferred_liquid_effect).map(InnerEffect::DeferredLiquid),
        object(lava_effect).map(InnerEffect::Lava),
    ))
    .map(|inner| {
        Some(match inner {
            InnerEffect::Deferred(e) => Effect::Deferred(e?),
            InnerEffect::Additive(e) => Effect::Additive(e?),
            InnerEffect::DeferredLiquid(e) => Effect::DeferredLiquid(e?),
            InnerEffect::Lava(e) => Effect::Lava(e?),
        })
    })
    .parse_next(input)
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct DeferredEffect {
    pub Alpha: f32,
    pub Sharpness: f32,
    pub VertexColorEnabled: bool,
    pub UseMaterialTextureForReflectiveness: bool,
    pub ReflectionMap: ExternalReference<TextureCube>,
    pub Layer0: DeferredEffectLayer,
    pub Layer1: Option<DeferredEffectLayer>,
}
#[allow(non_snake_case)]
#[derive(Debug)]
pub struct DeferredEffectLayer {
    pub DiffuseTexture0AlphaDisabled: bool,
    pub AlphaMask0Enabled: bool,
    pub DiffuseColor0: Vector3,
    pub SpecAmount0: f32,
    pub SpecPower0: f32,
    pub EmissiveAmount0: f32,
    pub NormalPower0: f32,
    pub Reflectiveness0: f32,
    pub DiffuseTexture0: ExternalReference<Texture2d>,
    pub MaterialTexture0: ExternalReference<Texture2d>,
    pub NormalTexture0: ExternalReference<Texture2d>,
}
#[allow(non_snake_case)]
pub(crate) fn deferred_effect(input: &mut Stream) -> Result<DeferredEffect> {
    pub fn layer(input: &mut Stream) -> Result<DeferredEffectLayer> {
        seq!(DeferredEffectLayer {
            DiffuseTexture0AlphaDisabled: bool,
            AlphaMask0Enabled: bool,
            DiffuseColor0: vec3,
            SpecAmount0: f32,
            SpecPower0: f32,
            EmissiveAmount0: f32,
            NormalPower0: f32,
            Reflectiveness0: f32,
            DiffuseTexture0: external_ref,
            MaterialTexture0: external_ref,
            NormalTexture0: external_ref,
        })
        .parse_next(input)
    }
    seq!(DeferredEffect {
        Alpha: f32,
        Sharpness: f32,
        VertexColorEnabled: bool,
        UseMaterialTextureForReflectiveness: bool,
        ReflectionMap: external_ref,
        Layer0: layer,
        Layer1: bool.flat_map(|has| cond(has, layer))
    })
    .parse_next(input)
}
impl TypeReaderMeta for DeferredEffect {
    const NAME: &'static str = "PolygonHead.Pipeline.RenderDeferredEffectReader, PolygonHead, Version=1.0.0.0, Culture=neutral";

    const VERSION: i32 = 0;
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct AdditiveEffect {
    pub ColorTint: Vector3,
    pub VertexColorEnabled: bool,
    pub TextureEnabled: bool,
    pub Texture: ExternalReference<Texture2d>,
}
#[allow(non_snake_case)]
fn additive_effect(input: &mut Stream) -> Result<AdditiveEffect> {
    seq!(AdditiveEffect {
        ColorTint: vec3,
        VertexColorEnabled: bool,
        TextureEnabled: bool,
        Texture: external_ref,
    })
    .parse_next(input)
}
impl TypeReaderMeta for AdditiveEffect {
    const NAME: &'static str =
        "PolygonHead.Pipeline.AdditiveEffectReader, PolygonHead, Version=1.0.0.0, Culture=neutral";

    const VERSION: i32 = 0;
}

#[derive(Debug)]
pub struct DeferredLiquidEffect {
    // HACK: This should actually be an ExternalReference<Texture> that gets forced to null if it doesn't point to TextureCube
    pub reflection_map: ExternalReference<TextureCube>,
    pub wave_height: f32,
    pub wave_speed0: Vector2,
    pub wave_speed1: Vector2,
    pub water_reflectiveness: f32,
    pub bottom_color: Vector3,
    pub deep_bottom_color: Vector3,
    pub water_emissive_amount: f32,
    pub water_spec_amount: f32,
    pub water_spec_power: f32,
    pub bottom_texture: ExternalReference<Texture2d>,
    pub water_normal_map: ExternalReference<Texture2d>,
    pub ice_reflectiveness: f32,
    pub ice_color: Vector3,
    pub ice_emissive_amount: f32,
    pub ice_spec_amount: f32,
    pub ice_spec_power: f32,
    pub ice_diffuse_map: ExternalReference<Texture2d>,
    pub ice_normal_map: ExternalReference<Texture2d>,
}
impl TypeReaderMeta for DeferredLiquidEffect {
    const NAME: &'static str = "PolygonHead.Pipeline.RenderDeferredLiquidEffectReader, PolygonHead";
    const VERSION: i32 = 0;
}
fn deferred_liquid_effect(input: &mut Stream) -> Result<DeferredLiquidEffect> {
    seq!(DeferredLiquidEffect {
        reflection_map: external_ref,
        wave_height: f32,
        wave_speed0: vec2,
        wave_speed1: vec2,
        water_reflectiveness: f32,
        bottom_color: vec3,
        deep_bottom_color: vec3,
        water_emissive_amount: f32,
        water_spec_amount: f32,
        water_spec_power: f32,
        bottom_texture: external_ref,
        water_normal_map: external_ref,
        ice_reflectiveness: f32,
        ice_color: vec3,
        ice_emissive_amount: f32,
        ice_spec_amount: f32,
        ice_spec_power: f32,
        ice_diffuse_map: external_ref,
        ice_normal_map: external_ref,
    })
    .parse_next(input)
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct LavaEffect {}
fn lava_effect(input: &mut Stream) -> Result<LavaEffect> {
    winnow::combinator::todo
        .context(StrContext::Label("LavaEffect"))
        .parse_next(input)
}
impl TypeReaderMeta for LavaEffect {
    const NAME: &'static str = "UNKNOWN";
    const VERSION: i32 = 0;
}
