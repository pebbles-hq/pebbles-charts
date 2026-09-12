# Architecture

This document explains how `pebbles-charts` is built — enough that you can read any chart's
code, follow how data becomes pixels, and extend the crate without surprises. For the
practical workflow (setup, gates, tests) see [`CONTRIBUTING.md`](CONTRIBUTING.md).

## The one-sentence model

A chart is a **builder** you configure, that turns into a **widget** whose paint closure
draws marks on the GPU **canvas**, with text labels layered above it and an accessibility
node wrapped around it.

```
   data types            builder (fluent)          widget tree
   Series/Slice/…  ──▶   bar_chart(..).width(..)  ──▶  Semantics(role, summary, value)
                                                          └─ [LayoutBuilder]        (only if .fill_width)
                                                               └─ component(render_fn)
                                                                    └─ Stack
                                                                         ├─ Canvas(painter)   ← grid, axes, marks
                                                                         └─ label overlays    ← text() widgets
```

Nothing draws until the framework paints; the builder just captures configuration.

## Layers, bottom-up

The crate is one module per concept (`lib.rs` re-exports the public names — that's the whole
API surface in one place).

- **`data`** — the plain input structs a caller builds (`Series`, `Slice`, `ComboSeries`,
  `ScatterPoint`, `Candle`, `HeatCell`, `SankeyLink`) and their constructors. No logic.
- **`config`** — the small enums that flavor a chart (`SeriesKind`, `AxisScale`,
  `CurveInterpolation`, `CategoryLabelMode`, `LegendPosition`).
- **`style`** — the categorical `palette` / `cvd_palette` and the per-slot `ChartStyle`
  (colors, stroke/point/area geometry, label typography). `ChartStyle`'s `pub(crate)`
  accessors resolve each slot to a concrete value, falling back to `theme()`.
- **`scale`** — the numeric core: plot-inset constants, `LinearScale` (a value→pixel affine),
  `ValueAxis` (a "nice" domain + ticks that always includes zero unless overridden),
  `XValueAxis` (the scatter x-axis, incl. log/time), and number formatting.
- **`overlays`** — `ReferenceLine`, `ReferenceBand`, `Annotation`: descriptors a cartesian
  chart draws on top of the plot.
- **`tooltip` / `legend` / `render`** — shared widgets/helpers: the cursor-following tooltip,
  the interactive legend (click a chip to toggle a series/slice), and small draw helpers
  (empty/loading/error panel, label font, on-fill contrast color, category strip).
- **The chart families** — `cartesian/` (the big one), `pie`, `scatter`, `radial`,
  `financial`, `heatmap`, `flow`. Each owns its builder, `IntoWidget`, and painter.

## How a chart renders (the cartesian path in detail)

`cartesian/` is split so no one file is the whole chart:

- **`mod.rs`** — `CartesianChart` (the builder struct + ~40 fluent methods), the
  constructors (`bar_chart`, `line_chart`, …), the internal `Kind` enum, the
  accessibility-text builder, and `impl IntoWidget`.
- **`draw.rs`** — `render_cartesian_chart`: the component function. It reads the builder,
  resolves the style + plot inset, wires interaction + animation, and returns a `Stack` of
  the `Canvas` painter plus the label overlays.
- **`geometry.rs`** — value math (stacking, percent-stacking) and the line/area path builders.
- **`hit.rs`** — pointer hit-testing: which category + series is under a point.
- **`labels.rs`** — the text overlays (axis titles, category labels, y-axis ticks, data
  labels, annotation layer).

`IntoWidget::into_widget` wraps everything:

1. Compute the a11y `(role, summary, value)` from the data (`cartesian_a11y`).
2. Build the plot as `component_props(render_cartesian_chart, self)` — a **component**, so
   its per-widget signals (active datum, animation progress, hidden set) get their own scope.
