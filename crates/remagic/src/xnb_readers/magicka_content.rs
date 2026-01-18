use crate::{
    xnb::{
        Stream, TypeReaderMeta, TypeReaderParser, object,
        types::{
            ExternalReference, Matrix, Quaternion, Vector3, bool, external_ref, f32, i32, matrix,
            quat, string, u8, u16, vec3,
        },
    },
    xnb_readers::{
        magicka_effect::{DeferredLiquidEffect, Effect, LavaEffect, effect},
        magicka_mesh::{BiTreeModel, bitree_model},
        xna_mesh::{
            IndexBuffer, Model, VertexBuffer, VertexDeclaration, index_buffer, model,
            vertex_buffer, vertex_decl,
        },
        xna_tex::Texture2d,
    },
};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use winnow::{
    Parser, Result,
    binary::length_repeat,
    combinator::{cond, repeat, seq},
    error::{ContextError, StrContext, StrContextValue},
    stream::Location,
};

#[derive(Debug)]
pub struct Level {
    pub model: Option<BiTreeModel>,
    pub animated_parts: Vec<AnimatedLevelPart>,
    pub lights: Vec<Light>,
    pub visual_effects: Vec<VisualEffect>,
    pub physics_entities: Vec<PhysicsEntity>,
    pub waters: Vec<Liquid>,
    pub force_fields: Vec<ForceField>,
    pub collision: Vec<GenericTriangleMesh>,
    pub camera_mesh: Option<GenericTriangleMesh>,
    pub trigger_areas: Vec<(String, TriggerArea)>,
    pub locators: Vec<(String, Locator)>,
    pub nav_mesh: NavMesh,
}
pub fn level_model(input: &mut Stream) -> Result<Level> {
    let model = object(bitree_model).parse_next(input)?;
    let animated_parts: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), animated_level_part).parse_next(input)?;
    let lights: Vec<_> = length_repeat(i32.try_map(usize::try_from), light).parse_next(input)?;
    let visual_effects: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), visual_effect).parse_next(input)?;
    let physics_entities =
        length_repeat(i32.try_map(usize::try_from), physics_entity).parse_next(input)?;
    let waters = length_repeat(i32.try_map(usize::try_from), liquid).parse_next(input)?;
    let force_fields =
        length_repeat(i32.try_map(usize::try_from), force_field).parse_next(input)?;
    let collision = level_collision
        .context(StrContext::Expected(StrContextValue::Description(
            "level collision",
        )))
        .parse_next(input)?;
    let camera_mesh = bool
        .flat_map(|has| cond(has, generic_triangle_mesh))
        .context(StrContext::Expected(StrContextValue::Description(
            "level camera mesh",
        )))
        .parse_next(input)?;
    let trigger_areas = length_repeat(
        i32.try_map(usize::try_from),
        (string.map(ToOwned::to_owned), trigger_area),
    )
    .context(StrContext::Expected(StrContextValue::Description(
        "level trigger areas",
    )))
    .parse_next(input)?;
    let locators = length_repeat(
        i32.try_map(usize::try_from),
        (string.map(ToOwned::to_owned), locator),
    )
    .context(StrContext::Expected(StrContextValue::Description(
        "level locators",
    )))
    .parse_next(input)?;
    let level_nav_mesh = nav_mesh
        .context(StrContext::Expected(StrContextValue::Description(
            "level nav mesh",
        )))
        .parse_next(input)?;
    Ok(Level {
        model,
        animated_parts,
        lights,
        visual_effects,
        physics_entities,
        waters,
        force_fields,
        collision,
        camera_mesh,
        trigger_areas,
        locators,
        nav_mesh: level_nav_mesh,
    })
}
impl TypeReaderMeta for Level {
    const NAME: &'static str = "Magicka.ContentReaders.LevelModelReader, Magicka";
    const VERSION: i32 = 0;
}

