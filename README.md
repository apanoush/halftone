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

`config.toml` at the working directory sets the default preset and adds custom
presets. Presets are applied field by field over the built-in defaults, so a
preset only needs to list what differs:

```toml
default_preset = "classic"

[presets.blueprint]
style = "lines"
cell_size = 5.0
angle = 0.0
foreground = "#1b4f9c"
```

Colors are hex strings (`#rrggbb` or `#rrggbbaa`, and `none` for transparent).
Use `--config PATH` to point at another file.

## Development

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

Sample inputs live in `data/` and generated output goes to `results/`.
