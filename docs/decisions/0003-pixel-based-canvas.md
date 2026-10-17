# 0003 Pixel-based canvas for version 1

## Context

"Size" in halftoning is ambiguous. For print work it means a physical size (for
example 100 x 150 mm) plus a screen frequency in lines per inch, with cell size
stated physically. For screen work it means a pixel canvas plus a cell size in
pixels. Version 1 had to pick one.

## Decision

Use a pixel-based canvas: `width` and `height` in pixels (defaulting to the
source size) and `cell_size` in pixels. The SVG carries matching `width`,
`height` and `viewBox`. The default background is transparent, so dots carry
all color and the result composites cleanly.

## Alternatives considered

- **Physical units plus LPI now.** Correct print semantics out of the box, but
  it needs a DPI assumption or explicit units throughout the UI and config, and
  it complicates previews. Deferred.
- **Both from the start.** More surface area than version 1 warranted.

## Consequences

- Simple mental model and direct previews: one pixel in the SVG is one pixel in
  the preview.
- LPI and physical sizes are absent. They can be added later without changing
  the engine: map millimeters to pixels with a chosen DPI, then express the
  screen frequency as a cell size. The SVG already emits real `width` and
  `height` attributes, so physical units can be switched on at the writer.
