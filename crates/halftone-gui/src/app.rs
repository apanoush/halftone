//! Application state, messages and view.

use std::path::{Path, PathBuf};

use halftone_core::config;
use halftone_core::pipeline;
use halftone_core::render3d::Camera;
use halftone_core::{AppConfig, ColorMode, DotShape, HalftoneSettings, HalftoneStyle, Rgba};
use iced::widget::{
    button, checkbox, column, container, image, mouse_area, pick_list, row, scrollable, slider,
    space, text, text_input,
};
use iced::{Alignment, Background, Color, ContentFit, Element, Length, Point, Task, Theme, mouse};

use crate::preview::{self, PreviewImage};
use crate::source::{self, SourceKind};

/// Longest side of the preview canvas, in pixels.
const PREVIEW_MAX: u32 = 720;

const STYLES: [HalftoneStyle; 5] = [
    HalftoneStyle::Dots,
    HalftoneStyle::Lines,
    HalftoneStyle::Crosshatch,
    HalftoneStyle::Ordered,
    HalftoneStyle::Diffusion,
];
const SHAPES: [DotShape; 5] = [
    DotShape::Circle,
    DotShape::Square,
    DotShape::Diamond,
    DotShape::Ellipse,
    DotShape::Cross,
];
const COLOR_MODES: [ColorMode; 2] = [ColorMode::Mono, ColorMode::Cmyk];
const BAYER_SIZES: [u8; 2] = [4, 8];

/// All events the UI can produce.
#[derive(Clone, Debug)]
pub enum Message {
    OpenImage,
    OpenMesh,
    ImageChosen(Option<PathBuf>),
    MeshChosen(Option<PathBuf>),
    Export,
    ExportChosen(Option<PathBuf>),

    PresetSelected(String),
    PresetNameChanged(String),
    SavePreset,

    StyleSelected(HalftoneStyle),
    ShapeSelected(DotShape),
    ColorModeSelected(ColorMode),
    BayerSizeSelected(u8),
    CellSizeChanged(f32),
    AngleChanged(f32),
    MaxDotScaleChanged(f32),
    InvertToggled(bool),
    ContrastChanged(f32),
    BrightnessChanged(f32),
    GammaChanged(f32),
    ForegroundChanged(String),
    BackgroundChanged(String),
    WidthChanged(String),
    HeightChanged(String),

    RotXChanged(f32),
    RotYChanged(f32),
    RotZChanged(f32),
    ZoomChanged(f32),
    AmbientChanged(f32),
    FovChanged(f32),

    PreviewPressed,
    PreviewReleased,
    PreviewMoved(Point),
}

/// The application.
pub struct Halftone {
    config: AppConfig,
    presets: Vec<String>,
    settings: HalftoneSettings,
    camera: Camera,
    source: SourceKind,
    source_label: String,
    preview: Option<PreviewImage>,
    preview_dirty: bool,
    dragging: bool,
    last_point: Option<Point>,

    preset_name: Option<String>,
    preset_field: String,
    foreground_text: String,
    background_text: String,
    width_text: String,
    height_text: String,
    config_path: Option<PathBuf>,
    status: String,
    error: Option<String>,
}

impl Halftone {
    pub fn new() -> Self {
        match config::load(None) {
            Ok(config) => Self::from_config(config),
            Err(error) => {
                let mut app = Self::from_config(AppConfig::default());
                app.error = Some(error.to_string());
                app
            }
        }
    }

