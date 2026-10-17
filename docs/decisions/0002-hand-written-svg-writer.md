# 0002 Hand-written SVG writer

## Context

Halftone output is a bounded set of simple primitives: circles, rectangles,
ellipses and polygons, each with a single fill color, grouped per ink layer.
We want compact, predictable output and full control over number formatting.

## Decision

Write a small SVG serializer in `halftone-core::svg`. `SvgDoc` holds the canvas
size, an optional background, and a list of `Layer`s; `to_svg` emits XML text
directly.

## Alternatives considered

- **`svg` crate.** Adds a dependency and an abstraction that does not buy much
  for such a small primitive set. Rejected as unnecessary.
- **`usvg`.** A parser and simplifier tailored to reading SVG, including
  rasterized images and filters. Not a lightweight builder. Rejected.
- **Generating raster PNG instead of vector.** Defeats the purpose; the whole
  point is scalable vector output.

## Consequences

- No extra dependency and exact control over formatting (two-decimal
  coordinates, no trailing zeros), which keeps files small.
- Roughly two hundred lines to maintain and test.
- We own escaping. Only layer ids need it, and they are sanitized to a safe
  alphabet.
- Adding new primitives (for example arcs) means extending both `Geometry` and
  the writer, which is straightforward.