#[derive(Debug)]
pub struct List<T>(pub Vec<T>);
/// Wrap your reader in [`object`] if it is a reference type
fn list<'i, 'p, T: TypeReaderMeta, P: TypeReaderParser<'i, T>, MakeP: (Fn() -> P) + 'p>(
    reader: MakeP,
) -> impl Parser<Stream<'i>, List<T>, ContextError> + 'p {
    move |input: &mut Stream<'i>| {
        let inner_reader = (reader)();
        let items = length_repeat(i32.try_map(usize::try_from), inner_reader).parse_next(input)?;
        Ok(List(items))
    }
}
impl TypeReaderMeta for List<Vector3> {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.ListReader`1[[Microsoft.Xna.Framework.Vector3, Microsoft.Xna.Framework, Version=3.1.0.0, Culture=neutral, PublicKeyToken=6d5c3888ef60e27d]]";
    const VERSION: i32 = 0;
}

#[derive(Debug)]
pub struct AnimatedLevelPart {
    pub name: String,
    pub affect_shields: bool,
    pub model: Option<Model>,
    pub mesh_settings: HashMap<String, (bool, bool)>,
    pub liquids: Vec<Liquid>,
    pub locators: HashMap<String, Locator>,
    pub animation_duration: f32,
    pub animation: AnimationChannel,
    pub effects: Vec<VisualEffect>,
    pub lights: Vec<(String, Matrix)>, // (hashed, position)
    pub collision: Option<AnimatedLevelPartCollision>,
    pub nav_mesh: Option<NavMesh>,
    pub children: Vec<AnimatedLevelPart>,
}
#[derive(Debug)]
pub struct AnimatedLevelPartCollision {
    pub material: u8, // CollisionMaterial
    pub vertices: Option<List<Vector3>>,
    pub triangle_vertex_indices: Vec<(i32, i32, i32)>,
}
fn animated_level_part(input: &mut Stream) -> Result<AnimatedLevelPart> {
    fn collision(input: &mut Stream) -> Result<AnimatedLevelPartCollision> {
        let material = u8.parse_next(input)?; // CollisionMaterial
        let vertices = object(list(|| vec3)).parse_next(input)?;
        let triangle_vertex_indices: Vec<_> =
            length_repeat(i32.try_map(usize::try_from), (i32, i32, i32)).parse_next(input)?;
        Ok(AnimatedLevelPartCollision {
            material,
            vertices,
            triangle_vertex_indices,
        })
    }

    let name = string.parse_next(input)?; // to lowercase
    let affect_shields = bool.parse_next(input)?;
    let model = object(model).parse_next(input)?;
    let mesh_settings: std::collections::HashMap<_, _> =
        length_repeat(i32.try_map(usize::try_from), (string, (bool, bool))).parse_next(input)?;
    let liquids: Vec<_> = length_repeat(i32.try_map(usize::try_from), liquid).parse_next(input)?;
    let locators: std::collections::HashMap<_, _> = length_repeat(
        i32.try_map(usize::try_from)
            .verify(|n| *n < 1000)
            .context(StrContext::Expected(StrContextValue::Description(
                "number of locators",
            ))),
        (string, locator).context(StrContext::Expected(StrContextValue::Description(
            "locator entry",
        ))),
    )
    .parse_next(input)?;
    let animation_duration = f32.parse_next(input)?;
    let animation = animation_channel.parse_next(input)?;
    let effects: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), visual_effect).parse_next(input)?;
    let lights: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), (string, matrix)).parse_next(input)?;
    let collision = bool
        .flat_map(|has| cond(has, collision))
        .parse_next(input)?;
    let nav_mesh = bool.flat_map(|has| cond(has, nav_mesh)).parse_next(input)?;
    let children: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), animated_level_part).parse_next(input)?;
    Ok(AnimatedLevelPart {
        name: name.to_owned(),
        affect_shields,
        model,
        mesh_settings: mesh_settings
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        liquids,
        locators: locators
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        animation_duration,
        animation,
        effects,
        lights: lights.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        collision,
        nav_mesh,
        children,
    })
}

#[derive(Debug)]
pub struct Liquid {
    pub effect: LiquidEffect,
    pub vertices: Option<VertexBuffer>,
    pub indices: Option<IndexBuffer>,
    pub vertex_declaration: Option<VertexDeclaration>,
    pub vertex_stride: i32,
    pub num_vertices: i32,
    pub primitive_count: i32,
    pub collision: bool,
    pub freezable: bool,
    pub auto_freeze: bool,
}
fn liquid(input: &mut Stream) -> Result<Liquid> {
    let effect = effect
        .try_map(LiquidEffect::try_from)
        .context(StrContext::Expected(StrContextValue::Description(
            "RenderDeferredLiquidEffect or LavaEffect",
        )))
        .parse_next(input)?;
    let vertices = object(vertex_buffer).parse_next(input)?;
    let indices = object(index_buffer).parse_next(input)?;
    let vertex_declaration = object(vertex_decl).parse_next(input)?;
    let vertex_stride = i32.parse_next(input)?;
    let num_vertices = i32.parse_next(input)?;
    let primitive_count = i32.parse_next(input)?;
    let collision = bool.parse_next(input)?;
    let freezable = bool.parse_next(input)?;
    let auto_freeze = bool.parse_next(input)?;
    Ok(Liquid {
        effect,
        vertices,
        indices,
        vertex_declaration,
        vertex_stride,
        num_vertices,
        primitive_count,
        collision,
        freezable,
        auto_freeze,
    })
}
#[derive(Debug)]
pub enum LiquidEffect {
    DeferredLiquid(DeferredLiquidEffect),
    Lava(LavaEffect),
}
impl TryFrom<Option<Effect>> for LiquidEffect {
    type Error = LiquidError;
    fn try_from(value: Option<Effect>) -> std::result::Result<Self, Self::Error> {
        match value {
            Some(Effect::DeferredLiquid(e)) => Ok(LiquidEffect::DeferredLiquid(e)),
            Some(Effect::Lava(e)) => Ok(LiquidEffect::Lava(e)),
            None => Err(LiquidError::NullEffect),
            Some(e) => Err(LiquidError::UnsupportedEffect(Box::new(e))),
        }
    }
}
#[derive(Debug)]
pub enum LiquidError {
    NullEffect,
    UnsupportedEffect(Box<Effect>),
}
impl std::error::Error for LiquidError {}
impl std::fmt::Display for LiquidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiquidError::NullEffect => f.write_str("Effect is null"),
            LiquidError::UnsupportedEffect(_effect) => f.write_str("Effect type is unsupported"),
        }
    }
}

#[derive(Debug)]
pub struct AnimationChannel {
    /// (time, pose)
    pub keyframes: Vec<(f32, Pose)>,
}
#[derive(Debug)]
pub struct Pose {
    pub translation: Vector3,
    pub orientation: Quaternion,
    pub scale: Vector3,
}
pub(crate) fn animation_channel(input: &mut Stream) -> Result<AnimationChannel> {
    let pose = seq!(Pose {
        translation: vec3,
        orientation: quat,
        scale: vec3,
    });
    let keyframes: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), (f32, pose)).parse_next(input)?;
    Ok(AnimationChannel { keyframes })
}
#[derive(Debug)]
pub struct VisualEffect {
    pub id: String,
    pub position: Vector3,
    pub direction: Vector3,
    pub range: f32,
    pub effect: String,
}
fn visual_effect(input: &mut Stream) -> Result<VisualEffect> {
    seq!(VisualEffect {
        id: string.map(ToOwned::to_owned),
        position: vec3,
        direction: vec3,
        range: f32,
        effect: string.map(ToOwned::to_owned),
    })
    .parse_next(input)
}

#[derive(Debug)]
pub struct NavMesh {
    pub vertices: Vec<Vector3>,
    pub triangles: Vec<PathFindingTriangle>,
}
#[derive(Debug)]
pub struct PathFindingTriangle {
    pub vertices: (u16, u16, u16),
    pub neighbors: (u16, u16, u16),
    /// (a->b, b->c, c->a)
    pub costs: (f32, f32, f32),
    pub properties: u8, // MovementProperties
}
fn nav_mesh(input: &mut Stream) -> Result<NavMesh> {
    fn triangle(input: &mut Stream) -> Result<PathFindingTriangle> {
        seq!(PathFindingTriangle {
            vertices: (u16, u16, u16),
            neighbors: (u16, u16, u16),
            costs: (f32, f32, f32),
            properties: u8,
        })
        .parse_next(input)
    }
    let vertices: Vec<_> = length_repeat(u16, vec3).parse_next(input)?;
    let triangles: Vec<_> = length_repeat(u16, triangle).parse_next(input)?;
    Ok(NavMesh {
        vertices,
        triangles,
    })
}

#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    pub variation_type: LightVariationType,
    pub diffuse_color: Vector3,
    pub ambient_color: Vector3,
    pub specular_amount: f32,
    pub variation_speed: f32,
    pub variation_amount: f32,
    pub shadow_map_size: i32,
    pub cast_shadows: bool,
    pub data: LightData,
}
#[derive(Debug, Clone)]
pub enum LightData {
    Point {
        position: Vector3,
        radius: f32,
    },
    Directional {
        direction: Vector3,
    },
    Spot {
        position: Vector3,
        range: f32,
        direction: Vector3,
        cutoff_angle: f32,
        sharpness: f32,
        use_attenuation: bool,
    },
}
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum LightVariationType {
    None,
    Sine,
    Flicker,
    Candle,
    Strobe,
}
fn light(input: &mut Stream) -> Result<Light> {
    #[derive(Debug, TryFromPrimitive)]
    #[repr(i32)]
    pub enum LightType {
        Point = 0,
        Directional = 1,
        Spot = 2,
        // Custom = 10,
    }
    let text = string.parse_next(input)?;
    let position = vec3.parse_next(input)?;
    let val = vec3.parse_next(input)?;
    let light_type = i32.try_map(LightType::try_from).parse_next(input)?;
    let variation_type = i32
        .try_map(LightVariationType::try_from)
        .parse_next(input)?;
    let num2 = f32.parse_next(input)?;
    let use_attenuation = bool.parse_next(input)?;
    let cutoff_angle = f32.parse_next(input)?;
    let sharpness = f32.parse_next(input)?;
    let data = match light_type {
        LightType::Point => LightData::Point {
            position,
            radius: num2,
        },
        LightType::Directional => LightData::Directional { direction: val },
        LightType::Spot => LightData::Spot {
            position,
            range: num2,
            direction: val,
            cutoff_angle,
            sharpness,
            use_attenuation,
        },
        // LightType::Custom => todo!(),
    };
    Ok(Light {
        name: text.to_owned(),
        variation_type,
        diffuse_color: vec3.parse_next(input)?,
        ambient_color: vec3.parse_next(input)?,
        specular_amount: f32.parse_next(input)?,
        variation_speed: f32.parse_next(input)?,
        variation_amount: f32.parse_next(input)?,
        shadow_map_size: i32.parse_next(input)?,
        cast_shadows: bool.parse_next(input)?,
        data,
    })
}

#[derive(Debug)]
pub struct PhysicsEntity {
    pub transform: Matrix,
    /// Load from Content/Data/PhysicsEntities/<template_name>
    pub template_base_name: String,
}
fn physics_entity(input: &mut Stream) -> Result<PhysicsEntity> {
    seq!(PhysicsEntity {
        transform: matrix,
        template_base_name: string.map(ToOwned::to_owned)
    })
    .parse_next(input)
}

#[derive(Debug)]
pub struct ForceField {
    pub material_color: Vector3,
    pub material_width: f32,
    pub material_alpha_power: f32,
    pub material_alpha_falloff_power: f32,
    pub material_max_radius: f32,
    pub material_ripple_distortion: f32,
    pub material_map_distortion: f32,
    pub material_vertex_color_enabled: bool,
    pub material_displacement_map: ExternalReference<Texture2d>,
    pub ttl: f32,
    pub vertices: Option<VertexBuffer>,
    pub indices: Option<IndexBuffer>,
    pub vertex_declaration: Option<VertexDeclaration>,
    pub vertex_stride: i32,
    pub num_vertices: i32,
    pub primitive_count: i32,
}
fn force_field(input: &mut Stream) -> Result<ForceField> {
    seq!(ForceField {
        material_color: vec3,
        material_width: f32,
        material_alpha_power: f32,
        material_alpha_falloff_power: f32,
        material_max_radius: f32,
        material_ripple_distortion: f32,
        material_map_distortion: f32,
        material_vertex_color_enabled: bool,
        material_displacement_map: external_ref,
        ttl: f32,
        vertices: object(vertex_buffer),
        indices: object(index_buffer),
        vertex_declaration: object(vertex_decl),
        vertex_stride: i32,
        num_vertices: i32,
        primitive_count: i32,
    })
    .parse_next(input)
}

#[derive(Debug, Default)]
pub struct GenericTriangleMesh {
    pub vertices: Vec<Vector3>,
    pub indices: Vec<(i32, i32, i32)>,
}
fn level_collision(input: &mut Stream) -> Result<Vec<GenericTriangleMesh>> {
    // TODO: Can this fill a fixed length slice instead?
    repeat::<_, _, Vec<_>, _, _>(
        10,
        bool.flat_map(|has| {
            cond(has, generic_triangle_mesh).map(|maybe| maybe.unwrap_or_default())
        }),
    )
    .parse_next(input)
}
fn generic_triangle_mesh(input: &mut Stream) -> Result<GenericTriangleMesh> {
    seq!(GenericTriangleMesh {
        vertices: object(list(|| vec3)).map(|l| l.map_or_else(Default::default, |l| l.0)),
        indices: length_repeat(i32.try_map(usize::try_from), (i32, i32, i32),),
    })
    .context(StrContext::Expected(StrContextValue::Description(
        "triangle mesh",
    )))
    .parse_next(input)
}

#[derive(Debug, Clone)]
pub struct TriggerArea {
    pub position: Vector3,
    pub side_lengths: Vector3,
    pub orientation: Quaternion,
}
fn trigger_area(input: &mut Stream) -> Result<TriggerArea> {
    seq!(TriggerArea {
        position: vec3,
        side_lengths: vec3,
        orientation: quat,
    })
    .parse_next(input)
}

#[derive(Debug)]
pub struct Locator {
    pub transform: Matrix,
    pub radius: f32,
}
fn locator(input: &mut Stream) -> Result<Locator> {
    seq!(Locator {
        transform: matrix,
        radius: f32,
    })
    .parse_next(input)
}
