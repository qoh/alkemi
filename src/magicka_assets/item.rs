// TODO: Load some of the models as dependencies

use bevy::{
    asset::{AssetLoader, LoadContext},
    prelude::*,
};
use remagic::xnb_readers::magicka_item::Item as MagickaItem;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Asset, Reflect, Debug)]
#[reflect(from_reflect = false)]
pub struct Item {
    #[reflect(ignore)]
    pub item: MagickaItem,
}

#[derive(Default, TypePath)]
pub(crate) struct ItemLoader;

impl AssetLoader for ItemLoader {
    type Asset = Item;

    type Settings = ItemLoaderSettings;

    type Error = ItemLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> std::result::Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let item_data = match remagic::parse_item(&bytes).map(|x| x.into_inner()) {
            Ok(o) => o.ok_or(ItemLoaderError::Null)?,
            Err(e) => {
                error!("failed to parse item .xnb: {}", e.inner());
                return Err(ItemLoaderError::Parse);
            }
        };

        let asset = Item { item: item_data };

        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["xnb"]
    }
}

/// An error when loading a character template image using [`CharacterTemplateLoader`].
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ItemLoaderError {
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
pub struct ItemLoaderSettings;
