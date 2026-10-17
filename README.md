# Halftone

Convert an image or a 3D OBJ mesh into a vector halftone SVG.

Halftone reproduces a picture as a pattern of dots, lines or dither cells. This
project turns that pattern into **vector** output (SVG), so the result scales
without pixelation and can be edited, printed or plotted.

## Features

- **Image input**: PNG, JPEG, WebP, BMP, TIFF, PPM, GIF (whatever the `image`
  crate decodes).
- **3D input**: Wavefront OBJ, rendered with a built-in software rasterizer.
  The GUI lets you rotate the model with the mouse.
- **Halftone styles**: amplitude-modulated dots on a rotatable screen, parallel
  lines, crosshatch, ordered (Bayer) dither, and Floyd-Steinberg error
  diffusion.
- **Dot shapes**: circle, square, diamond, ellipse, cross.
- **Color**: single-ink mono, or four-layer CMYK at the classic screen angles
  (cyan 15, magenta 75, yellow 0, black 45 degrees).
- **Tone controls**: brightness, contrast, gamma and inversion.
- **Presets**: shipped with the app and editable in `config.toml`.
- **Vector output**: every result is an SVG file.

## Workspace

| Crate | Role |
| --- | --- |
| `crates/halftone-core` | The engine: sources, halftone algorithms, SVG writer, pipeline |
| `crates/halftone-cli` | `halftone`, the command line interface |
| `crates/halftone-gui` | `halftone-gui`, the iced desktop interface |

See `doc/index.md` for the documentation index and `doc/architecture.md` for the
design.

## Build

```sh
cargo build --release
```

Binaries are written to `target/release/halftone` and
`target/release/halftone-gui`.

## Platforms

Halftone runs on Linux, Windows and macOS. The engine and CLI have no platform
specific code; the GUI uses iced, which renders with wgpu (Vulkan, Metal or
DirectX 12) and falls back to the tiny-skia software renderer when no GPU
adapter is available.

| | Status |
| --- | --- |
| Linux | Built and run locally, and covered by CI. Config at `$XDG_CONFIG_HOME/halftone/config.toml`. |
| Windows | Covered by CI. Config at `%APPDATA%\halftone\config\config.toml`. |
| macOS | Covered by CI. Config at `~/Library/Application Support/halftone/config.toml`. |

Requirements:

- Rust 1.85 or newer (the workspace uses the 2024 edition).
- Windows: the MSVC toolchain.
- macOS: the Xcode Command Line Tools.
- Linux: a desktop session for the GUI. File dialogs use the XDG desktop
  portal, so an `xdg-desktop-portal` service must be running (it is present on
  GNOME and KDE by default).

Continuous integration builds, lints, and tests the workspace on all three
operating systems.

## Command line

Convert an image:

```sh
halftone image photo.png -o photo.svg --preset classic
```

Convert a mesh and choose a viewing angle:

```sh
halftone mesh model.obj -o model.svg --preset engraving --rot 25,35,0 --width 800 --height 800
```

Override any setting on top of a preset:

```sh
halftone image photo.png --style dots --cell-size 5 --angle 30 --color-mode cmyk
```

Manage presets:

```sh
halftone preset list
halftone preset show newsprint
halftone preset save bold --preset classic --cell-size 8 --contrast 1.3
```

Run `halftone --help` or `halftone image --help` for the full flag list.

## Desktop interface

```sh
cargo run -p halftone-gui
```

The left panel holds every setting. The preview is rasterized from the exact
SVG that will be exported, so it is what-you-see-is-what-you-get. With an OBJ
loaded, drag the preview to rotate the model. `Export SVG` writes the
full-resolution result.

## Configuration

Configuration is layered. From lowest to highest precedence:

1. Built-in presets, compiled into the tool.
2. The user config:
   - Linux: `$XDG_CONFIG_HOME/halftone/config.toml`, usually `~/.config/halftone/config.toml`
   - macOS: `~/Library/Application Support/halftone/config.toml`
   - Windows: `%APPDATA%\halftone\config\config.toml`
3. The project config, `./config.toml`, when present.

Later layers win, so a project can override your personal defaults. A preset is
applied field by field over the built-in defaults, so it only lists what
differs:

```toml
default_preset = "classic"

[presets.blueprint]
style = "lines"
cell_size = 5.0
angle = 0.0
foreground = "#1b4f9c"
```

Colors are hex strings (`#rrggbb` or `#rrggbbaa`, and `none` for transparent).

- `halftone preset path` prints the config locations in use and where saves go.
- `halftone preset save NAME` writes to the user config by default.
- `--config PATH` uses only the built-ins and that file, ignoring the user and
  project configs. A missing explicit file is an error.

## Development

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI (`.github/workflows/ci.yml`) runs these and a build of both binaries on
Linux, Windows and macOS for every push and pull request.

Sample inputs live in `data/` and generated output goes to `results/`.
