//! Test-only fixtures: a small, real, file-backed photo library.
//!
//! The demo library generates 24 MP procedural scenes, which Nextest regenerates in every
//! process. The library/lifecycle tests only need an active detail photo whose file really
//! decodes, so this fixture writes a 64×48 gradient PNG and imports it as the only photo.

use std::path::PathBuf;

use lightcraft_catalog::{Op, Photo, Source};

use crate::{LightcraftApp, Services};

/// A temporary folder holding one small PNG; removes only its own folder when dropped.
///
/// Keep this guard alive until after the [`Headless`](crate::headless::Headless) built from
/// [`SyntheticLibrary::app`] is dropped: the renderer reads the photo file from disk.
pub(crate) struct SyntheticLibrary {
    dir: PathBuf,
}

impl SyntheticLibrary {
    /// A fresh folder for `tag`, with a deterministic 64×48 gradient `a.png`.
    pub(crate) fn new(tag: &str) -> SyntheticLibrary {
        let dir = std::env::temp_dir().join(format!("lc-ui-fixture-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (w, h) = (64usize, 48usize);
        let data: Vec<[u8; 4]> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                [(x * 3) as u8, (y * 4) as u8, (x + y) as u8, 255]
            })
            .collect();
        let img = lightcraft_raster::Rgba8 { width: w, height: h, data };
        let png = lightcraft_codecs::encode_png(&lightcraft_codecs::EncodeImage::rgba8(&img), &lightcraft_codecs::EncodeMeta::default()).unwrap();
        std::fs::write(dir.join("a.png"), png).unwrap();
        SyntheticLibrary { dir }
    }

    /// A session whose library holds only the fixture photo, active and selected, with the file
    /// system loaders installed so Detail really decodes the PNG.
    pub(crate) fn app(&self) -> LightcraftApp {
        let mut session = lightcraft_engine::Session::new().with_fs();
        let id = session.catalog.alloc_photo_id();
        let mut photo = Photo::new(
            id,
            Source::File { path: self.dir.join("a.png").to_string_lossy().into_owned() },
            "a.png",
            "PNG",
            64,
            48,
            "2026-01-01T10:00:00",
        );
        photo.captured = Some("2026-01-01T10:00:00".into());
        session.catalog.apply(Op::AddPhoto { photo: Box::new(photo) }).unwrap();
        session.selection = lightcraft_engine::Selection::single(id);
        LightcraftApp::new(session, Services { png: None, ..Default::default() })
    }
}

impl Drop for SyntheticLibrary {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
