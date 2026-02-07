use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages},
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::Extent3d,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Default)]
pub(crate) struct MagickaTexture2dLoader;

impl AssetLoader for MagickaTexture2dLoader {
    type Asset = Image;

    type Settings = MagickaTexture2dLoaderSettings;

    type Error = MagickaTexture2dLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> std::result::Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let texture_2d = match remagic::parse_texture_2d(&bytes).map(|x| x.into_inner()) {
            Ok(o) => o.ok_or(MagickaTexture2dLoaderError::Null)?,
            Err(e) => {
                error!("failed to parse texture .xnb: {}", e.inner());
                return Err(MagickaTexture2dLoaderError::Parse);
            }
        };

        let texture_format = match texture_2d.format {
            remagic::xnb_readers::xna_tex::SurfaceFormat::Dxt1 => {
                if settings.is_srgb {
                    bevy::render::render_resource::TextureFormat::Bc1RgbaUnormSrgb
                } else {
                    bevy::render::render_resource::TextureFormat::Bc1RgbaUnorm
                }
            }
            remagic::xnb_readers::xna_tex::SurfaceFormat::Dxt5 => {
                if settings.is_srgb {
                    bevy::render::render_resource::TextureFormat::Bc3RgbaUnormSrgb
                } else {
                    bevy::render::render_resource::TextureFormat::Bc3RgbaUnorm
                }
            }
            remagic::xnb_readers::xna_tex::SurfaceFormat::Color => {
                let format = bevy::render::render_resource::TextureFormat::Bgra8Unorm;
                if settings.is_srgb {
                    format.add_srgb_suffix()
                } else {
                    format.remove_srgb_suffix()
                }
            }
            _ => unimplemented!("texture format {:?}", texture_2d.format),
        };

        if texture_2d.data_levels.len() != 1 {
            debug!(
                "unhandled image multi-levels: {}",
                texture_2d.data_levels.len()
            );
        }

        let mut image = Image::new(
            Extent3d {
                width: texture_2d.width.try_into().unwrap(),
                height: texture_2d.height.try_into().unwrap(),
                // depth_or_array_layers: texture_2d.data_levels.len().try_into().unwrap(),
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            texture_2d.data_levels.first().unwrap().clone(),
            texture_format,
            settings.asset_usage,
        );
        image.sampler = settings.sampler.clone();

        Ok(image)
    }

    fn extensions(&self) -> &[&str] {
        &["xnb"]
    }
}

/// An error when loading an image using [`MagickaTexture2dLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MagickaTexture2dLoaderError {
    /// An error occurred while trying to load the image bytes.
    #[error("Failed to load image bytes: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to decode the image bytes.
    #[error("No texture in file: the Texture2D object is null.")]
    Null,
    #[error("Could not load texture file")] // : {0}
    Parse, //(#[from] remagic::InnerError),
}

/// Settings for loading an [`Image`] using a [`MagickaTexture2dLoader`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MagickaTexture2dLoaderSettings {
    /// Specifies whether image data is linear
    /// or in sRGB space when this is not determined by
    /// the image format.
    pub is_srgb: bool,
    /// [`ImageSampler`] to use when rendering - this does
    /// not affect the loading of the image data.
    pub sampler: ImageSampler,
    /// Where the asset will be used - see the docs on
    /// [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
}

impl Default for MagickaTexture2dLoaderSettings {
    fn default() -> Self {
        Self {
            is_srgb: true,
            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..default()
            }),
            asset_usage: RenderAssetUsages::default(),
        }
    }
}
