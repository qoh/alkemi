pub mod character_template;
pub mod image;
pub mod item;
pub mod skinned_model;
pub mod visual_effect;

use bevy::{asset::AssetPath, prelude::*};
use std::{
    ffi::OsStr,
    fs, io,
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};
use typed_path::{PlatformPath, PlatformPathBuf};

pub fn plugin(app: &mut App) {
    app.register_asset_loader(image::MagickaTexture2dLoader);

    app.init_asset::<character_template::CharacterTemplate>();
    app.init_asset_loader::<character_template::CharacterTemplateLoader>();

    app.init_asset::<item::Item>();
    app.init_asset_loader::<item::ItemLoader>();

    app.init_asset::<visual_effect::VisualEffect>();
    app.init_asset_loader::<visual_effect::VisualEffectLoader>();
}

static CONTENT_DIR: OnceLock<PlatformPathBuf> = OnceLock::new();

pub fn content_root() -> &'static PlatformPath {
    const VAR: &str = "MAGICKA_CONTENT_DIR";

    CONTENT_DIR.get_or_init(|| {
        match std::env::var_os(VAR) {
            Some(dir) => {
                match PlatformPathBuf::try_from(PathBuf::from(dir)) {
                    Ok(dir) => dir,
                    Err(dir) => panic!(
                        "Magicka Content directory configured with {VAR} is not a valid path: {dir:?}"
                    ),
                }
            }
            // TODO: Automatically scan for game location
            None => panic!(
                "\n\nMagicka Content directory unknown. Set the {VAR} environment variable to the path to your Magicka install's Content directory.\n\n"
            ),
        }
    })
}

#[derive(Debug)]
pub struct ResolvedPath {
    /// The absolute path of the asset file
    pub resolved_path: PlatformPathBuf,
    /// The content path to resolve further assets referenced relative from the resolved asset
    pub transitive_content_path: PlatformPathBuf,
}

pub fn resolve_relative_path(
    from_content_path: &PlatformPath,
    relative_path: &str,
) -> ResolvedPath {
    let relative_path = typed_path::WindowsPathBuf::from(relative_path);
    let resolved_content_path = from_content_path.parent().unwrap().join(
        relative_path
            .with_platform_encoding_checked()
            .unwrap()
            .as_bytes(),
    );
    let mut absolute_path = crate::magicka_assets::content_root()
        .join_checked(&resolved_content_path)
        .unwrap();
    absolute_path.set_extension("xnb");
    if !matches!(std::fs::exists(absolute_path.as_ref() as &OsStr), Ok(true))
        && let Ok(found_path) = crate::magicka_assets::find_path_ignore_ascii_case(
            std::path::Path::new(absolute_path.as_ref() as &OsStr),
        )
    {
        absolute_path =
            typed_path::PlatformPath::new(found_path.as_os_str().as_encoded_bytes()).to_owned();
    }
    ResolvedPath {
        resolved_path: absolute_path,
        transitive_content_path: resolved_content_path,
    }
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

pub(crate) fn content_path_from_handle<A: Asset>(
    asset_handle: &Handle<A>,
) -> Option<&PlatformPath> {
    let asset_path = asset_handle.path()?;
    content_path_from_asset_path(asset_path)
}

pub(crate) fn content_path_from_asset_path<'p, 'r: 'p>(
    asset_path: &'r AssetPath<'p>,
) -> Option<&'p PlatformPath> {
    let asset_path = asset_path.path();
    let content_path = PlatformPath::new(asset_path.as_os_str().as_encoded_bytes())
        .strip_prefix(content_root())
        .ok()?;
    Some(content_path)
}
