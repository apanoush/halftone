# Architecture

## Data flow

```
Source (image file | OBJ mesh | in-memory raster)
   |
   v
Raster (RGBA8)                       image_source / render3d
   |
   v
CoverageMap (0 = paper, 1 = full ink)   coverage (mono or CMYK)
   |
   v
Vec<Geometry> per ink layer             halftone
   |
   v
SvgDoc -> SVG string                     svg
```

The pipeline is data oriented: each stage is a plain data structure, and each
transform is a function over it. `pipeline::render` wires the stages together
and is the single entry point shared by the CLI and the GUI.

## Why coverage maps

Every pattern algorithm consumes a `CoverageMap`, a single channel where `0`
means no ink and `1` means full ink. Color handling therefore lives entirely in
one place (`CoverageMap::mono` and `CoverageMap::cmyk`), and the dot, line,
dither and diffusion generators never need to know about color. Mono reduces to
`1 - luminance`; CMYK separates the tone-adjusted RGB into four ink amounts at
classic screen angles.

## Screen geometry

Dot and line patterns are laid out on a lattice rotated by the screen angle.

- `halftone::grid::CellGrid` maps every source pixel to a rotated square cell,
  accumulates coverage, and reports the mean per cell. The dot radius is derived
  from the cell area, so the printed density tracks the image without any manual
  calibration.
- `halftone::grid::LineGrid` does the same in one dimension for parallel line
  bands. Line width is proportional to band coverage.

Ordered and diffusion patterns work on an axis-aligned cell grid because their
threshold matrices and error propagation are defined in raster order.

## 3D rendering

`render3d::rasterize` normalizes the mesh into a unit bounding sphere, applies
an Euler rotation, projects with a perspective matrix, and fills triangles with
a z-buffer. Shading is Lambert plus an ambient term. The result is an ordinary
`Raster`, so the mesh path and the image path converge before halftoning. See
[decisions/0001](decisions/0001-software-rasterizer.md).

## Output

`SvgDoc` holds a background color and a list of `Layer`s, each with one ink
color and a list of `Geometry` primitives (circle, rect, ellipse, polygon).
`to_svg` serializes them with compact number formatting. Dot shapes that are not
circles are emitted as rotated polygons. See
[decisions/0002](decisions/0002-hand-written-svg-writer.md).

## Configuration

`AppConfig` holds a default preset name and a map of `SettingsPatch`. A patch
only lists the fields that differ from `HalftoneSettings::default`, so config
files stay readable.

Locations are resolved by `config`: built-in presets first, then the user config
from `ProjectDirs` (`~/.config/halftone/config.toml` on Linux), then the project
`./config.toml`. `load_from_paths` applies them in order, so a project can
override personal defaults. An explicit `--config` path replaces the user and
project layers, while `save_target` selects the user config for writes unless an
explicit path is given. See
[decisions/0004](decisions/0004-config-locations.md).