3. If `.fill_width(true)`, wrap that in a `layout_builder` that rebuilds at the parent's
   width each layout pass (that's how responsive + auto-resize work).
4. Wrap the whole thing in a `Semantics` node so a screen reader announces the chart.

`render_cartesian_chart` then:

1. **Early-outs** for aspect-ratio (derive height), `.error(..)`, `.loading(..)`, and the
   empty state — each returns a centered panel.
2. Resolves the **style** (`ChartStyle` → concrete colors/geometry/typography) and the
   **plot inset** `pad` (one `EdgeInsets`, defaulting to `10/12/10/8`).
3. Sets up **reactive state**: the active (hovered/focused) datum, the entry-animation
   progress, the data-change tween snapshots, and the legend's hidden-series set.
4. Computes the **visible** subset (series not toggled off in the legend), the **animated**
   values (morphed toward the target by the tween), the **axis** (nice domain + ticks,
   expanded to fit error bars), and the tick/category labels.
5. Emits a `canvas(|c| …)` painter that draws — in order — reference bands, grid, zero
   baseline, reference lines (dashed), the marks (bar/line/area per `Kind`), error-bar
   whiskers, and the fading exit ghost. The entry animation clips the marks to a
   left-to-right wipe.
6. Stacks the label overlays and data labels above the canvas, wires a `GestureDetector`
   (hover/tap → hit-test → tooltip + active band) and keyboard traversal, and assembles the
   y-axis gutter, plot, axis titles, category-label row, and legend into the final column.

The other families are simpler versions of the same shape: a builder, an `IntoWidget` that
wraps a `canvas()` painter (plus, for pie, an interactive sub-component for hover pop-out),
and reuse of `scale`/`legend`/`tooltip`/`render`.

## Invariants (break these and things drift)

- **One plot inset, one scale.** The draw, hit-testing, and every label overlay must use the
  same `pad` and the same value scale — otherwise labels fan away from their marks or the
  tooltip lands on the wrong datum. Plot geometry is threaded, not recomputed independently
  (there's a regression test asserting a custom `.plot_padding` keeps hit-testing aligned).
- **Never panic on user data.** A chart handed mismatched lengths, `NaN`/inf, negatives,
  empty data, or a zero size renders calmly. In practice: prefer `.get(i)` and iteration
  over raw indexing, avoid `unwrap`, and guard every `clamp` upper bound with `.max(lo)`.
  `tests/robustness.rs` paints a battery of malformed inputs for every family.
- **Colors come from the theme or the palette**, never hardcoded — chrome uses
  `theme().colors`; series/slices use the palette or a `.color(..)` override.
- **The canvas draws no text.** Labels are `text()` widgets in the `Stack` above the plot,
  gated to appear only after the entry animation settles.
- **`bd` … n/a here** — this crate has no `unsafe` (`#![forbid(unsafe_code)]`) and every
  public item is documented (`#![deny(missing_docs)]`); both are compiler-enforced.

## Animation model

All animation is a `0→1` progress signal kicked once and advanced by the framework clock:

- **Entry** — marks wipe in left-to-right (cartesian) or sweep radially (pie) on mount.
- **Data-change tween** — when values change on a re-render (same shape), marks morph from
  the previous values to the new ones; snapshots are keyed by original index so a legend
  toggle isn't mistaken for a data change.
- **Series enter/exit** — an added series re-runs the reveal; a removed one (matched by
  label) fades out as a ghost of its real last values for one cycle.
- **Reduced motion** — `.animate` defaults to the OS `prefers-reduced-motion` setting
  (`pebbles_core::prefers_reduced_motion`); `.animate(true|false)` forces it.

Labels and data read-outs always reflect the *target* values, never the mid-animation ones.

## Rendering & export

Charts paint via the framework `Canvas` (solid + gradient fills, solid + dashed strokes) on
whichever Vello backend the app builds (`vello-hybrid` by default). Text is GPU-rendered by
the framework. A chart is a widget, so it can't rasterize itself — **export** goes through
the framework's offscreen capture (`pebbles::shell::capture::capture_png`), which renders any
widget headlessly on an offscreen GPU target.

## What is intentionally out of scope

- **SVG export** — would need a parallel vector renderer alongside the GPU canvas; low ROI.
  PNG (via capture) covers the export use case.
- **Automated visual-regression (golden-image) tests** — output is verified with headless
  renders during review; the automated suite guards correctness and panic-safety, not pixel
  drift (which would require a GPU CI runner).
