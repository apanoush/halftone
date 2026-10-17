# 0001 Software rasterizer for meshes

## Context

The 3D path must turn an OBJ mesh into a 2D image before halftoning. Two front
ends need this: the CLI (headless, deterministic) and the GUI (interactive
rotation). Halftoning only needs a shaded luminance image, not a photorealistic
render.

## Decision

Implement a small software rasterizer in `halftone-core` (`render3d`): mesh
normalization, Euler model rotation, perspective projection via `glam`, an
OpenGL-style z-buffer, and Lambert shading with an ambient term. Output is an
ordinary `Raster`, identical to the image path.

## Alternatives considered

- **wgpu offscreen rendering.** Closer to a real renderer, but adds a large
  dependency, needs a working adapter, and is fragile in headless environments
  and CI. Rejected for the added complexity.
- **Embedded 3D viewport (`three-d`, raw OpenGL).** Couples rendering to a
  window and an event loop, so the CLI could not share it. Rejected.
- **CPU ray tracing.** Better soft shadows and reflections, but far slower and
  unnecessary: the halftone step discards most of that detail.

## Consequences

- Deterministic, headless, testable, and shared verbatim by CLI and GUI.
- No GPU dependency for the engine.
- Shading is limited to Lambert plus ambient: no textures, no PBR, no shadows.
  Acceptable because the halftone screen dominates the final look.
- Cost scales with canvas pixels times triangles; fine for typical meshes and
  proof/preview sizes. Very dense meshes would need mesh decimation or a GPU
  path later.
