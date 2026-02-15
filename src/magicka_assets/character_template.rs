// TODO: Load some of the models as dependencies - at least the skeleton

use bevy::{
    asset::{AssetLoader, LoadContext},
    prelude::*,
};
use remagic::xnb_readers::magicka_character::CharacterTemplate as MagickaCharacterTemplate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use typed_path::PlatformPathBuf;

#[derive(Asset, Reflect, Debug)]
#[reflect(from_reflect = false)]
pub struct CharacterTemplate {
    #[reflect(ignore)]
    pub template: MagickaCharacterTemplate,
    #[reflect(ignore)]
    pub content_path: PlatformPathBuf,
}

#[derive(Default)]
pub(crate) struct CharacterTemplateLoader;

impl AssetLoader for CharacterTemplateLoader {
    type Asset = CharacterTemplate;

    type Settings = CharacterTemplateLoaderSettings;

    type Error = CharacterTemplateLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> std::result::Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let _template = match remagic::parse_character(&bytes).map(|x| x.into_inner()) {
            Ok(o) => o.ok_or(CharacterTemplateLoaderError::Null)?,
            Err(e) => {
                error!("failed to parse character template .xnb: {}", e.inner());
                return Err(CharacterTemplateLoaderError::Parse);
            }
        };

        //let asset = CharacterTemplate { template };
        //Ok(asset)
        todo!()
    }

    fn extensions(&self) -> &[&str] {
        &["xnb"]
    }
}

/// An error when loading a character template image using [`CharacterTemplateLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CharacterTemplateLoaderError {
    /// An error occurred while trying to load the file bytes.
    #[error("Failed to load file bytes: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to decode the file bytes.
    #[error("Could not parse file")] // : {0}
    Parse, //(#[from] remagic::InnerError),
    #[error("No object in file (null)")]
    Null,
}

/// Settings for loading a [`CharacterTemplate`] using [`CharacterTemplateLoader`].
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CharacterTemplateLoaderSettings;
