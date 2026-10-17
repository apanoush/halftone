//! Source loading and job construction for the GUI.

use std::path::Path;

use halftone_core::HalftoneSettings;
use halftone_core::mesh::{self, Mesh};
use halftone_core::pipeline::{Job, Source};
use halftone_core::raster::Raster;
use halftone_core::render3d::{self, Camera};

/// Default canvas for meshes, matching the pipeline.
pub const MESH_DEFAULT_SIZE: u32 = 1024;

/// The loaded input, kept in memory so settings changes never re-read files.
pub enum SourceKind {
    None,
    Image(Raster),
    Mesh(Mesh),
}

impl SourceKind {
    pub fn is_loaded(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn is_mesh(&self) -> bool {
        matches!(self, Self::Mesh(_))
    }

    /// Intrinsic pixel size, when known.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match self {
            Self::None => None,
            Self::Image(raster) => Some((raster.width, raster.height)),
            Self::Mesh(_) => None,
        }
    }
}

/// Decode an image file into a raster.
pub fn load_image(path: &Path) -> Result<Raster, String> {
    halftone_core::image_source::load(path).map_err(|e| e.to_string())
}

/// Load an OBJ file into a mesh.
pub fn load_mesh(path: &Path) -> Result<Mesh, String> {
    mesh::load_obj(path).map_err(|e| e.to_string())
}

/// Scale dimensions down so the longest side is at most `max`.
pub fn fit_within(width: u32, height: u32, max: u32) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let longest = width.max(height);
    if longest <= max {
        return (width, height);
    }
    let scale = max as f32 / longest as f32;
    (
        (width as f32 * scale).round().max(1.0) as u32,
        (height as f32 * scale).round().max(1.0) as u32,
    )
}

/// Export canvas for a mesh.
pub fn mesh_dimensions(width: Option<u32>, height: Option<u32>) -> (u32, u32) {
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w),
        (None, Some(h)) => (h, h),
        (None, None) => (MESH_DEFAULT_SIZE, MESH_DEFAULT_SIZE),
    }
}

/// Build a render job.
///
/// `preview_max` caps the canvas for interactive previews; `None` renders at
/// the export resolution.
pub fn build_job(
    source: &SourceKind,
    settings: &HalftoneSettings,
    camera: &Camera,
    preview_max: Option<u32>,
) -> Option<Job> {
    let mut settings = settings.clone();
    let source = match source {
        SourceKind::None => return None,
        SourceKind::Image(raster) => {
            if let Some(max) = preview_max {
                let (w, h) = fit_within(raster.width, raster.height, max);
                settings.width = Some(w);
                settings.height = Some(h);
            }
            Source::Raster(raster.clone())
        }
        SourceKind::Mesh(mesh) => {
            let (w, h) = mesh_dimensions(settings.width, settings.height);
            let (w, h) = match preview_max {
                Some(max) => fit_within(w, h, max),
                None => (w, h),
            };
            settings.width = Some(w);
            settings.height = Some(h);
            Source::Raster(render3d::rasterize(mesh, camera, w, h))
        }
    };
    Some(Job {
        source,
        settings,
        camera: camera.clone(),
    })
}
