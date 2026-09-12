//! # pebbles-charts
//!
//! Composable, themeable **chart widgets** for the [Pebbles](https://github.com/pebbles-hq/pebbles)
//! GUI framework — from everyday bars, lines, areas, pie, and donut charts to stacked,
//! scatter, radar, radial, dense, flow, and sparkline variants — drawn on the GPU canvas
//! with a config-driven palette, y-axis values, cartesian tooltips, and an auto legend. The
//! chrome (grid, axis labels) follows the app's `theme()`, so charts match light/dark
//! for free; series colors come from a [`palette()`] you can override per series.
//!
//! ```ignore
//! use pebbles::prelude::*;
//! use pebbles_charts::{bar_chart, series};
//!
//! fn view() -> impl IntoWidget {
//!     bar_chart(
//!         vec!["Jan".into(), "Feb".into(), "Mar".into()],
//!         vec![
//!             series("Desktop", vec![186.0, 305.0, 237.0]),
//!             series("Mobile", vec![80.0, 200.0, 120.0]),
//!         ],
//!     )
//!     .width(520.0)
//!     .height(260.0)
//! }
//! ```
//!
//! ## Where things live
//!
//! The crate is organized concept-per-module so contributors can find code by name:
//!
//! - [`data`] — the input types you build a chart from (`Series`, `Slice`, `ComboSeries`,
//!   `ScatterPoint`, `Candle`, …) and their constructors.
//! - [`config`] — the shared enums (`SeriesKind`, `AxisScale`, `CurveInterpolation`,
//!   `CategoryLabelMode`, `LegendPosition`).
//! - [`style`] — the [`palette()`]/[`cvd_palette`] and the per-slot [`ChartStyle`].
//! - [`overlays`] — reference lines/bands and point annotations.
//! - `scale` / `tooltip` / `legend` / `render` — shared internals (numeric scales +
//!   formatting, the cursor tooltip, the interactive legend, and small draw helpers).
//! - `cartesian` — the bar/line/area family (`mod` = builder, `draw` = painter,
//!   `geometry`/`hit`/`labels` = the math, hit-testing, and label overlays).
//! - `pie`, `scatter`, `radial`, `financial`, `heatmap`, `flow` — the other chart families.
//!
//! Everything a caller needs is re-exported at the crate root (below).

// --- concept modules (shared internals are private to the crate) --------------
pub mod config;
pub mod data;
pub mod overlays;
pub mod style;

mod cartesian;
mod financial;
mod flow;
mod heatmap;
mod legend;
mod pie;
mod radial;
mod render;
mod scale;
mod scatter;
mod tooltip;

// --- the public API surface, in one place -------------------------------------

pub use config::{AxisScale, CategoryLabelMode, CurveInterpolation, LegendPosition, SeriesKind};

pub use data::{
    Candle, ComboSeries, HeatCell, IntoSeriesValues, PointSeries, SankeyLink, ScatterPoint, Series,
    Slice, bubble_point, candle, combo_series, heat_cell, point, point_series, sankey_link, series,
    series_with_errors, series_with_gaps, slice,
};

pub use style::{ChartStyle, cvd_palette, palette, palette_color};

pub use overlays::{
    Annotation, ReferenceBand, ReferenceLine, annotation, reference_band, reference_line,
};

pub use cartesian::{
    AreaChart, BarChart, CartesianChart, ComboChart, HorizontalBarChart, LineChart,
    PercentStackedAreaChart, PercentStackedBarChart, Sparkline, StackedAreaChart, StackedBarChart,
    SteppedLineChart, area_chart, bar_chart, combo_chart, horizontal_bar_chart, line_chart,
    percent_stacked_area_chart, percent_stacked_bar_chart, sparkline, stacked_area_chart,
    stacked_bar_chart, stepped_line_chart,
};

pub use pie::{PieChart, donut_chart, pie_chart};

pub use scatter::{BubbleChart, ScatterChart, bubble_chart, scatter_chart};

pub use radial::{
    GaugeChart, RadarChart, RadialProgressChart, gauge_chart, progress_ring, radar_chart,
};

pub use financial::{CandlestickChart, OhlcChart, candlestick_chart, ohlc_chart};

pub use heatmap::{HeatmapChart, heatmap_chart};

pub use flow::{FunnelChart, SankeyChart, funnel_chart, sankey_chart};
