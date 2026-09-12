# Changelog

All notable changes to `pebbles-charts` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[semantic versioning](https://semver.org/) (pre-1.0: any `0.x` release may break).

## [Unreleased]

The crate is feature-complete for its scope and used within the Pebbles ecosystem via a
git dependency; it is not yet published to crates.io.

### Charts
- Cartesian: bar, stacked/100%-stacked bar, horizontal bar, line, stepped line, area,
  stacked/100%-stacked area, combo, sparkline.
- Radial: pie, donut, radar, progress ring, gauge.
- Specialized: scatter, bubble, candlestick, OHLC, heatmap, funnel, sankey.

### Features
- Config-driven `palette` + colorblind-safe `cvd_palette`; per-chart `.palette(..)` override.
- Per-slot `ChartStyle` (colors, stroke/point/area geometry, label font + size) via `.style(..)`.
- A real value axis with nice ticks, negative baselines, log/time x-scales, value formatters,
  axis titles, a secondary right axis, reference lines/bands, and data labels.
- Interaction: cursor-following tooltips + crosshair, click callbacks, an interactive legend
  (click to toggle a series/slice), and keyboard category traversal.
- Animation: entry reveal, data-change tweens, per-series enter/exit, auto reduced-motion.
- Gradient area fills, dashed reference lines, error bars, point annotations.
- Layout: explicit sizing, aspect-ratio lock, responsive fill-parent + auto-resize,
  configurable plot padding.
- States: empty / loading / error placeholders.
- Accessibility: each chart emits a screen-reader node (role + spoken summary + a per-point
  data read-out).
- PNG export via the framework's offscreen capture (`pebbles::shell::capture`).

### Quality
- No panics on malformed input (mismatched lengths, `NaN`/inf, negatives, degenerate sizes)
  — guarded by `tests/robustness.rs`.
- Every public item is documented (`#![deny(missing_docs)]`).
- CI enforces `fmt`, `clippy -D warnings`, tests, and `doc -D warnings`.
