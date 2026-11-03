# Built-in icon guidelines

These rules apply to GRKit built-in icons. They follow the static image audit of
2026-11-03 and the subsequent icon revision. Review new artwork at its original
size and in the simulator before delivery.

## Scope and evidence

The audit covers 33 module PNGs and 11 samples rendered by the existing core
artwork functions. It also compares 14 original GR III 2.10 resources extracted
directly from the firmware icon table and `ICONBIN`. No simulator captures were
used as source artwork. GR IV resources have not been audited.

The GR III firmware SHA-256 is
`d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4`.
Its icon descriptor table is at `0x550020fc`. The static scan found 388 resources
of 40 × 40 pixels. The selected examples include Focus (`0x096`), Std (`0x166`),
Custom1 (`0x15b`), and Custom2 (`0x15c`). A visible label does not establish the
menu function of a resource. For example, the functions of the Grid and Date
text resources were not traced.

The local audit outputs are in `dist/icon-audit/`: pixel metrics, contour
measurements, original resource extracts, and an image comparison page. Firmware
extracts remain local artifacts and are not part of the maintained artwork.

Implementation references:

- [Core artwork and typography](../crates/grkit-core/src/artwork.rs).
- [Module PNG decoding](../crates/grkit-build/src/module.rs).
- [Effect icon validation](../crates/grkit-core/src/effects.rs).
- [Existing artwork tests](../crates/grkit-core/tests/effects.rs).
- [Native icon descriptors](../runtime/installer.c).

## Current technical contracts

| Property | Current contract |
| --- | --- |
| Canvas | 40 × 40 pixels. The runtime consumes 6400 bytes of RGBA8 per tile. |
| Alpha | Effect validation requires 0 or 255. Repository tests enforce this for built-in module PNGs. Module PNG loading does not yet enforce it. |
| Antialiasing | Current framed artwork blends coverage into RGB over an opaque background. |
| Color | Built-in artwork is grayscale. The loader does not require grayscale. |
| Frame, font, and glyph size | These are design conventions, not general loader constraints. |

The selected framed native samples also use binary alpha. This does not describe
all native resources: 80 of the 388 scanned resources have intermediate alpha.

## Choose the icon role first

Coordinates below are inclusive and start at zero.

| Role | Current opaque bounds | Use |
| --- | --- | --- |
| List tile | `x=1..38, y=2..37`, 38 × 36 | Module entries, their options, OFF, VER, and MOD. |
| Toolbar tile | `x=1..38, y=1..38`, 38 × 38 | The GRK entry in the native ADJ toolbar. |
| Film cover | `x=0..39, y=3..36`, 40 × 34 | Film Looks artwork with its own bands and typography. |

A module's root entry, including Glow, is a **list tile**. Being inside ADJ does
not make it a toolbar tile. Status indicators currently reuse the list artwork.

Current framed GRKit tiles have a one-pixel `#808080` border and `#333333` fill.
The sampled native tiles use the same border, with fill RGB `(53,53,54)`.
This small fill difference is separate from glyph size and contrast. GRKit retains its existing neutral fill.

## Use a layout that fits the meaning

| Layout | Current examples | Rule |
| --- | --- | --- |
| Centered symbol | Framing, Guides, Film Looks, and Glow root entries | Preserve the symbol's proportions and optical center. |
| Symbol with caption | Framing, Guides, and Grain options | Reserve separate symbol and caption regions. Align caption baselines. |
| Centered text | DATE, OFF, and VER | Center the text and use a shared type scale. Allow narrower strings. |
| Label with optional second line | Date format options | Center the whole text block vertically. Use smaller text for the time component. |
| Film cover | Film Looks | Keep its dedicated cover layout and font rules. |

Current root symbols illustrate why one bounding box is insufficient:

- Framing is 26 × 16 pixels.
- Guides is 28 × 18 pixels.
- Film Looks is 28 × 16 pixels.
- Grain and Glow root entries occupy 28 × 28 pixels.

These different proportions communicate different objects. A 28 × 28 circle is
a useful Glow template, not a rule for every symbol. The `LABEL_WIDTH = 28`
constant in the generator limits a text line, not every glyph.

Captions use Roboto Condensed Bold at scale 12 with a shared baseline at `y=32`.
Their raster bounds stay within `y=24..34`, below symbols near `y=6..23`.
Descenders end by `y=34`, leaving rows 35 and 36 clear.
Framing's wide ratios use a 30-pixel width. Date order labels occupy 25 or 26
pixels horizontally. Center the date order and optional time line as one block.

### Spacing rules

- Keep canvas and role-specific frame coordinates fixed.
- Define symbol bounds, caption baseline, and descender allowance in each template.
- For ordinary symbols, start with three to four clear pixels inside the frame.
- For captions, target at least two clear pixels before the bottom border,
  including descenders. Confirm the baseline visually before moving existing text.
- Evaluate geometric centering and optical balance separately. Text asymmetry
  alone does not prove incorrect alignment.
- Review both the final 40-pixel image and a nearest-neighbor enlargement.
  Enlargements expose raster details. The original size determines readability.

Before the revision, the `g` in Guides' Diag and Grain's Strong reached `y=35`.
The revised captions share one font, scale, and baseline.
Their alignment does not depend on the first faint antialiased pixel.

## Glow: preserve the visible contour

Glow uses a shared frame and center. Judge the apparent size at 40 × 40 pixels.
Equal bounding boxes alone do not establish visual consistency: dense white rings
can make a stronger icon appear larger.

The audit measured the brightest outer-ring pixel along 72 directions.
The following medians describe the icons before the revision. Background gray is 51.

| Effect | Weak | Medium | Strong |
| --- | ---: | ---: | ---: |
| Bloom | 83 | 137 | 234.5 |
| Halation | 98 | 174 | 252.5 |

