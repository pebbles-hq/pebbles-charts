# Changelog

All notable changes to `pebbles-charts` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[semantic versioning](https://semver.org/) (pre-1.0: any `0.x` release may break).

## [Unreleased]

### Changed — the framework renders with wgpu only
Requires pebbles ≥ `5a9fdbd` (wgpu is the only renderer and cosmic-text the only text stack).
The renderer-selection features of this crate (and its demo) are gone: depend on
it with default features. Nothing else changes for users; the demo capture uses the public offscreen API.

**Migration:** none for users of default features.

### Changed — demo screenshots use the framework's offscreen renderer
`SHOT=` now calls the framework's `pebbles::shell::capture::capture_rgba` (settled frame,
system fonts — same as the app); the demo no longer drives a renderer itself.

### Changed — demo follows the system light/dark setting
The demo no longer pins `Theme::light()`: it starts in the framework's default
`ThemeMode::System` and has a Light/Dark toggle (`toggle_theme`). Titles take the
theme's ambient text colour instead of a hard-coded `.color(foreground)`. `SHOT=`
captures stay light (`set_theme_mode(ThemeMode::Light)`) so docs screenshots are stable.

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
