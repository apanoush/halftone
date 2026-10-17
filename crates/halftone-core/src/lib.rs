//! Halftone conversion engine.
//!
//! The crate is organized as a data pipeline:
//!
//! `Source` -> `Raster` -> `CoverageMap` -> vector `Geometry` -> `SvgDoc`
//!
//! Both the CLI and the GUI drive the same [`pipeline::render`] entry point.

pub mod color;
pub mod config;
pub mod coverage;
pub mod error;
pub mod halftone;
pub mod image_source;
pub mod mesh;
pub mod preset;
pub mod raster;
pub mod render3d;
pub mod settings;
pub mod shape;
pub mod svg;
pub mod tone;

pub mod pipeline;

pub use color::Rgba;
pub use error::{Error, Result};
pub use preset::AppConfig;
pub use settings::{ColorMode, DotShape, HalftoneSettings, HalftoneStyle, SettingsPatch};
pub use shape::{Geometry, Layer};