At a gray threshold of 128, both previous weak icons retained only the central
8 × 8 light source. Their outer rings contributed no pixels above that threshold.
The previous Bloom Medium also lost its surrounding rings at threshold 160. These probes
compare images. They are not human-visibility or accessibility standards.

### Strength rules

- Keep the center and central light source fixed.
- Use small, documented optical insets when denser strokes make an icon appear larger.
- Keep the defining rings clearly visible at every strength.
- Increase stroke thickness inward to express strength without increasing diameter.
- Use brightness as a secondary cue within a high-contrast range.
- Define all strengths from one shared source and a small parameter set.
- Do not use near-background rings to represent a weak but enabled effect.

The implemented nominal stroke grays are **224 / 240 / 255** for Weak / Medium / Strong.
Bloom uses inward stroke widths of **1.5 / 1.8 / 2.1 pixels**.
Its outer diameters are **28 / 27 / 26 pixels**, with the stronger contours inset
to compensate for their greater white area.

Halation uses **1.5 / 2.25 / 3.0 pixels**.
Halation keeps its 14-pixel outer radius.
Bloom's middle ring radii are 10.5, 10.25, and 10 pixels. Its inner ring radius stays at 7 pixels.
The central light source has a fixed 4-pixel radius.

The generator averages 16 × 16 samples per pixel and retains binary alpha.
Both weak contours now have a radial peak median of 216 and a minimum of 184.
The weaker contours retain their brightness. Optical compensation must not use dim outer rings to make a dense icon appear smaller.
The generator checks each radial peak against 176 as a regression guard for this
geometry. This is not an accessibility threshold or a rule for unrelated artwork.

Compare radial peak brightness along continuous rings and flag weak sections for
review. Antialiased edge pixels can remain darker. Dotted symbols have intentional
gaps, so the same continuity check does not apply to the Glow root entry.

## Grain: vary density, not particle size

Grain Strength and Grain Size are separate settings. Keep particle radius and
brightness fixed across strength icons. Use 18, 32, 46, and 60 particles for
Weak, Medium, Strong, and Extra. Each level adds particles to the same pattern.

The generator uses a fixed seed and selects dispersed points within a 28 × 18
region. Each particle has a 0.75-pixel radius and nominal gray 240.
This avoids directional stripes and large connected white areas.
The root icon uses the same method in a 28 × 28 region with 70 particles.

Retain the shared caption baseline. Review particle separation and apparent size
at 40 pixels, especially for Extra. Do not enlarge the particle region to express strength.

## Date: show the format, not an example value

Use `YMD`, `MDY`, and `DMY` to identify year, month, and day order.
Do not use example values such as `'26` or `9:05` as status icons.
These values can look like live camera data.

All order labels use Roboto Condensed Bold at scale 16. A single line uses
baseline `y=25`, with visible capitals in rows 15 through 24.

Formats that include time use baseline `y=20` for the date order and `y=30`
for a smaller `HM` line at scale 12. The two-line block spans rows 10 through 29.
Both layouts center on `y=19.5`, the center of the list frame. Keep the two clear
rows between the date and time lines. Do not use a clock pictogram.
The root entry retains its `DATE` label.

## Differences from the sampled native style

| Aspect | Static observation | Design implication |
| --- | --- | --- |
| Frames | GRKit matches the sampled native frame geometry and border gray. | Preserve separate list and toolbar templates. |
| Fill | GRKit uses `(51,51,51)`; sampled native tiles use `(53,53,54)`. | Treat fill changes as a separate palette decision. |
| Symbols | Several native symbols have large solid white areas. GRKit often uses thin diagrams. | Match optical weight without forcing identical bounding boxes. |
| Text | Native Focus and Date text have bright glyphs about 11 pixels high; Std is about 16 pixels high. | Keep type hierarchy. Native text does not have one universal size. |
| Captions | GRKit's diagram captions are smaller than native standalone text tiles. | Preserve caption space and contrast; do not judge them as full-tile labels. |
| Strength | Native ISO NR examples add visible white bars while retaining their layout. | Keep recognizable structure stable as the level changes. |
| Film covers | GRKit uses several text bands and its own cover typography. | Document this as an intentional separate style. |

The native resources support high-contrast structure and stable layout. They do
not establish a universal stroke width, a 28-pixel symbol rule, or binary alpha
for every firmware graphic.

## Source and review workflow

1. Select the role and layout template before drawing.
2. Store reproducible source geometry or drawing parameters alongside its generator.
3. Reuse the existing frame, palette, and typography helpers where applicable.
4. Derive related variants from the same geometry. Change only approved parameters.
5. Export directly to the final 40 × 40 raster with a fixed rendering method.
6. Produce an original-size comparison and a pixel enlargement, including native references.
7. Review visible contour, spacing, caption baseline, and family consistency.
8. After approval, update the PNGs and relevant regression fixtures, then test in the simulator.

Potential automated checks include role-specific frame bounds, binary alpha,
transparent-edge RGB, caption bounds, unused files, and shared variant geometry.
Checks should validate each role separately and preserve the film-cover exception.
Contour brightness can assist review but cannot replace an original-size comparison.

## Regenerate the revised assets

Run from the repository root:

```sh
cargo run --locked -p grkit-core --example module-icons
cargo test --locked -p grkit-core --test effects
```

The [generator](../crates/grkit-core/examples/module-icons.rs) derives Glow, Grain,
and Date variants from shared parameters. It retains the dotted Glow root icon
and the `DATE` root label. It also redraws Framing and Guides captions while
preserving their diagrams. Repeated runs do not change the output.

The other existing module PNGs remain their own source artwork. Future revisions
must provide reproducible geometry or drawing parameters where practical.
