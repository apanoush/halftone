# Documentation index

Halftone converts images and 3D OBJ meshes into vector halftone SVG.

## Overview

- [Architecture](architecture.md): the rendering pipeline and crate layout.
- [Decisions](decisions/): records of significant design choices.
  - [0001 Software rasterizer](decisions/0001-software-rasterizer.md)
  - [0002 Hand-written SVG writer](decisions/0002-hand-written-svg-writer.md)
  - [0003 Pixel-based canvas for v1](decisions/0003-pixel-based-canvas.md)
  - [0004 Configuration locations](decisions/0004-config-locations.md)

## Packages

### halftone-core

The engine. One module per concern:

| Module | Responsibility |
| --- | --- |
| `color` | `Rgba` with hex string serde |
| `settings` | `HalftoneSettings`, enums, `SettingsPatch` |
| `preset` | Built-in presets and `AppConfig` |
| `config` | `config.toml` load and save |
| `raster` | RGBA8 buffer with luminance sampling |
| `image_source` | Decode, resize, canvas resolution |
| `tone` | Brightness, contrast, gamma, invert |
| `coverage` | Ink coverage maps, mono and CMYK separation |
| `halftone` | Pattern algorithms (dots, lines, ordered, diffusion) |
| `mesh` | OBJ loading and normalization |
| `render3d` | Software rasterizer and camera |
| `shape` | Vector primitives (`Geometry`, `Layer`) |
| `svg` | SVG document and writer |
| `pipeline` | End-to-end render entry point |

### halftone-cli

The `halftone` binary. Parses arguments with `clap`, resolves a preset, applies
overrides, and calls `pipeline::render`.

### halftone-gui

The `halftone-gui` binary. iced desktop UI. Keeps the loaded source in memory,
rebuilds the SVG on every change, and rasterizes it with `resvg` for an exact
preview. For meshes it calls the rasterizer directly and supports drag rotation.