    /// Build the application around a specific configuration.
    ///
    /// Used by tests so they do not depend on the ambient user config.
    pub fn from_config(config: AppConfig) -> Self {
        let settings = config.resolve(&config.default_preset).unwrap_or_default();
        let presets = config
            .preset_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        let mut app = Self {
            preset_name: Some(config.default_preset.clone()),
            config,
            presets,
            settings,
            camera: Camera::default(),
            source: SourceKind::None,
            source_label: String::from("no source"),
            preview: None,
            preview_dirty: false,
            dragging: false,
            last_point: None,
            preset_field: String::new(),
            foreground_text: String::new(),
            background_text: String::new(),
            width_text: String::new(),
            height_text: String::new(),
            config_path: config::user_config_path(),
            status: String::from("Open an image or an OBJ mesh to begin."),
            error: None,
        };
        app.sync_text_fields();
        app
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenImage => {
                return Task::perform(pick_image(), Message::ImageChosen);
            }
            Message::OpenMesh => {
                return Task::perform(pick_mesh(), Message::MeshChosen);
            }
            Message::ImageChosen(path) => {
                if let Some(path) = path {
                    match source::load_image(&path) {
                        Ok(raster) => {
                            self.source = SourceKind::Image(raster);
                            self.source_label = file_label(&path);
                            self.settings.width = None;
                            self.settings.height = None;
                            self.sync_text_fields();
                            self.error = None;
                            self.mark_dirty();
                        }
                        Err(message) => self.error = Some(message),
                    }
                }
            }
            Message::MeshChosen(path) => {
                if let Some(path) = path {
                    match source::load_mesh(&path) {
                        Ok(mesh) => {
                            self.source = SourceKind::Mesh(mesh);
                            self.source_label = file_label(&path);
                            self.settings.width = None;
                            self.settings.height = None;
                            self.sync_text_fields();
                            self.error = None;
                            self.mark_dirty();
                        }
                        Err(message) => self.error = Some(message),
                    }
                }
            }
            Message::Export => {
                if self.source.is_loaded() {
                    let name = self.export_name();
                    return Task::perform(save_svg(name), Message::ExportChosen);
                }
            }
            Message::ExportChosen(path) => {
                if let Some(path) = path {
                    self.export(&path);
                }
            }

            Message::PresetSelected(name) => {
                if let Some(settings) = self.config.resolve(&name) {
                    self.settings = settings;
                    self.preset_name = Some(name);
                    self.sync_text_fields();
                    self.error = None;
                    self.mark_dirty();
                }
            }
            Message::PresetNameChanged(name) => self.preset_field = name,
            Message::SavePreset => self.save_preset(),

            Message::StyleSelected(value) => self.edit(|s| s.style = value),
            Message::ShapeSelected(value) => self.edit(|s| s.dot_shape = value),
            Message::ColorModeSelected(value) => self.edit(|s| s.color_mode = value),
            Message::BayerSizeSelected(value) => self.edit(|s| s.bayer_size = value),
            Message::CellSizeChanged(value) => self.edit(|s| s.cell_size = value),
            Message::AngleChanged(value) => self.edit(|s| s.angle = value),
            Message::MaxDotScaleChanged(value) => self.edit(|s| s.max_dot_scale = value),
            Message::InvertToggled(value) => self.edit(|s| s.invert = value),
            Message::ContrastChanged(value) => self.edit(|s| s.contrast = value),
            Message::BrightnessChanged(value) => self.edit(|s| s.brightness = value),
            Message::GammaChanged(value) => self.edit(|s| s.gamma = value),
            Message::ForegroundChanged(value) => {
                self.foreground_text = value;
                if let Some(color) = Rgba::from_hex(&self.foreground_text) {
                    self.edit(|s| s.foreground = color);
                }
            }
            Message::BackgroundChanged(value) => {
                self.background_text = value;
                if let Some(color) = Rgba::from_hex(&self.background_text) {
                    self.edit(|s| s.background = color);
                }
            }
            Message::WidthChanged(value) => {
                self.width_text = value;
                self.settings.width = parse_dimension(&self.width_text);
                self.mark_dirty();
            }
            Message::HeightChanged(value) => {
                self.height_text = value;
                self.settings.height = parse_dimension(&self.height_text);
                self.mark_dirty();
            }

            Message::RotXChanged(value) => self.set_rotation(0, value),
            Message::RotYChanged(value) => self.set_rotation(1, value),
            Message::RotZChanged(value) => self.set_rotation(2, value),
            Message::ZoomChanged(value) => {
                self.camera.zoom = value;
                self.mark_dirty();
            }
            Message::AmbientChanged(value) => {
                self.camera.ambient = value;
                self.mark_dirty();
            }
            Message::FovChanged(value) => {
                self.camera.fov = value;
                self.mark_dirty();
            }

            Message::PreviewPressed => {
                self.dragging = self.source.is_mesh();
                self.last_point = None;
            }
            Message::PreviewReleased => {
                self.dragging = false;
                self.last_point = None;
            }
            Message::PreviewMoved(point) => {
                if self.dragging {
                    if let Some(last) = self.last_point {
                        let dx = point.x - last.x;
                        let dy = point.y - last.y;
                        self.camera.rotation[1] += dx * 0.4;
                        self.camera.rotation[0] =
                            (self.camera.rotation[0] - dy * 0.4).clamp(-89.0, 89.0);
                        self.mark_dirty();
                    }
                    self.last_point = Some(point);
                }
            }
        }

