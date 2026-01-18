#![allow(non_snake_case)]

/// PolygonHead
use crate::{
    xnb::{
        Stream, TypeReaderMeta, object,
        types::{ExternalReference, Vector2, Vector3, bool, external_ref, f32, i32, vec2, vec3},
    },
    xnb_readers::magicka_effect::{Effect, effect},
    xnb_readers::xna_mesh::{
        IndexBuffer, Texture2d, TextureCube, VertexBuffer, VertexDeclaration, index_buffer,
        vertex_buffer, vertex_decl,
    },
};
use winnow::{
    Parser as _, Result,
    binary::length_repeat,
    combinator::{alt, cond, seq},
    error::StrContext,
};

/// BiTreeModel
#[derive(Debug)]
pub struct BiTreeModel {
    pub bitrees: Vec<BiTree>,
}
pub(crate) fn bitree_model(input: &mut Stream) -> Result<BiTreeModel> {
    let bitrees = length_repeat(i32.try_map(usize::try_from), bitree).parse_next(input)?;
    Ok(BiTreeModel { bitrees })
}
impl TypeReaderMeta for BiTreeModel {
    const NAME: &'static str = "PolygonHead.Pipeline.BiTreeModelReader, PolygonHead";
    const VERSION: i32 = 0;
}

/// BiTreeRootNode
#[derive(Debug)]
pub struct BiTree {
    pub visible: bool,
    pub cast_shadows: bool,
    pub sway: f32,
    pub entity_influence: f32,
    pub ground_level: f32,
    pub vertex_count: usize,
    pub vertex_stride: usize,
    pub vertex_declaration: Option<VertexDeclaration>,
    pub vertex_buffer: Option<VertexBuffer>,
    pub index_buffer: Option<IndexBuffer>,
    pub effect: Option<Effect>,
    pub node: Box<BiTreeNode>,
}
fn bitree<'a>(input: &mut Stream<'a>) -> Result<BiTree> {
    // seq!(BiTree {
    //     visible: bool,
    //     cast_shadows: bool,
    //     sway: f32,
    //     entity_influence: f32,
    //     ground_level: f32,
    //     vertex_count: i32.try_map(usize::try_from),
    //     vertex_stride: i32.try_map(usize::try_from),

    //     vertex_declaration: object(vertex_decl),
    //     vertex_buffer: object(vertex_buffer),
    //     index_buffer: object(index_buffer),
    //     effect: object(deferred_effect),
    //     node: bitree_node,
    // })
    // .parse_next(input);
    #[derive(Debug)]
    struct Fields {
        visible: bool,
        cast_shadows: bool,
        sway: f32,
        entity_influence: f32,
        ground_level: f32,
        vertex_count: usize,
        vertex_stride: usize,
    }
    let fields = seq!(Fields {
        visible: bool,
        cast_shadows: bool,
        sway: f32,
        entity_influence: f32,
        ground_level: f32,
        vertex_count: i32.try_map(usize::try_from),
        vertex_stride: i32.try_map(usize::try_from),
    })
    .parse_next(input)?;
    let Fields {
        visible,
        cast_shadows,
        sway,
        entity_influence,
        ground_level,
        vertex_count,
        vertex_stride,
    } = fields;
    let vertex_declaration = object(vertex_decl).parse_next(input)?;
    let vertex_buffer = object(vertex_buffer).parse_next(input)?;
    let index_buffer = object(index_buffer).parse_next(input)?;
    let effect = effect.parse_next(input)?;
    let node = bitree_node.parse_next(input)?;
    Ok(BiTree {
        visible,
        cast_shadows,
        sway,
        entity_influence,
        ground_level,
        vertex_count,
        vertex_stride,
        vertex_declaration,
        vertex_buffer,
        index_buffer,
        effect,
        node,
    })
}

#[derive(Debug)]
pub struct BiTreeNode {
    pub primitive_count: i32,
    pub start_index: i32,
    pub bounding_box: (Vector3, Vector3),
    pub child_a: Option<Box<BiTreeNode>>,
    pub child_b: Option<Box<BiTreeNode>>,
}
fn bitree_node(input: &mut Stream) -> Result<Box<BiTreeNode>> {
    seq!(BiTreeNode {
        primitive_count: i32,
        start_index: i32,
        bounding_box: (vec3, vec3),
        child_a: bool.flat_map(|has| cond(has, bitree_node)),
        child_b: bool.flat_map(|has| cond(has, bitree_node)),
    })
    .map(Box::new)
    .parse_next(input)
}

