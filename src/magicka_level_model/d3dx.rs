// Based on https://github.com/microsoft/DirectXMesh/blob/main/Utilities/FlexibleVertexFormat.h

/*
constexpr uint8_t g_declTypeSizes[] =
{
    4,  // D3DDECLTYPE_FLOAT1
    8,  // D3DDECLTYPE_FLOAT2
    12, // D3DDECLTYPE_FLOAT3
    16, // D3DDECLTYPE_FLOAT4
    4,  // D3DDECLTYPE_D3DCOLOR
    4,  // D3DDECLTYPE_UBYTE4
    4,  // D3DDECLTYPE_SHORT2
    8,  // D3DDECLTYPE_SHORT4
    4,  // D3DDECLTYPE_UBYTE4N
    4,  // D3DDECLTYPE_SHORT2N
    8,  // D3DDECLTYPE_SHORT4N
    4,  // D3DDECLTYPE_USHORT2N
    8,  // D3DDECLTYPE_USHORT4N
    4,  // D3DDECLTYPE_UDEC3
    4,  // D3DDECLTYPE_DEC3N
    4,  // D3DDECLTYPE_FLOAT16_2
    8,  // D3DDECLTYPE_FLOAT16_4
};
*/

/*
public enum VertexElementFormat : byte
    {
    Single = 0,
    Vector2 = 1,
    Vector3 = 2,
    Vector4 = 3,
        Color = 4,
        Byte4 = 5,
        Short2 = 6,
        Short4 = 7,
        Rgba32 = 8,
        NormalizedShort2 = 9,
        NormalizedShort4 = 10,
        Rg32 = 11,
        Rgba64 = 12,
        UInt101010 = 13,
        Normalized101010 = 14,
        HalfVector2 = 15,
        HalfVector4 = 16,
        Unused = 17
    }
 */

use remagic::xnb_readers::xna_mesh::{VertexElement, VertexElementMethod};

pub fn vertex_size(elements: &[VertexElement], stream: i16) -> usize {
    elements
        .iter()
        // only look at items of this stream and vertex elements actually in the data stream (not generated)
        // UV is phantom data.
        .filter(|e| e.stream == stream)
        .filter(|e| e.element_method != VertexElementMethod::UV)
        .map(|e| {
            use remagic::xnb_readers::xna_mesh::VertexElementFormat::*;
            let slot_size = match e.element_format {
                Single => 4,
                Vector2 => 8,
                Vector3 => 12,
                Vector4 => 16,
                Color => 4,
                Byte4 => 4,
                Short2 => 4,
                Short4 => 8,
                Rgba32 => 4,
                NormalizedShort2 => 4,
                NormalizedShort4 => 8,
                Rg32 => 4,
                Rgba64 => 8,
                UInt101010 => 4,
                Normalized101010 => 4,
                HalfVector2 => 4,
                HalfVector4 => 8,
                Unused => panic!("invalid element format"),
            };
            usize::try_from(e.offset).unwrap() + slot_size
        })
        .max()
        .unwrap_or(0)
}
