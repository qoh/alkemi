use num_enum::TryFromPrimitive;
use winnow::{
    Parser, Result,
    binary::{length_repeat, length_take, u8},
    combinator::seq,
    error::{ContextError, StrContext, StrContextValue},
};

use crate::{
    xnb::{
        SharedResourceReference, Stream, TypeReaderMeta, object, object_any, shared_resource_ref,
        types::*,
    },
    xnb_readers::magicka_effect::Effect,
};

#[derive(Debug)]
pub struct Model {
    pub bones: Vec<ModelBone>,
    pub vertex_declarations: Vec<Option<VertexDeclaration>>,
    pub meshes: Vec<Mesh>,
    pub root_bone: Option<i32>,
}
impl TypeReaderMeta for Model {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.ModelReader";
    const VERSION: i32 = 0;
}
pub(crate) fn model(input: &mut Stream) -> Result<Model> {
    fn bone_ref<'i>(
        bone_count: usize,
    ) -> impl winnow::Parser<Stream<'i>, Option<i32>, ContextError> {
        move |input: &mut Stream<'i>| {
            let num = if bone_count + 1 > 255 {
                i32.parse_next(input)
            } else {
                u8.map(i32::from).parse_next(input)
            }?;
            Ok(if num == 0 { None } else { Some(num - 1) })
        }
    }
    fn bones(input: &mut Stream) -> Result<Vec<ModelBone>> {
        let bones: Vec<_> = length_repeat(
            i32.try_map(usize::try_from),
            (
                object(string_object).context(StrContext::Label("bone name")),
                matrix,
            ),
        )
        .parse_next(input)?;
        let bone_count = bones.len();
        let bones: Vec<_> = bones
            .into_iter()
            .map(|(name, transform)| {
                let parent = bone_ref(bone_count).parse_next(input)?;
                let children: Vec<_> =
                    length_repeat(i32.try_map(usize::try_from), bone_ref(bone_count))
                        .parse_next(input)?;
                Ok(ModelBone {
                    name,
                    transform,
                    parent,
                    children,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(bones)
    }
    fn mesh_part(input: &mut Stream) -> Result<MeshPart> {
        let (
            stream_offset,
            base_vertex,
            num_vertices,
            start_index,
            primitive_count,
            vertex_declaration_index,
        ) = (i32, i32, i32, i32, i32, i32).parse_next(input)?;
        let obj = object_any.parse_next(input)?;
        let effect = shared_resource_ref.parse_next(input)?;
        Ok(MeshPart {
            stream_offset,
            base_vertex,
            num_vertices,
            start_index,
            primitive_count,
            vertex_declaration_index,
            effect,
        })
    }
    fn mesh<'i>(bone_count: usize) -> impl Parser<Stream<'i>, Mesh, ContextError> {
        move |input: &mut Stream<'i>| {
            let name = object(string_object).parse_next(input)?;
            let parent_bone = bone_ref(bone_count).parse_next(input)?;
            let bounding_sphere_center = vec3.parse_next(input)?;
            let bounding_sphere_radius = f32.parse_next(input)?;
            let vertex_buffer = object(vertex_buffer).parse_next(input)?;
            let index_buffer = object(index_buffer).parse_next(input)?;
            let obj = object_any.parse_next(input)?;
            let parts: Vec<_> =
                length_repeat(i32.try_map(usize::try_from), mesh_part).parse_next(input)?;
            Ok(Mesh {
                name,
                parent_bone,
                bounding_sphere_center,
                bounding_sphere_radius,
                vertex_buffer,
                index_buffer,
                parts,
            })
        }
    }
    let bones = bones
        .context(StrContext::Expected(StrContextValue::Description(
            "model bones",
        )))
        .parse_next(input)?;
    let vertex_declarations: Vec<_> =
        length_repeat(i32.try_map(usize::try_from), object(vertex_decl))
            .context(StrContext::Expected(StrContextValue::Description(
                "model vertex declarations",
            )))
            .parse_next(input)?;
    let meshes: Vec<_> = length_repeat(i32.try_map(usize::try_from), mesh(bones.len()))
        .context(StrContext::Expected(StrContextValue::Description(
            "model meshes",
        )))
        .parse_next(input)?;
    let root_bone = bone_ref(bones.len()).parse_next(input)?;
    let tag = object_any
        .context(StrContext::Expected(StrContextValue::Description(
            "model tag",
        )))
        .parse_next(input)?;
    Ok(Model {
        bones,
        vertex_declarations,
        meshes,
        root_bone,
    })
}
#[derive(Debug)]
pub struct ModelBone {
    pub name: Option<NetString>,
    pub transform: Matrix,
    pub parent: Option<i32>,
    pub children: Vec<Option<i32>>,
}
#[derive(Debug)]
pub struct Mesh {
    pub name: Option<NetString>,
    pub parent_bone: Option<i32>,
    pub bounding_sphere_center: Vector3,
    pub bounding_sphere_radius: f32,
    pub vertex_buffer: Option<VertexBuffer>,
    pub index_buffer: Option<IndexBuffer>,
    pub parts: Vec<MeshPart>,
}
#[derive(Debug)]
pub struct MeshPart {
    pub stream_offset: i32,
    pub base_vertex: i32,
    pub num_vertices: i32,
    pub start_index: i32,
    pub primitive_count: i32,
    pub vertex_declaration_index: i32,
    // TODO: This should be any Effect, not DeferredEffect
    // pub effect: Option<SharedResourceReference<Effect>>,
    // HACK: Use common one since we don't support polymorphism yet
    // pub effect: Option<SharedResourceReference<crate::xnb_readers::magicka_effect::DeferredEffect>>,
    pub effect: Option<SharedResourceReference<AnyEffect>>,
}
/// Use [`crate::xnb::SharedResources::shared_resource_any`] to access this value.
#[derive(Debug)]
pub struct AnyEffect;

// https://github.com/MonoGame/MonoGame/blob/b5ead4c88dd114354f0d433fcb0ce635e7a05212/MonoGame.Framework/Content/ContentReaders/VertexDeclarationReader.cs#L9
pub(crate) fn vertex_decl(input: &mut Stream) -> Result<VertexDeclaration> {
    return length_repeat(i32.try_map(usize::try_from), vertex_element)
        .map(|elements| VertexDeclaration { elements })
        .parse_next(input);
    fn vertex_element(input: &mut Stream) -> Result<VertexElement> {
        seq!(VertexElement {
            stream: i16,
            offset: i16,
            element_format: u8.try_map(TryInto::try_into),
            element_method: u8.try_map(TryInto::try_into),
            element_usage: u8.try_map(TryInto::try_into),
            usage_index: u8,
        })
        .parse_next(input)
    }
}
#[derive(Debug)]
pub struct VertexDeclaration {
    pub elements: Vec<VertexElement>,
}
impl TypeReaderMeta for VertexDeclaration {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.VertexDeclarationReader";
    const VERSION: i32 = 0;
}

#[derive(Debug)]
pub struct VertexElement {
    pub stream: i16,
    pub offset: i16,
    pub element_format: VertexElementFormat,
    pub element_method: VertexElementMethod,
    pub element_usage: VertexElementUsage,
    pub usage_index: u8,
}
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum VertexElementFormat {
    Byte4 = 5,
    Color = 4,
    HalfVector2 = 15,
    HalfVector4 = 16,
    Normalized101010 = 14,
    NormalizedShort2 = 9,
    NormalizedShort4 = 10,
    Rg32 = 11,
    Rgba32 = 8,
    Rgba64 = 12,
    Short2 = 6,
    Short4 = 7,
    Single = 0,
    UInt101010 = 13,
    Vector2 = 1,
    Vector3 = 2,
    Vector4 = 3,
    Unused = 17,
}
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum VertexElementMethod {
    Default = 0,
    LookUp = 5,
    LookUpPresampled = 6,
    UV = 4,
}
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum VertexElementUsage {
    Binormal = 7,
    BlendIndices = 2,
    BlendWeight = 1,
    Color = 10,
    Depth = 12,
    Fog = 11,
    Normal = 3,
    PointSize = 4,
    Position = 0,
    Sample = 13,
    Tangent = 6,
    TessellateFactor = 8,
    TextureCoordinate = 5,
}

// https://github.com/MonoGame/MonoGame/blob/b5ead4c88dd114354f0d433fcb0ce635e7a05212/MonoGame.Framework/Content/ContentReaders/VertexBufferReader.cs
pub(crate) fn vertex_buffer(input: &mut Stream) -> Result<VertexBuffer> {
    let data = length_take(u32.try_map(usize::try_from)).parse_next(input)?;
    Ok(VertexBuffer {
        data: data.to_owned(),
    })
}
pub struct VertexBuffer {
    pub data: Vec<u8>,
}
impl std::fmt::Debug for VertexBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VertexBuffer")
            .field("data", &"...")
            .finish()
    }
}
impl TypeReaderMeta for VertexBuffer {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.VertexBufferReader";
    const VERSION: i32 = 0;
}

pub(crate) fn index_buffer(input: &mut Stream) -> Result<IndexBuffer> {
    let sixteenbit = bool.parse_next(input)?;
    let data = length_take(i32.try_map(usize::try_from)).parse_next(input)?;
    Ok(IndexBuffer {
        sixteenbit,
        data: data.to_owned(),
    })
}
pub struct IndexBuffer {
    pub sixteenbit: bool,
    pub data: Vec<u8>,
}
impl std::fmt::Debug for IndexBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexBuffer")
            .field("sixteenbit", &self.sixteenbit)
            .field("data", &"...")
            .finish()
    }
}
impl TypeReaderMeta for IndexBuffer {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.IndexBufferReader";
    const VERSION: i32 = 0;
}

pub use crate::xnb_readers::xna_tex::{Texture2d, TextureCube};
