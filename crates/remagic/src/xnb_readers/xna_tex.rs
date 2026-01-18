use crate::xnb::TypeReaderMeta;
use crate::xnb::{Stream, types::i32};
use num_enum::TryFromPrimitive;
use winnow::Parser as _;
use winnow::Result;
use winnow::binary::length_repeat;
use winnow::binary::length_take;
use winnow::error::StrContext;

pub struct Texture2d {
    pub format: SurfaceFormat,
    pub width: i32,
    pub height: i32,
    pub data_levels: Vec<Vec<u8>>,
}

impl TypeReaderMeta for Texture2d {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.Texture2DReader";

    const VERSION: i32 = 0;
}

impl std::fmt::Debug for Texture2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture2d")
            .field("format", &self.format)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("data_levels", &format!("[...; {}]", self.data_levels.len()))
            .finish()
    }
}
pub(crate) fn texture_2d(input: &mut Stream) -> Result<Texture2d> {
    let format = i32.try_map(TryInto::try_into).parse_next(input)?;
    let width = i32.parse_next(input)?;
    let height = i32.parse_next(input)?;
    let data_levels = length_repeat(
        i32.try_map(usize::try_from)
            .context(StrContext::Label("Texture2D layer count")),
        length_take(
            i32.try_map(usize::try_from)
                .context(StrContext::Label("Texture2D data layer length")),
        )
        .context(StrContext::Label(
            "Texture2D data layer (with length prefix)",
        ))
        .map(ToOwned::to_owned),
    )
    .context(StrContext::Label("Texture2D data"))
    .parse_next(input)?;
    Ok(Texture2d {
        format,
        width,
        height,
        data_levels,
    })
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
#[repr(i32)]
pub enum SurfaceFormat {
    Color = 1,
    Bgr32 = 2,
    Bgra1010102 = 3,
    Rgba32 = 4,
    Rgb32 = 5,
    Rgba1010102 = 6,
    Rg32 = 7,
    Rgba64 = 8,
    Bgr565 = 9,
    Bgra5551 = 10,
    Bgr555 = 11,
    Bgra4444 = 12,
    Bgr444 = 13,
    Bgra2338 = 14,
    Alpha8 = 15,
    Bgr233 = 16,
    Bgr24 = 17,
    NormalizedByte2 = 18,
    NormalizedByte4 = 19,
    NormalizedShort2 = 20,
    NormalizedShort4 = 21,
    Single = 22,
    Vector2 = 23,
    Vector4 = 24,
    HalfSingle = 25,
    HalfVector2 = 26,
    HalfVector4 = 27,
    Dxt1 = 28,
    Dxt2 = 29,
    Dxt3 = 30,
    Dxt4 = 31,
    Dxt5 = 32,
    Luminance8 = 33,
    Luminance16 = 34,
    LuminanceAlpha8 = 35,
    LuminanceAlpha16 = 36,
    Palette8 = 37,
    PaletteAlpha16 = 38,
    NormalizedLuminance16 = 39,
    NormalizedLuminance32 = 40,
    NormalizedAlpha1010102 = 41,
    NormalizedByte2Computed = 42,
    VideoYuYv = 43,
    VideoUyVy = 44,
    VideoGrGb = 45,
    VideoRgBg = 46,
    Multi2Bgra32 = 47,
    Depth24Stencil8 = 48,
    Depth24Stencil8Single = 49,
    Depth24Stencil4 = 50,
    Depth24 = 51,
    Depth32 = 52,
    Depth16 = 54,
    Depth15Stencil1 = 56,
    Unknown = -1,
}

#[derive(Debug)]
pub struct TextureCube;