pub fn write_to_obj(model: BiTreeModel) {
    use crate::xnb::types::*;
    use crate::xnb_readers::xna_mesh::{VertexElementFormat, VertexElementUsage};
    use std::io::Write as _;
    use winnow::binary::{le_u16, le_u32};
    use winnow::combinator::{preceded, repeat};
    use winnow::error::ContextError;
    use winnow::token::take;

    let mut out = std::fs::File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open("/tmp/magicka_havindr.obj")
        .unwrap();
    let mut out_mtl = std::fs::File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open("/tmp/magicka_havindr.mtl")
        .unwrap();
    for (bti, bitree) in model.bitrees.iter().enumerate() {
        use std::fs::File;

        use winnow::Bytes;

        use crate::xnb_readers::magicka_mesh::BiTreeNode;

        {
            use crate::xnb_readers::magicka_mesh::Effect;
            // Write the material
            writeln!(out_mtl, "newmtl level-model-bitree{}", bti).unwrap();
            match &bitree.effect {
                Some(Effect::Deferred(effect)) => {
                    if effect.Alpha < 1. {
                        writeln!(out_mtl, "  d {}", effect.Alpha).unwrap();
                    }
                    writeln!(
                        out_mtl,
                        "  Kd {} {} {}",
                        effect.Layer0.DiffuseColor0.0,
                        effect.Layer0.DiffuseColor0.1,
                        effect.Layer0.DiffuseColor0.2
                    )
                    .unwrap();
                    writeln!(out_mtl, "  Ns {}", effect.Layer0.SpecPower0).unwrap();
                    writeln!(out_mtl, "  Ke {}", effect.Layer0.EmissiveAmount0).unwrap();
                    writeln!(out_mtl, "  Pm {}", effect.Layer0.Reflectiveness0).unwrap();
                    if let Some(layer1) = effect.Layer1.as_ref() { /* todo */ }
                }
                _ => todo!(),
            }
        }

        writeln!(out, "o level-model-bitree{}", bti).unwrap();
        writeln!(out, "  usemtl level-model-bitree{}", bti).unwrap();

        let vertex_buffer = &bitree.vertex_buffer.as_ref().unwrap().data;
        let input = winnow::Bytes::new(&vertex_buffer);
        let position_element = bitree
            .vertex_declaration
            .as_ref()
            .unwrap()
            .elements
            .iter()
            .find(|e| e.element_usage == VertexElementUsage::Position)
            .unwrap();
        assert_eq!(
            position_element.element_format,
            VertexElementFormat::Vector3
        );
        let parse_vertex = preceded(
            take::<_, _, ContextError>(usize::try_from(position_element.offset).unwrap()),
            vec3,
        );
        let mut parse_vertices = repeat(
            bitree.vertex_count,
            take(bitree.vertex_stride).and_then(parse_vertex),
        );
        let vertices: Vec<Vector3> = parse_vertices.parse(input).unwrap();
        for vert in &vertices {
            writeln!(out, "  v {} {} {}", vert.0, vert.1, vert.2).unwrap();
        }

        let index_buffer = &bitree.index_buffer.as_ref().unwrap();
        let indices: Vec<u32> = if index_buffer.sixteenbit {
            repeat(0.., le_u16::<_, ContextError>.map(u32::from))
                .parse(Bytes::new(&index_buffer.data))
                .unwrap()
        } else {
            repeat(0.., le_u32::<_, ContextError>)
                .parse(Bytes::new(&index_buffer.data))
                .unwrap()
        };

        fn explore_node(vcount: isize, indices: &[u32], file: &mut File, node: &BiTreeNode) {
            for i in 0..(node.primitive_count as usize) {
                let vert1 = indices[(node.start_index as usize) + i * 3];
                let vert2 = indices[(node.start_index as usize) + i * 3 + 1];
                let vert3 = indices[(node.start_index as usize) + i * 3 + 2];
                writeln!(
                    file,
                    "  f {} {} {}",
                    -vcount + (vert1 as isize),
                    -vcount + (vert2 as isize),
                    -vcount + (vert3 as isize),
                )
                .unwrap();
            }
            if let Some(a) = node.child_a.as_ref().map(AsRef::as_ref) {
                explore_node(vcount, indices, file, a);
            }
            if let Some(b) = node.child_b.as_ref().map(AsRef::as_ref) {
                explore_node(vcount, indices, file, b);
            }
        }
        explore_node(vertices.len() as isize, &indices, &mut out, &bitree.node);
    }
}
