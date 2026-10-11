//! Cover art: picks the cover for a track (embedded first, then image files next to the
//! music), de-duplicates by content hash, and keeps square JPEG thumbnails in a cache folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbImage;
use image::codecs::jpeg::JpegEncoder;

/// Thumbnail sizes kept on disk: lists and grids, and Now Playing.
pub const SIZES: [u32; 2] = [300, 1000];

/// File names (without extension) treated as album art when they sit next to the music.
const SIDECAR_NAMES: [&str; 6] = [
    "cover",
    "folder",
    "front",
    "album",
    "albumart",
    "albumartlarge",
];
const SIDECAR_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

static NEXT_TMP: AtomicU64 = AtomicU64::new(0);

pub fn hash(bytes: &[u8]) -> String {
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(bytes))
}

/// The cover image file in `dir`, if any (case-insensitive, preferring the order above).
pub fn find_sidecar(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<(usize, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let (Some(stem), Some(ext)) = (path.file_stem(), path.extension()) else {
            continue;
        };
        let stem = stem.to_string_lossy().to_lowercase();
        let ext = ext.to_string_lossy().to_lowercase();
        if !SIDECAR_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let rank = SIDECAR_NAMES
            .iter()
            .position(|n| *n == stem)
            .or_else(|| stem.starts_with("albumart").then_some(SIDECAR_NAMES.len()));
        if let Some(rank) = rank
            && best.as_ref().is_none_or(|(r, _)| rank < *r)
        {
            best = Some((rank, path));
        }
    }
    best.map(|(_, p)| p)
}

#[derive(Clone, Debug)]
pub struct ArtStore {
    dir: PathBuf,
}

impl ArtStore {
    pub fn new(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn path(&self, hash: &str, size: u32) -> PathBuf {
        self.dir.join(format!("{hash}-{size}.jpg"))
    }

    /// The thumbnail closest to `size`, when it exists.
    pub fn existing(&self, hash: &str, size: u32) -> Option<PathBuf> {
        let size = if size <= SIZES[0] { SIZES[0] } else { SIZES[1] };
        let path = self.path(hash, size);
        path.exists().then_some(path)
    }

    /// Stores `image_bytes` (any common format) and returns its hash. Thumbnails are only
    /// generated the first time a cover is seen.
    pub fn store(&self, image_bytes: &[u8]) -> Result<String, String> {
        let hash = hash(image_bytes);
        if SIZES.iter().all(|&s| self.path(&hash, s).exists()) {
            return Ok(hash);
        }
        let decoded = image::load_from_memory(image_bytes).map_err(|e| e.to_string())?;
        let rgb = decoded.to_rgb8();
        if rgb.width() == 0 || rgb.height() == 0 {
            return Err("empty image".into());
        }
        let mut resizer = Resizer::new();
        let options = ResizeOptions::new()
            .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3))
            .fit_into_destination(Some((0.5, 0.5)));
        for size in SIZES {
            // Never upscale small covers beyond their own size.
            let side = size.min(rgb.width().max(rgb.height()));
            let mut out = RgbImage::new(side, side);
            resizer
                .resize(&rgb, &mut out, &options)
                .map_err(|e| e.to_string())?;
            let mut jpeg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpeg, 88)
                .encode_image(&out)
                .map_err(|e| e.to_string())?;
            // Unique temp name: scanner threads may store the same album cover at once.
            let target = self.path(&hash, size);
            let tmp = self.dir.join(format!(
                "{hash}-{size}.{}.tmp",
                NEXT_TMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&tmp, &jpeg).map_err(|e| e.to_string())?;
            fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
        }
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        bytes
    }

    #[test]
    fn stores_square_thumbnails_once() {
        let dir = tempfile::tempdir().unwrap();
        let store = ArtStore::new(dir.path()).unwrap();
        let bytes = png(1600, 1200);
        let h = store.store(&bytes).unwrap();
        for size in SIZES {
            let img = image::open(store.path(&h, size)).unwrap();
            assert_eq!((img.width(), img.height()), (size, size));
        }
        assert_eq!(store.store(&bytes).unwrap(), h, "same bytes, same hash");
        assert!(
            store
                .existing(&h, 64)
                .unwrap()
                .ends_with(format!("{h}-300.jpg"))
        );
    }

    #[test]
    fn small_covers_are_not_upscaled() {
        let dir = tempfile::tempdir().unwrap();
        let store = ArtStore::new(dir.path()).unwrap();
        let h = store.store(&png(200, 200)).unwrap();
        assert_eq!(image::open(store.path(&h, 1000)).unwrap().width(), 200);
    }

    #[test]
    fn finds_sidecar_covers_by_preference() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AlbumArtSmall.jpg"), b"x").unwrap();
        fs::write(dir.path().join("Folder.JPG"), b"x").unwrap();
        fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        assert_eq!(
            find_sidecar(dir.path()).unwrap().file_name().unwrap(),
            "Folder.JPG"
        );
        fs::write(dir.path().join("cover.png"), b"x").unwrap();
        assert_eq!(
            find_sidecar(dir.path()).unwrap().file_name().unwrap(),
            "cover.png"
        );
    }
}
