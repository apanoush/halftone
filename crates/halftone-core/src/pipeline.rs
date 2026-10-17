//! End-to-end rendering pipeline shared by the CLI and the GUI.

use std::path::PathBuf;

use crate::color::Rgba;
use crate::coverage::CoverageMap;
use crate::error::Result;
use crate::halftone::{self, PatternOptions};
use crate::image_source;
use crate::mesh;
use crate::raster::Raster;
use crate::render3d::{self, Camera};
use crate::settings::{ColorMode, HalftoneSettings};
use crate::shape::Layer;
use crate::svg::SvgDoc;
use crate::tone::Tone;

/// Where a render takes its pixels from.
#[derive(Clone, Debug)]
pub enum Source {
    /// An image file decoded by the `image` crate.
    Image(PathBuf),
    /// An OBJ mesh rendered with the software rasterizer.
    Mesh(PathBuf),
    /// An in-memory raster, useful for previews and tests.
    Raster(Raster),
}

/// A complete render request.
#[derive(Clone, Debug)]
pub struct Job {
    pub source: Source,
    pub settings: HalftoneSettings,
    pub camera: Camera,
}

impl Job {
    pub fn image(path: impl Into<PathBuf>, settings: HalftoneSettings) -> Self {
        Self {
            source: Source::Image(path.into()),
            settings,
            camera: Camera::default(),
        }
    }

    pub fn mesh(path: impl Into<PathBuf>, settings: HalftoneSettings) -> Self {
        Self {
            source: Source::Mesh(path.into()),
            settings,
            camera: Camera::default(),
        }
    }
}

/// The result of a render.
#[derive(Clone, Debug)]
pub struct RenderResult {
    pub svg: String,
    pub width: u32,
    pub height: u32,
    pub shapes: usize,
}

impl RenderResult {
    pub fn is_empty(&self) -> bool {
        self.shapes == 0
    }
}

/// Default canvas for meshes, which have no intrinsic pixel size.
const MESH_DEFAULT_SIZE: u32 = 1024;

/// Render a job into an SVG document.
pub fn render(job: &Job) -> Result<RenderResult> {
    job.settings.validate()?;
    let (raster, dims) = source_raster(job)?;

    let tone = Tone::from_settings(&job.settings);
    let layers = build_layers(&raster, &job.settings, &tone);

    let mut doc = SvgDoc::new(dims.0, dims.1);
    doc.background = job.settings.background;
    doc.layers = layers;

    Ok(RenderResult {
        svg: doc.to_svg(),
        width: dims.0,
        height: dims.1,
        shapes: doc.shape_count(),
    })
}

fn source_raster(job: &Job) -> Result<(Raster, (u32, u32))> {
    match &job.source {
        Source::Image(path) => {
            image_source::load_for_canvas(path, job.settings.width, job.settings.height)
        }
        Source::Mesh(path) => {
            let mesh = mesh::load_obj(path)?;
            let dims = mesh_canvas(job.settings.width, job.settings.height);
            let raster = render3d::rasterize(&mesh, &job.camera, dims.0, dims.1);
            Ok((raster, dims))
        }
        Source::Raster(raster) => {
            let dims = image_source::resolve_dimensions(
                raster.width,
                raster.height,
                job.settings.width,
                job.settings.height,
            );
            Ok((image_source::resize(raster, dims.0, dims.1), dims))
        }
    }
}

fn mesh_canvas(width: Option<u32>, height: Option<u32>) -> (u32, u32) {
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w),
        (None, Some(h)) => (h, h),
        (None, None) => (MESH_DEFAULT_SIZE, MESH_DEFAULT_SIZE),
    }
}

/// Build the ink layers for a raster.
pub fn build_layers(raster: &Raster, settings: &HalftoneSettings, tone: &Tone) -> Vec<Layer> {
    match settings.color_mode {
        ColorMode::Mono => {
            let map = CoverageMap::mono(raster, tone);
            let shapes = halftone::render(
                settings.style,
                &map,
                PatternOptions {
                    cell: settings.cell_size,
                    angle: settings.angle,
                    shape: settings.dot_shape,
                    max_scale: settings.max_dot_scale,
                    bayer_size: settings.bayer_size,
                },
            );
            vec![Layer::new("ink", settings.foreground, shapes)]
        }
        ColorMode::Cmyk => {
            let maps = CoverageMap::cmyk(raster, tone);
            // Classic process angles: cyan 15, magenta 75, yellow 0, black 45.
            let offsets = [15.0f32, 75.0, 0.0, 45.0];
            let names = ["cyan", "magenta", "yellow", "black"];
            let colors = [
                Rgba::rgb(0, 174, 239),
                Rgba::rgb(236, 0, 140),
                Rgba::rgb(255, 242, 0),
                Rgba::BLACK,
            ];
            maps.iter()
                .enumerate()
                .map(|(index, map)| {
                    let shapes = halftone::render(
                        settings.style,
                        map,
                        PatternOptions {
                            cell: settings.cell_size,
                            angle: settings.angle + offsets[index],
                            shape: settings.dot_shape,
                            max_scale: settings.max_dot_scale,
                            bayer_size: settings.bayer_size,
                        },
                    );
                    Layer::new(names[index], colors[index], shapes)
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(value: u8, size: u32) -> Raster {
        let mut raster = Raster::new(size, size);
        for y in 0..size {
            for x in 0..size {
                raster.set_pixel(x, y, Rgba::rgb(value, value, value));
            }
        }
        raster
    }

    fn job(raster: Raster) -> Job {
        Job {
            source: Source::Raster(raster),
            settings: HalftoneSettings {
                width: Some(8),
                height: Some(8),
                cell_size: 4.0,
                angle: 0.0,
                ..HalftoneSettings::default()
            },
            camera: Camera::default(),
        }
    }

    #[test]
    fn black_image_gives_dots() {
        let result = render(&job(solid(0, 8))).unwrap();
        assert_eq!(result.shapes, 4);
        assert!(result.svg.contains("<circle"));
        assert!(result.svg.contains("viewBox=\"0 0 8 8\""));
    }

    #[test]
    fn white_image_gives_empty_svg() {
        let result = render(&job(solid(255, 8))).unwrap();
        assert!(result.is_empty());
        assert!(!result.svg.contains("<circle"));
    }

    #[test]
    fn cmyk_black_uses_only_the_k_layer() {
        let mut j = job(solid(0, 8));
        j.settings.color_mode = ColorMode::Cmyk;
        let result = render(&j).unwrap();
        // Four layers exist; only black has ink for pure black pixels.
        assert!(result.shapes > 0);
        assert!(result.svg.contains("id=\"black\""));
        assert!(!result.svg.contains("id=\"cyan\""));
        assert!(!result.svg.contains("id=\"magenta\""));
        assert!(!result.svg.contains("id=\"yellow\""));
    }

    #[test]
    fn all_styles_render() {
        for style in [
            crate::settings::HalftoneStyle::Dots,
            crate::settings::HalftoneStyle::Lines,
            crate::settings::HalftoneStyle::Crosshatch,
            crate::settings::HalftoneStyle::Ordered,
            crate::settings::HalftoneStyle::Diffusion,
        ] {
            let mut j = job(solid(80, 16));
            j.settings.style = style;
            let result = render(&j).unwrap();
            assert!(!result.is_empty(), "style {style} produced nothing");
        }
    }
}
