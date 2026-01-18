use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use remagic::xnb_readers::xna_mesh::{
    IndexBuffer, VertexElement, VertexElementFormat, VertexElementMethod, VertexElementUsage,
};

pub(crate) fn init_mesh_from_xna_vertices(
    declaration: &remagic::xnb_readers::xna_mesh::VertexDeclaration,
    buffer: &remagic::xnb_readers::xna_mesh::VertexBuffer,
    vertex_stride: usize,
    vertex_count: usize,
    stream: i16,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    for element in &declaration.elements {
        if element.stream != stream {
            continue;
        }
        // https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dvertexelement9
        // https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3ddeclusage
        match element {
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector3,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::Position,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 3];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector3,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::Normal,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 3];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector3,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::Tangent,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 3];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    // Magicka tangents are 3D but Bevy expects 4D
                    // Unity documents tangents as 4D (x,y,z,w) where w is the orientation (CW/CCW?)
                    // https://docs.unity3d.com/6000.0/Documentation/Manual/mesh-vertex-data.html
                    // Randomly threw 1. in here to get it working
                    // TODO: Is this orientation right?
                    .map(|[x, y, z]| [x, y, z, 1.])
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector2,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::TextureCoordinate,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 2];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Color,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::Color,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [u8; 4];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .map(|[r, g, b, a]| {
                        [
                            r as f32 / 255.,
                            g as f32 / 255.,
                            b as f32 / 255.,
                            a as f32 / 255.,
                        ]
                    })
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector4,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::Color,
                usage_index: 0,
            } => {
                // Alpha channel = blend between layer 0 and layer 1
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 4];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .map(|[r, g, b, a]| [r, g, b, 1.0])
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Vector4,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::BlendWeight,
                usage_index: 0,
            } => {
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [f32; 4];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, values);
            }
            VertexElement {
                stream: _,
                offset,
                element_format: VertexElementFormat::Byte4,
                element_method: VertexElementMethod::Default,
                element_usage: VertexElementUsage::BlendIndices,
                usage_index: 0,
            } => {
                // https://learn.microsoft.com/en-us/windows/win32/direct3d9/using-indexed-vertex-blending#passing-matrix-indices-to-direct3d
                // "A packed DWORD contains index3, index2, index1, and index0, where index0 is located in the lowest byte of the DWORD."
                let offset: usize = (*offset).try_into().unwrap();
                let mut buffer = &buffer.data[..];
                let buffer_stride_count = buffer.len().checked_div(vertex_stride);
                assert_eq!(buffer_stride_count, Some(vertex_count));
                type Value = [u8; 4];
                assert!(offset <= vertex_stride - size_of::<Value>());
                let values: Vec<_> = (0..vertex_count)
                    .map(|_| {
                        let vertex_data = buffer.split_off(..vertex_stride).unwrap();
                        let value_bytes = vertex_data
                            .get(offset..(offset + size_of::<Value>()))
                            .unwrap();
                        *bytemuck::try_from_bytes::<Value>(value_bytes).unwrap()
                    })
                    .map(|[a, b, c, d]| -> [u16; 4] { [a.into(), b.into(), c.into(), d.into()] })
                    .collect();
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    VertexAttributeValues::Uint16x4(values),
                );
            }
            //         VertexElementUsage::Binormal => todo!(),
            //         VertexElementUsage::Depth => todo!(),
            //         VertexElementUsage::Fog => todo!(),
            //         VertexElementUsage::PointSize => todo!(),
            //         VertexElementUsage::Sample => todo!(),
            //         VertexElementUsage::TessellateFactor => todo!(),
            // Mesh::ATTRIBUTE_UV_), [f32; 2]
            _ => debug!("unhandled mesh {element:?}"),
        }
    }
    mesh
}

pub(crate) fn assign_mesh_indices(
    mesh: &mut Mesh,
    base_vertex_index: u32,
    start_index: usize,
    primitive_count: usize,
    index_buffer: &IndexBuffer,
) -> bool {
    let indices: Indices = {
        if index_buffer.sixteenbit {
            let base_vertex_index: u16 = base_vertex_index.try_into().unwrap();
            let indices: &[u16] = bytemuck::try_cast_slice(&index_buffer.data)
                .unwrap_or_else(|e| todo!("index buffer data bad: {e}"));
            let indices: Vec<_> = (0..primitive_count)
                .flat_map(|i| {
                    [
                        indices[start_index + i * 3] + base_vertex_index,
                        indices[start_index + i * 3 + 1] + base_vertex_index,
                        indices[start_index + i * 3 + 2] + base_vertex_index,
                    ]
                })
                .collect();
            Indices::U16(indices.to_vec())
        } else {
            let indices: &[u32] = bytemuck::try_cast_slice(&index_buffer.data)
                .unwrap_or_else(|e| todo!("index buffer data bad: {e}"));
            let indices: Vec<_> = (0..primitive_count)
                .flat_map(|i| {
                    [
                        indices[start_index + i * 3] + base_vertex_index,
                        indices[start_index + i * 3 + 1] + base_vertex_index,
                        indices[start_index + i * 3 + 2] + base_vertex_index,
                    ]
                })
                .collect();
            Indices::U32(indices.to_vec())
        }
    };
    mesh.insert_indices(indices);
    let invert_winding = true;
    if invert_winding {
        mesh.invert_winding()
            .unwrap_or_else(|e| warn!("Failed to invert index winding: {e}")); // PERF: it would be better if we did this with the material
    }
    // mesh.compute_normals();
    invert_winding
}
