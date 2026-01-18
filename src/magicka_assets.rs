pub mod character_template;
pub mod image;
pub mod skinned_model;

use bevy::prelude::*;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};
use typed_path::PlatformPath;

pub fn plugin(app: &mut App) {
    app.register_asset_loader(image::MagickaTexture2dLoader);

    app.init_asset::<character_template::CharacterTemplate>();
    app.init_asset_loader::<character_template::CharacterTemplateLoader>();
}

pub fn content_root() -> &'static PlatformPath {
    // TODO: Configurable game path
    PlatformPath::new("/data/SteamLibrary/steamapps/common/Magicka/Content/")
}

pub fn read_ignore_path_ascii_case(path: impl AsRef<Path>) -> Result<Vec<u8>, io::Error> {
    let path = path.as_ref();
    match fs::read(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let fixed_path = find_path_ignore_ascii_case(path)?;
            fs::read(fixed_path)
        }
        r => r,
    }
}

pub fn find_path_ignore_ascii_case(path: &Path) -> Result<PathBuf, io::Error> {
    // Find a parent path that exists
    let mut comps = path.components();
    let mut removed = Vec::new();
    let parent = loop {
        let candidate = comps.as_path();
        if candidate.try_exists()? {
            break Some(candidate);
        }
        if let Some(comp @ (Component::Normal(_) | Component::CurDir | Component::ParentDir)) =
            comps.next_back()
        {
            removed.push(comp);
        } else {
            break None;
        }
    }
    .ok_or(io::Error::from(io::ErrorKind::NotFound))?;

    if removed.is_empty() {
        return Ok(parent.to_owned()); // XXX: If we returned Cow instead then this to_owned would be unnecessary
    }

    // Now search for each component case insensitively
    let mut path = parent.to_owned();

    'comps: for comp in removed.into_iter().rev() {
        match comp {
            Component::CurDir | Component::ParentDir => path.push(comp.as_os_str()),
            Component::Normal(segment) => {
                // Try the segment directly first to skip the search
                path.push(segment);
                if path.try_exists().unwrap_or(false) {
                    continue 'comps;
                }
                // It didn't exist, so search case insensitively
                path.pop();
                for entry in fs::read_dir(&path)? {
                    let entry = entry?;
                    if entry.file_name().eq_ignore_ascii_case(segment) {
                        path.push(entry.file_name());
                        continue 'comps;
                    }
                }
                // It still didn't exist, give up
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            Component::Prefix(_) | Component::RootDir => unreachable!(),
        }
    }

    Ok(path)
}
