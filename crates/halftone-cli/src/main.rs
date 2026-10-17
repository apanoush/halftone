//! `halftone` command line interface.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use halftone_core::config;
use halftone_core::pipeline::{self, Job, RenderResult};
use halftone_core::render3d::Camera;
use halftone_core::{AppConfig, ColorMode, DotShape, HalftoneSettings, HalftoneStyle, Rgba};

#[derive(Parser)]
#[command(
    name = "halftone",
    version,
    about = "Convert images and 3D OBJ meshes to vector halftone SVG"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert an image file.
    Image(ImageArgs),
    /// Render an OBJ mesh, rotate it, and convert the result.
    Mesh(MeshArgs),
    /// Inspect and edit presets.
    Preset(PresetArgs),
}

#[derive(Args)]
struct ImageArgs {
    /// Input image file.
    input: PathBuf,
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Args)]
struct MeshArgs {
    /// Input OBJ file.
    input: PathBuf,
    #[command(flatten)]
    common: CommonArgs,
    #[command(flatten)]
    camera: CameraArgs,
}

#[derive(Args)]
struct CommonArgs {
    /// Output SVG path. Defaults to `<input>.halftone.svg`.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Preset name. Defaults to the config's `default_preset`.
    #[arg(long)]
    preset: Option<String>,
    /// Config file. Defaults to `./config.toml` when it exists.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Output width in pixels.
    #[arg(long)]
    width: Option<u32>,
    /// Output height in pixels.
    #[arg(long)]
    height: Option<u32>,
    /// Render but do not write the SVG.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    overrides: SettingsOverrides,
}

#[derive(Args, Default)]
struct SettingsOverrides {
    /// Halftone pattern.
    #[arg(long, value_enum)]
    style: Option<StyleArg>,
    /// Cell size in pixels.
    #[arg(long)]
    cell_size: Option<f32>,
    /// AM dot shape.
    #[arg(long, value_enum)]
    dot_shape: Option<ShapeArg>,
    /// Screen angle in degrees.
    #[arg(long)]
    angle: Option<f32>,
    /// Invert the source.
    #[arg(long)]
    invert: bool,
    /// Force inversion off, overriding the preset.
    #[arg(long)]
    no_invert: bool,
    /// Contrast multiplier around mid-gray.
    #[arg(long)]
    contrast: Option<f32>,
    /// Brightness offset in -1..=1.
    #[arg(long)]
    brightness: Option<f32>,
    /// Display gamma. Values above 1 lighten.
    #[arg(long)]
    gamma: Option<f32>,
    /// Color handling.
    #[arg(long, value_enum)]
    color_mode: Option<ColorModeArg>,
    /// Ink color, as #rrggbb.
    #[arg(long, value_name = "#RRGGBB")]
    fg: Option<Rgba>,
    /// Background color, or `none` for transparent.
    #[arg(long, value_name = "#RRGGBB|none")]
    bg: Option<Rgba>,
    /// Coverage multiplier for dot size.
    #[arg(long)]
    max_dot_scale: Option<f32>,
    /// Ordered dither matrix size (4 or 8).
    #[arg(long)]
    bayer_size: Option<u8>,
}

impl SettingsOverrides {
    fn apply(&self, settings: &mut HalftoneSettings) {
        if let Some(v) = self.style {
            settings.style = v.into();
        }
        if let Some(v) = self.cell_size {
            settings.cell_size = v;
        }
        if let Some(v) = self.dot_shape {
            settings.dot_shape = v.into();
        }
        if let Some(v) = self.angle {
            settings.angle = v;
        }
        if self.invert {
            settings.invert = true;
        }
        if self.no_invert {
            settings.invert = false;
        }
        if let Some(v) = self.contrast {
            settings.contrast = v;
        }
        if let Some(v) = self.brightness {
            settings.brightness = v;
        }
        if let Some(v) = self.gamma {
            settings.gamma = v;
        }
        if let Some(v) = self.color_mode {
            settings.color_mode = v.into();
        }
        if let Some(v) = self.fg {
            settings.foreground = v;
        }
        if let Some(v) = self.bg {
            settings.background = v;
        }
        if let Some(v) = self.max_dot_scale {
            settings.max_dot_scale = v;
        }
        if let Some(v) = self.bayer_size {
            settings.bayer_size = v;
        }
    }
}

#[derive(Args, Default)]
struct CameraArgs {
    /// Euler rotation in degrees, as X,Y,Z.
    #[arg(long, value_name = "X,Y,Z")]
    rot: Option<String>,
    /// Camera zoom, 1.0 fits the object.
    #[arg(long)]
    zoom: Option<f32>,
    /// Light direction, as X,Y,Z.
    #[arg(long, value_name = "X,Y,Z")]
    light: Option<String>,
    /// Ambient light intensity in 0..=1.
    #[arg(long)]
    ambient: Option<f32>,
    /// Vertical field of view in degrees.
    #[arg(long)]
    fov: Option<f32>,
}

impl CameraArgs {
    fn apply(&self, camera: &mut Camera) -> Result<()> {
        if let Some(raw) = &self.rot {
            camera.rotation = parse_vec3(raw).context("invalid --rot")?;
        }
        if let Some(v) = self.zoom {
            camera.zoom = v;
        }
        if let Some(raw) = &self.light {
            camera.light = parse_vec3(raw).context("invalid --light")?;
        }
        if let Some(v) = self.ambient {
            camera.ambient = v;
        }
        if let Some(v) = self.fov {
            camera.fov = v;
        }
        Ok(())
    }
}

#[derive(Args)]
struct PresetArgs {
    #[command(subcommand)]
    command: PresetCommand,
}