        if self.preview_dirty {
            self.refresh_preview();
        }
        Task::none()
    }

    /// Apply a settings mutation and mark the preview stale.
    fn edit(&mut self, mutate: impl FnOnce(&mut HalftoneSettings)) {
        mutate(&mut self.settings);
        self.preset_name = None;
        self.mark_dirty();
    }

    fn set_rotation(&mut self, axis: usize, value: f32) {
        self.camera.rotation[axis] = value;
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.preview_dirty = true;
        self.preset_name = None;
    }

    fn sync_text_fields(&mut self) {
        self.foreground_text = self.settings.foreground.to_hex();
        self.background_text = self.settings.background.to_hex();
        self.width_text = self
            .settings
            .width
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.height_text = self
            .settings
            .height
            .map(|v| v.to_string())
            .unwrap_or_default();
    }

    fn refresh_preview(&mut self) {
        self.preview_dirty = false;
        let Some(job) = source::build_job(
            &self.source,
            &self.settings,
            &self.camera,
            Some(PREVIEW_MAX),
        ) else {
            self.preview = None;
            return;
        };
        match pipeline::render(&job) {
            Ok(result) => {
                self.status = format!(
                    "{}x{} px, {} shapes",
                    result.width, result.height, result.shapes
                );
                self.preview = preview::rasterize_svg(&result.svg, PREVIEW_MAX);
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error.to_string());
                self.preview = None;
            }
        }
    }

    fn export(&mut self, path: &Path) {
        let Some(job) = source::build_job(&self.source, &self.settings, &self.camera, None) else {
            self.error = Some("nothing to export".to_string());
            return;
        };
        match pipeline::render(&job) {
            Ok(result) => match std::fs::write(path, &result.svg) {
                Ok(()) => {
                    self.status = format!("exported {} ({} shapes)", path.display(), result.shapes);
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn save_preset(&mut self) {
        let name = self.preset_field.trim().to_string();
        if name.is_empty() {
            self.error = Some("Enter a preset name before saving.".to_string());
            return;
        }
        let target = match config::save_target(None) {
            Ok(path) => path,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        match config::save_preset(&target, &name, &self.settings) {
            Ok(()) => {
                self.config = config::load(None).unwrap_or_default();
                self.presets = self
                    .config
                    .preset_names()
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                self.preset_name = Some(name.clone());
                self.preset_field.clear();
                self.status = format!("saved preset '{name}' to {}", target.display());
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn export_name(&self) -> String {
        let stem = self
            .source_label
            .rsplit_once('.')
            .map(|(stem, _)| stem.to_string())
            .unwrap_or_else(|| "halftone".to_string());
        format!("{stem}.halftone.svg")
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = row![scrollable(self.sidebar()).width(340), self.canvas()].height(Length::Fill);

        column![self.header(), body, self.footer()]
            .spacing(10)
            .padding(12)
            .into()
    }

    fn header(&self) -> Element<'_, Message> {
        let open_image = button("Open image").on_press(Message::OpenImage);
        let open_mesh = button("Open mesh").on_press(Message::OpenMesh);
        let export =
            button("Export SVG").on_press_maybe(self.source.is_loaded().then_some(Message::Export));

        row![
            text("Halftone").size(22),
            text(self.source_label.as_str()).size(14),
            space::horizontal(),
            open_image,
            open_mesh,
            export,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    fn footer(&self) -> Element<'_, Message> {
        let message = match &self.error {
            Some(error) => text(format!("Error: {error}")).color(Color::from_rgb(1.0, 0.45, 0.45)),
            None => text(self.status.as_str()),
        };
        let config_line = match &self.config_path {
            Some(path) => format!("config: {}", path.display()),
            None => String::from("config: built-ins only"),
        };
        container(
            column![message, text(config_line).size(12)]
                .spacing(2)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let mut sections = column![].spacing(14).padding(2);

        // Source.
        let source_info = match self.source.dimensions() {
            Some((width, height)) => format!("{} ({}x{})", self.source_label, width, height),
            None if self.source.is_loaded() => self.source_label.clone(),
            None => String::from("none"),
        };
        sections = sections.push(section("Source", text(source_info).into()));

        // Presets.
        let preset_picker = pick_list(
            &self.presets[..],
            self.preset_name.clone(),
            Message::PresetSelected,
        )
        .width(Length::Fill)
        .placeholder("custom");
        let save_row = row![
            text_input("new preset name", self.preset_field.as_str())
                .on_input(Message::PresetNameChanged)
                .width(Length::Fill),
            button("Save").on_press(Message::SavePreset),
        ]
        .spacing(6);
        sections = sections.push(section(
            "Preset",
            column![preset_picker, save_row].spacing(6).into(),
        ));

        // Pattern.
        let mut pattern = column![
            labeled(
                "Style",
                pick_list(
                    &STYLES[..],
                    Some(self.settings.style),
                    Message::StyleSelected
                )
                .width(Length::Fill)
            ),
            labeled(
                "Dot shape",
                pick_list(
                    &SHAPES[..],
                    Some(self.settings.dot_shape),
                    Message::ShapeSelected
                )
                .width(Length::Fill)
            ),
            labeled(
                "Color mode",
                pick_list(
                    &COLOR_MODES[..],
                    Some(self.settings.color_mode),
                    Message::ColorModeSelected
                )
                .width(Length::Fill)
            ),
            labeled(
                "Cell size",
                slider(
                    2.0..=40.0,
                    self.settings.cell_size,
                    Message::CellSizeChanged
                )
                .step(0.5)
            ),
            labeled(
                "Angle",
                slider(-180.0..=180.0, self.settings.angle, Message::AngleChanged).step(1.0)
            ),
            labeled(
                "Dot scale",
                slider(
                    0.2..=1.6,
                    self.settings.max_dot_scale,
                    Message::MaxDotScaleChanged
                )
                .step(0.05)
            ),
        ]
        .spacing(8);
        if self.settings.style == HalftoneStyle::Ordered {
            pattern = pattern.push(labeled(
                "Matrix",
                pick_list(
                    &BAYER_SIZES[..],
                    Some(self.settings.bayer_size),
                    Message::BayerSizeSelected,
                )
                .width(Length::Fill),
            ));
        }
        sections = sections.push(section("Pattern", pattern.into()));

        // Tone.
        sections = sections.push(section(
            "Tone",
            column![
                labeled(
                    "Contrast",
                    slider(0.2..=2.5, self.settings.contrast, Message::ContrastChanged).step(0.05)
                ),
                labeled(
                    "Brightness",
                    slider(
                        -0.5..=0.5,
                        self.settings.brightness,
                        Message::BrightnessChanged
                    )
                    .step(0.01)
                ),
                labeled(
                    "Gamma",
                    slider(0.3..=3.0, self.settings.gamma, Message::GammaChanged).step(0.05)
                ),
                checkbox(self.settings.invert)
                    .label("Invert source")
                    .on_toggle(Message::InvertToggled),
            ]
            .spacing(8)
            .into(),
        ));

        // Ink.
        sections = sections.push(section(
            "Ink",
            column![
                labeled(
                    "Foreground",
                    text_input("#000000", self.foreground_text.as_str())
                        .on_input(Message::ForegroundChanged)
                ),
                labeled(
                    "Background",
                    text_input("none", self.background_text.as_str())
                        .on_input(Message::BackgroundChanged)
                ),
            ]
            .spacing(8)
            .into(),
        ));

        // Canvas.
        sections = sections.push(section(
            "Canvas",
            column![
                labeled(
                    "Width",
                    text_input("auto", self.width_text.as_str()).on_input(Message::WidthChanged)
                ),
                labeled(
                    "Height",
                    text_input("auto", self.height_text.as_str()).on_input(Message::HeightChanged)
                ),
                text("Leave empty to use the source size.").size(12),
            ]
            .spacing(8)
            .into(),
        ));

        // Camera.
        if self.source.is_mesh() {
            sections = sections.push(section(
                "Camera",
                column![
                    text("Drag the preview to rotate.").size(12),
                    labeled(
                        "Rotate X",
                        slider(
                            -180.0..=180.0,
                            self.camera.rotation[0],
                            Message::RotXChanged
                        )
                        .step(1.0)
                    ),
                    labeled(
                        "Rotate Y",
                        slider(
                            -180.0..=180.0,
                            self.camera.rotation[1],
                            Message::RotYChanged
                        )
                        .step(1.0)
                    ),
                    labeled(
                        "Rotate Z",
                        slider(
                            -180.0..=180.0,
                            self.camera.rotation[2],
                            Message::RotZChanged
                        )
                        .step(1.0)
                    ),
                    labeled(
                        "Zoom",
                        slider(0.2..=3.0, self.camera.zoom, Message::ZoomChanged).step(0.05)
                    ),
                    labeled(
                        "Ambient",
                        slider(0.0..=1.0, self.camera.ambient, Message::AmbientChanged).step(0.02)
                    ),
                    labeled(
                        "FOV",
                        slider(20.0..=90.0, self.camera.fov, Message::FovChanged).step(1.0)
                    ),
                ]
                .spacing(8)
                .into(),
            ));
        }

        sections.into()
    }

    fn canvas(&self) -> Element<'_, Message> {
        let inner: Element<'_, Message> = match &self.preview {
            Some(preview) => image(preview.handle.clone())
                .content_fit(ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            None => text("Open an image or an OBJ mesh to begin.").into(),
        };
        let interaction = if self.source.is_mesh() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::Idle
        };
        mouse_area(
            container(inner)
                .center(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(12)
                .style(canvas_style),
        )
        .on_press(Message::PreviewPressed)
        .on_release(Message::PreviewReleased)
        .on_move(Message::PreviewMoved)
        .interaction(interaction)
        .into()
    }
}

impl Default for Halftone {
    fn default() -> Self {
        Self::new()
    }
}

fn section<'a>(title: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    column![text(title).size(16), body].spacing(6).into()
}

fn labeled<'a>(label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(13), control.into()]
        .spacing(3)
        .into()
}

fn canvas_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.07, 0.07, 0.09))),
        border: iced::Border {
            color: Color::from_rgb(0.2, 0.2, 0.24),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

fn parse_dimension(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

async fn pick_image() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter(
            "Images",
            &["png", "jpg", "jpeg", "webp", "bmp", "tiff", "ppm", "gif"],
        )
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

async fn pick_mesh() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Wavefront OBJ", &["obj"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

async fn save_svg(default_name: String) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("SVG", &["svg"])
        .set_file_name(default_name)
        .save_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use halftone_core::color::Rgba;
    use halftone_core::raster::Raster;

    fn solid(value: u8, size: u32) -> Raster {
        let mut raster = Raster::new(size, size);
        for y in 0..size {
            for x in 0..size {
                raster.set_pixel(x, y, Rgba::rgb(value, value, value));
            }
        }
        raster
    }

    #[test]
    fn changing_settings_regenerates_preview() {
        let mut app = Halftone::from_config(AppConfig::default());
        app.source = SourceKind::Image(solid(0, 32));
        let _ = app.update(Message::CellSizeChanged(8.0));
        assert!(app.preview.is_some());
        assert!(app.status.contains("shapes"), "status: {}", app.status);
        assert!(app.error.is_none(), "error: {:?}", app.error);
    }

    #[test]
    fn selecting_a_preset_updates_settings() {
        let mut app = Halftone::from_config(AppConfig::default());
        app.source = SourceKind::Image(solid(120, 32));
        let _ = app.update(Message::PresetSelected("ink".to_string()));
        assert_eq!(app.settings.style, HalftoneStyle::Lines);
        assert!(app.preview.is_some());
    }

    #[test]
    fn invalid_color_text_keeps_previous_value() {
        let mut app = Halftone::from_config(AppConfig::default());
        let before = app.settings.foreground;
        let _ = app.update(Message::ForegroundChanged("nope".to_string()));
        assert_eq!(app.settings.foreground, before);
        let _ = app.update(Message::ForegroundChanged("#ff0000".to_string()));
        assert_eq!(app.settings.foreground, Rgba::rgb(255, 0, 0));
    }

    #[test]
    fn dragging_rotates_only_meshes() {
        let mut app = Halftone::from_config(AppConfig::default());
        app.source = SourceKind::Image(solid(255, 8));
        let _ = app.update(Message::PreviewPressed);
        assert!(!app.dragging);
    }
}
