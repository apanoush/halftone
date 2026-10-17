//! `halftone-gui` entry point.

mod app;
mod preview;
mod source;

use iced::{Size, Theme};

fn main() -> iced::Result {
    iced::application(
        app::Halftone::new,
        app::Halftone::update,
        app::Halftone::view,
    )
    .title("Halftone")
    .theme(Theme::TokyoNight)
    .window_size(Size::new(1280.0, 840.0))
    .antialiasing(true)
    .run()
}
