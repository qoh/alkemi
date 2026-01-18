pub mod xnb;

pub mod xnb_readers {
    pub mod magicka_character;
    pub mod magicka_content;
    pub mod magicka_effect;
    pub mod magicka_mesh;
    pub mod skinning;
    pub mod xna_mesh;
    pub mod xna_tex;
}

use winnow::Parser;

use crate::{
    xnb::{XnbAsset, object},
    xnb_readers::xna_tex::Texture2d,
};

pub use crate::xnb::SharedResources;

pub use winnow::error::ContextError as InnerError;

pub fn parse_level<'i>(
    bytes: &'i [u8],
) -> Result<
    XnbAsset<Option<xnb_readers::magicka_content::Level>>,
    winnow::error::ParseError<xnb::Stream<'i>, winnow::error::ContextError>,
> {
    xnb::parse(
        bytes,
        xnb::object(xnb_readers::magicka_content::level_model),
    )
}

pub fn parse_texture_2d<'i>(
    bytes: &'i [u8],
) -> Result<
    XnbAsset<Option<Texture2d>>,
    winnow::error::ParseError<xnb::Stream<'i>, winnow::error::ContextError>,
> {
    xnb::parse(bytes, object(xnb_readers::xna_tex::texture_2d))
}

pub fn parse_character<'i>(
    bytes: &'i [u8],
) -> Result<
    XnbAsset<Option<xnb_readers::magicka_character::CharacterTemplate>>,
    winnow::error::ParseError<xnb::Stream<'i>, winnow::error::ContextError>,
> {
    xnb::parse(
        bytes,
        object(xnb_readers::magicka_character::character_template),
    )
}

pub fn parse_skinned_model<'i>(
    bytes: &'i [u8],
) -> Result<
    XnbAsset<Option<xnb_readers::skinning::SkinnedModel>>,
    winnow::error::ParseError<xnb::Stream<'i>, winnow::error::ContextError>,
> {
    xnb::parse(bytes, object(xnb_readers::skinning::skinned_model))
}
