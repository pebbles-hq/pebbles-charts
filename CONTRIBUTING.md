# Contributing to pebbles-charts

Thanks for helping! This crate is the chart-widget library for the
[Pebbles](https://github.com/pebbles-hq/pebbles) GUI framework. It's small and organized so
you can find things by name and land changes with confidence.

## Setup

You need a recent stable Rust (edition 2024; `rust-version = 1.90`). The crate depends on
`pebbles` as a git dependency, so the first build fetches and compiles the framework (and
its `wgpu`/`vello` stack) — expect a few minutes the first time, fast after that.

```sh
git clone https://github.com/pebbles-hq/pebbles-charts
cd pebbles-charts
cargo build
```

## Where the code lives

The crate is one module per concept. `lib.rs` is a thin hub: crate docs, the module tree,
and the public re-exports (the whole API surface in one place).

| Path | What's in it |
|---|---|
| `src/data.rs` | Input types: `Series`, `Slice`, `ComboSeries`, `ScatterPoint`, `Candle`, `HeatCell`, `SankeyLink` + constructors |
| `src/config.rs` | Enums: `SeriesKind`, `AxisScale`, `CurveInterpolation`, `CategoryLabelMode`, `LegendPosition` |
| `src/style.rs` | `palette` / `cvd_palette` and the per-slot `ChartStyle` |
| `src/overlays.rs` | `ReferenceLine`, `ReferenceBand`, `Annotation` |
| `src/scale.rs` | Plot constants, `LinearScale` / `ValueAxis` / `XValueAxis`, ticks, number formatting |
| `src/tooltip.rs` `src/legend.rs` `src/render.rs` | Shared internals: cursor tooltip, interactive legend, small draw helpers |
| `src/cartesian/` | The bar/line/area family: `mod.rs` (builder + constructors), `draw.rs` (painter), `geometry.rs` (math), `hit.rs` (hit-testing), `labels.rs` (label overlays) |
| `src/pie.rs` `scatter.rs` `radial.rs` `financial.rs` `heatmap.rs` `flow.rs` | The other chart families |

Charts draw on the framework `Canvas` (solid/gradient fills, solid/dashed strokes) and are
GPU-rendered. Text is not drawn on the canvas — labels are `text()` widgets layered above
the plot in a `stack`.

## The rules (what CI enforces)

Run these before opening a PR — they are exactly what CI runs:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps            # with RUSTDOCFLAGS="-D warnings"
```

Plus a few conventions that keep the crate honest:

- **Every public item is documented.** `#![deny(missing_docs)]` will fail the build
  otherwise. A one-line doc that says what the thing is / does is enough.
- **Never panic on user data.** This is a *data* library — a chart handed mismatched
  lengths, `NaN`/inf, negatives, empty data, or a zero size must render calmly, not crash.
  Add a case to `tests/robustness.rs` when you touch drawing/layout code. (Avoid `unwrap`,
  and guard every `clamp`/index against degenerate input.)
- **Keep the geometry consistent.** The draw, hit-testing, and every label overlay share
  one plot inset (`pad`) and one scale — if you change plot geometry, thread the same value
  through all of them (there's a regression test for it).
- **Version-less colors from the theme.** Chrome (grid, labels) uses `theme().colors`;
  series colors come from the palette or a `.color(..)` override. Don't hardcode colors.

## Tests

```sh
cargo test                     # unit + a11y + robustness + features (no GPU needed)
cargo test -- --ignored        # GPU tests: PNG export + widget capture (needs a GPU)
```

- `tests/robustness.rs` — malformed-input panic-safety, across every chart family.
- `tests/a11y.rs` — screen-reader nodes (role + data read-out).
- `tests/features.rs` — style / aspect / states / error-bars / annotations smoke tests.
- `tests/export.rs` — chart → PNG via the framework capture API (`#[ignore]`, needs a GPU).
- Cartesian internals (scales, hit-testing, labels) are unit-tested in `src/cartesian/mod.rs`.

## Seeing your changes

The demo app shows one card per chart family:

```sh
cargo run -p demo
# headless screenshot (no display; renders the static, animation-free state):
PEBBLES_REDUCED_MOTION=1 SHOT=1180:2400:/tmp/charts.rgba cargo run -p demo --release
```

## Adding a new chart family

1. Add its input type(s) to `src/data.rs` (if not reusing `Series`/`Slice`).
2. Create `src/<family>.rs` with the chart struct, its constructor(s), builder methods, and
   an `impl IntoWidget`. Reuse `scale`, `legend`, `tooltip`, and `render` helpers.
3. Re-export the public names from `lib.rs`.
4. Add a card to `examples/demo` and cases to `tests/robustness.rs`.
5. Emit an accessibility node (see how `pie.rs` / `cartesian` wrap the chart in `semantics`).

Questions or ideas? Open an issue — small, focused PRs are easiest to review.
