//! Avatar storage: validation, normalization and atomic deployment.
//!
//! The canonical avatar location is `$HOME/.face`, the long-standing Unix/
//! freedesktop convention consumed by LightDM, SDDM and most desktop
//! environments. Files are always regular, owned by the target user, mode
//! `0644`, PNG-encoded at 256x256 pixels so any consumer (including a greeter
//! running as an unprivileged user) can render them without extra logic.
//!
//! Updates are atomic: the new image is written to a temporary file inside
//! the user's home directory, fsynced, permissioned, chowned and then renamed
//! over `.face`. An interrupted update therefore never destroys the previous
//! avatar.

use std::fs;
use std::io::{Cursor, Write};
use std::os::unix::fs::{chown, MetadataExt, PermissionsExt};
use std::path::Path;

use image::ImageFormat;

use crate::error::{Error, Result};

/// Canonical avatar file name inside the user's home directory.
pub const AVATAR_FILENAME: &str = ".face";

/// Maximum accepted size for a source avatar file (20 MiB).
pub const MAX_SOURCE_FILE_BYTES: u64 = 20 * 1024 * 1024;

/// Edge length of the normalized output image.
pub const OUTPUT_SIZE: u32 = 256;

/// Minimum accepted input dimension.
pub const MIN_INPUT_DIMENSION: u32 = 16;

/// Maximum accepted input dimension (guards against decompression bombs).
pub const MAX_INPUT_DIMENSION: u32 = 16384;

/// Hard budget for decoded pixel buffers (128 MiB).
const MAX_DECODED_ALLOC_BYTES: u64 = 128 * 1024 * 1024;

/// Returns the canonical avatar path for a given home directory.
#[must_use]
pub fn avatar_path(home: &Path) -> std::path::PathBuf {
    home.join(AVATAR_FILENAME)
}

/// Returns the avatar path if, and only if, it currently exists as a regular
/// file. Symlinks are deliberately not followed.
#[must_use]
pub fn find_avatar(home: &Path) -> Option<std::path::PathBuf> {
    let path = avatar_path(home);
    let is_regular = path
        .symlink_metadata()
        .map(|meta| meta.is_file())
        .unwrap_or(false);
    is_regular.then_some(path)
}

/// Validates `source` and deploys it atomically as the user's avatar.
///
/// The source file is fully copied/normalized; the original is never touched.
/// `owner` must match the uid/gid of the account that owns `home`.
pub fn set_avatar(home: &Path, source: &Path, owner: (u32, u32)) -> Result<std::path::PathBuf> {
    let meta = fs::metadata(source).map_err(|err| {
        Error::InvalidImage(format!(
            "cannot read source image '{}': {err}",
            source.display()
        ))
    })?;
    if meta.len() > MAX_SOURCE_FILE_BYTES {
        return Err(Error::ImageTooLarge {
            actual: meta.len(),
            max: MAX_SOURCE_FILE_BYTES,
        });
    }
    let bytes = fs::read(source)?;
    if bytes.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err(Error::ImageTooLarge {
            actual: bytes.len() as u64,
            max: MAX_SOURCE_FILE_BYTES,
        });
    }
    let png = normalize_png(&bytes)?;
    deploy_png(home, &png, owner)
}

/// Removes the avatar of a user whose home directory is `home`.
pub fn remove_avatar(home: &Path) -> Result<()> {
    let dest = avatar_path(home);
    match fs::symlink_metadata(&dest) {
        Ok(meta) => {
            if meta.is_dir() {
                return Err(Error::system(format!(
                    "'{}' is a directory; refusing to remove it",
                    dest.display()
                )));
            }
            fs::remove_file(&dest)?;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::AvatarNotFound(AVATAR_FILENAME.to_string()));
        }
        Err(err) => return Err(err.into()),
    }
    sync_directory(home);
    Ok(())
}

/// Sniffs the real format, decodes under strict limits and re-encodes as a
/// square 256x256 PNG using a center crop. The format is detected from the
/// byte stream itself; file extensions are never trusted.
fn normalize_png(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes));
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_INPUT_DIMENSION);
    limits.max_image_height = Some(MAX_INPUT_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_ALLOC_BYTES);
    reader.limits(limits);

    let reader = reader.with_guessed_format()?;
    let format = reader
        .format()
        .ok_or_else(|| Error::UnsupportedImageFormat("unrecognized".to_string()))?;
    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP => {}
        other => {
            return Err(Error::UnsupportedImageFormat(format!("{other:?}")));
        }
    }

    let decoded = reader.decode()?;
    let (width, height) = (decoded.width(), decoded.height());
    if width < MIN_INPUT_DIMENSION || height < MIN_INPUT_DIMENSION {
        return Err(Error::InvalidImage(format!(
            "dimensions {width}x{height} below minimum {MIN_INPUT_DIMENSION}x{MIN_INPUT_DIMENSION}"
        )));
    }

    let squared = decoded.resize_to_fill(
        OUTPUT_SIZE,
        OUTPUT_SIZE,
        image::imageops::FilterType::Lanczos3,
    );
    let mut png = Vec::with_capacity(64 * 1024);
    squared.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

fn deploy_png(home: &Path, png: &[u8], owner: (u32, u32)) -> Result<std::path::PathBuf> {
    let home_meta = fs::symlink_metadata(home).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::HomeDirUnusable(home.display().to_string())
        } else {
            err.into()
        }
    })?;
    if !home_meta.is_dir() {
        return Err(Error::HomeDirUnusable(home.display().to_string()));
    }
    if home_meta.uid() != owner.0 {
        return Err(Error::PermissionDenied(format!(
            "home directory '{}' is owned by uid {}, expected uid {}",
            home.display(),
            home_meta.uid(),
            owner.0
        )));
    }

    let mut temp = tempfile::Builder::new()
        .prefix(".face-tmp-")
        .suffix(".png")
        .tempfile_in(home)?;
    temp.write_all(png)?;
    temp.as_file().sync_all()?;
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o644))?;
    chown(temp.path(), Some(owner.0), Some(owner.1))?;

    let dest = avatar_path(home);
    // rename(2): replaces any existing entry (including symlinks) atomically.
    if let Err(err) = temp.persist(&dest) {
        return Err(Error::Io(err.error));
    }
    sync_directory(home);
    Ok(dest)
}

fn sync_directory(dir: &Path) {
    // Best effort: durability hint, not a correctness requirement here.
    let _ = fs::File::open(dir).and_then(|file| file.sync_all());
}