#[derive(Subcommand)]
enum PresetCommand {
    /// List available presets.
    List {
        /// Config file.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    /// Print one preset as TOML.
    Show {
        name: String,
        /// Config file.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    /// Save settings as a preset.
    Save {
        name: String,
        /// Config file to write. Defaults to `./config.toml`.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// Base preset to copy before applying overrides.
        #[arg(long)]
        preset: Option<String>,
        #[command(flatten)]
        overrides: SettingsOverrides,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum StyleArg {
    Dots,
    Lines,
    Crosshatch,
    Ordered,
    Diffusion,
}

impl From<StyleArg> for HalftoneStyle {
    fn from(value: StyleArg) -> Self {
        match value {
            StyleArg::Dots => Self::Dots,
            StyleArg::Lines => Self::Lines,
            StyleArg::Crosshatch => Self::Crosshatch,
            StyleArg::Ordered => Self::Ordered,
            StyleArg::Diffusion => Self::Diffusion,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ShapeArg {
    Circle,
    Square,
    Diamond,
    Ellipse,
    Cross,
}

impl From<ShapeArg> for DotShape {
    fn from(value: ShapeArg) -> Self {
        match value {
            ShapeArg::Circle => Self::Circle,
            ShapeArg::Square => Self::Square,
            ShapeArg::Diamond => Self::Diamond,
            ShapeArg::Ellipse => Self::Ellipse,
            ShapeArg::Cross => Self::Cross,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorModeArg {
    Mono,
    Cmyk,
}

impl From<ColorModeArg> for ColorMode {
    fn from(value: ColorModeArg) -> Self {
        match value {
            ColorModeArg::Mono => Self::Mono,
            ColorModeArg::Cmyk => Self::Cmyk,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Image(args) => run_image(args),
        Command::Mesh(args) => run_mesh(args),
        Command::Preset(args) => run_preset(args),
    }
}

fn run_image(args: ImageArgs) -> Result<()> {
    let settings = resolve_settings(&args.common)?;
    let job = Job::image(&args.input, settings);
    let result = pipeline::render(&job).context("rendering failed")?;
    finish(&args.input, &args.common, &result)
}

fn run_mesh(args: MeshArgs) -> Result<()> {
    let settings = resolve_settings(&args.common)?;
    let mut job = Job::mesh(&args.input, settings);
    args.camera.apply(&mut job.camera)?;
    let result = pipeline::render(&job).context("rendering failed")?;
    finish(&args.input, &args.common, &result)
}

fn run_preset(args: PresetArgs) -> Result<()> {
    match args.command {
        PresetCommand::List { config: path } => {
            let cfg = config::load(config::discover(path.as_deref()).as_deref())?;
            for name in cfg.preset_names() {
                let marker = if name == cfg.default_preset { " *" } else { "" };
                println!("{name}{marker}");
            }
            Ok(())
        }
        PresetCommand::Show { name, config: path } => {
            let cfg = config::load(config::discover(path.as_deref()).as_deref())?;
            let patch = cfg
                .presets
                .get(&name)
                .ok_or_else(|| anyhow!("unknown preset '{name}'"))?;
            print!("{}", config::describe_patch(patch)?);
            Ok(())
        }
        PresetCommand::Save {
            name,
            config: path,
            preset,
            overrides,
        } => {
            let path = path.unwrap_or_else(|| PathBuf::from("config.toml"));
            let cfg = config::load(config::discover(Some(&path)).as_deref())?;
            let base_name = preset.unwrap_or_else(|| cfg.default_preset.clone());
            let mut settings = cfg
                .resolve(&base_name)
                .ok_or_else(|| anyhow!("unknown base preset '{base_name}'"))?;
            overrides.apply(&mut settings);
            settings.validate().map_err(|e| anyhow!(e))?;
            config::save_preset(&path, &name, &settings)?;
            println!("saved preset '{name}' to {}", path.display());
            Ok(())
        }
    }
}

fn resolve_settings(common: &CommonArgs) -> Result<HalftoneSettings> {
    let path = config::discover(common.config.as_deref());
    let cfg: AppConfig = config::load(path.as_deref())?;
    let name = common
        .preset
        .clone()
        .unwrap_or_else(|| cfg.default_preset.clone());
    let mut settings = cfg
        .resolve(&name)
        .ok_or_else(|| anyhow!("unknown preset '{name}'; run `halftone preset list`"))?;
    common.overrides.apply(&mut settings);
    if common.width.is_some() {
        settings.width = common.width;
    }
    if common.height.is_some() {
        settings.height = common.height;
    }
    settings.validate().map_err(|e| anyhow!(e))?;
    Ok(settings)
}

fn finish(input: &Path, common: &CommonArgs, result: &RenderResult) -> Result<()> {
    if common.dry_run {
        println!(
            "{}x{}, {} shapes (dry run)",
            result.width, result.height, result.shapes
        );
        return Ok(());
    }
    let output = output_path(input, common.output.as_deref());
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, &result.svg)
        .with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "wrote {} ({}x{}, {} shapes)",
        output.display(),
        result.width,
        result.height,
        result.shapes
    );
    Ok(())
}

fn output_path(input: &Path, output: Option<&Path>) -> PathBuf {
    output.map(Path::to_path_buf).unwrap_or_else(|| {
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "output".to_string());
        input.with_file_name(format!("{stem}.halftone.svg"))
    })
}

fn parse_vec3(raw: &str) -> Result<[f32; 3]> {
    let parts: Vec<_> = raw.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        bail!("expected three comma-separated numbers, got '{raw}'");
    }
    let mut out = [0.0f32; 3];
    for (slot, part) in out.iter_mut().zip(parts) {
        *slot = part
            .parse::<f32>()
            .with_context(|| format!("'{}' is not a number", part))?;
    }
    Ok(out)
}
