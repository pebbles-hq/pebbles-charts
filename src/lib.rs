//! # pebbles-charts
//!
//! Composable, themeable **chart widgets** for the [Pebbles](https://github.com/pebbles-hq/pebbles)
//! GUI framework — from everyday bars, lines, areas, pie, and donut charts to stacked,
//! scatter, radar, radial, dense, flow, and sparkline variants — drawn on the GPU canvas
//! with a config-driven palette, y-axis values, cartesian tooltips, and an auto legend. The
//! chrome (grid, axis labels) follows the app's `theme()`, so charts match light/dark
//! for free; series colors come from a [`palette`] you can override per series.
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

mod charts;

pub use charts::{
    AreaChart, AxisScale, BarChart, BubbleChart, Candle, CandlestickChart, CategoryLabelMode,
    ComboChart, ComboSeries, CurveInterpolation, FunnelChart, GaugeChart, HeatCell, HeatmapChart,
    HorizontalBarChart, LegendPosition, LineChart, OhlcChart, PercentStackedAreaChart,
    PercentStackedBarChart,
    PieChart, PointSeries, RadarChart, RadialProgressChart, ReferenceBand, ReferenceLine,
    SankeyChart, SankeyLink, ScatterChart, ScatterPoint, SeriesKind, Sparkline, StackedAreaChart,
    StackedBarChart, SteppedLineChart, area_chart, bar_chart, bubble_chart, bubble_point, candle,
    candlestick_chart, combo_chart, combo_series, donut_chart, funnel_chart, gauge_chart,
    heat_cell, heatmap_chart, horizontal_bar_chart, line_chart, ohlc_chart,
    percent_stacked_area_chart, percent_stacked_bar_chart, pie_chart, point, point_series,
    progress_ring, radar_chart, reference_band, reference_line, sankey_chart, sankey_link,
    scatter_chart, sparkline, stacked_area_chart, stacked_bar_chart, stepped_line_chart,
};

use pebbles::prelude::*;

/// A named data series (one line / one bar color) with a value per category.
#[derive(Clone)]
pub struct Series {
    pub label: String,
    pub values: Vec<f64>,
    pub color: Option<Color>,
}

/// Values accepted by [`series`]. `None` becomes a visible gap for line/area charts and
/// an omitted mark for bars/tooltips.
pub trait IntoSeriesValues {
    fn into_series_values(self) -> Vec<f64>;
}

impl IntoSeriesValues for Vec<f64> {
    fn into_series_values(self) -> Vec<f64> {
        self
    }
}

impl IntoSeriesValues for Vec<Option<f64>> {
    fn into_series_values(self) -> Vec<f64> {
        self.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect()
    }
}

/// Create a [`Series`]. Give it a color with [`Series::color`], or let the palette pick.
pub fn series(label: impl Into<String>, values: impl IntoSeriesValues) -> Series {
    Series {
        label: label.into(),
        values: values.into_series_values(),
        color: None,
    }
}

/// Create a [`Series`] with optional values; `None` draws a gap / omitted mark.
pub fn series_with_gaps(label: impl Into<String>, values: Vec<Option<f64>>) -> Series {
    series(label, values)
}

impl Series {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A single wedge of a pie / donut chart.
#[derive(Clone)]
pub struct Slice {
    pub label: String,
    pub value: f64,
    pub color: Option<Color>,
}

/// Create a [`Slice`].
pub fn slice(label: impl Into<String>, value: f64) -> Slice {
    Slice {
        label: label.into(),
        value,
        color: None,
    }
}

impl Slice {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// The default categorical palette (distinct, legible on light and dark). Override any
/// series/slice with `.color(..)`. Indexed cyclically.
pub fn palette() -> [Color; 6] {
    let c = |r, g, b| Color::from_rgba8(r, g, b, 0xFF);
    [
        c(0x63, 0x66, 0xF1), // indigo
        c(0x2D, 0xD4, 0xBF), // teal
        c(0xF5, 0x9E, 0x0B), // amber
        c(0xF4, 0x3F, 0x5E), // rose
        c(0x8B, 0x5C, 0xF6), // violet
        c(0x22, 0xC5, 0x5E), // green
    ]
}

/// The color for series/slice `i` (palette, cycled).
pub fn palette_color(i: usize) -> Color {
    let p = palette();
    p[i % p.len()]
}

/// A colorblind-safe categorical palette (Okabe–Ito, the widely-used CVD-safe ramp —
/// distinguishable under protanopia/deuteranopia/tritanopia). Pass it to a chart with
/// `.palette(cvd_palette().to_vec())`.
pub fn cvd_palette() -> [Color; 7] {
    let c = |r, g, b| Color::from_rgba8(r, g, b, 0xFF);
    [
        c(0x00, 0x72, 0xB2), // blue
        c(0xE6, 0x9F, 0x00), // orange
        c(0x00, 0x9E, 0x73), // bluish green
        c(0xCC, 0x79, 0xA7), // reddish purple
        c(0x56, 0xB4, 0xE9), // sky blue
        c(0xD5, 0x5E, 0x00), // vermillion
        c(0xF0, 0xE4, 0x42), // yellow
    ]
}

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    let [r, g, b, _] = c.components;
    Color::new([r, g, b, a])
}

/// A per-slot visual style for a chart — colors, stroke/point/area geometry, label typography.
///
/// Every field is optional: `None` falls back to the theme-derived default (grid/label from
/// `theme().colors.muted_foreground`, `line_width = 2.0`, `point_radius = 2.6`,
/// `area_alpha = 0.18`, `bar_radius = 4.0`, `label_size = 11.0`, the app's default font).
/// Build one fluently and pass it with `.style(..)` to make a chart fully themeable beyond
/// the palette — the "theme-as-config" surface. Default = every slot at the theme default.
#[derive(Clone, Debug, Default)]
pub struct ChartStyle {
    /// Grid-line color (default `muted_foreground` @ 16% alpha).
    pub grid_color: Option<Color>,
    /// Zero-baseline line color (default `muted_foreground` @ 34%).
    pub zero_line_color: Option<Color>,
    /// Reference/target line + band color (default `muted_foreground` @ 56%).
    pub reference_color: Option<Color>,
    /// Axis / category / data-label text color (default `muted_foreground`).
    pub label_color: Option<Color>,
    /// Line / area stroke width in px (default 2.0).
    pub line_width: Option<f64>,
    /// Line/scatter point radius in px (default 2.6; the active point is larger).
    pub point_radius: Option<f64>,
    /// Area fill alpha for flat (non-gradient) fills (default 0.18).
    pub area_alpha: Option<f32>,
    /// Max rounded-corner radius for bars in px (default 4.0).
    pub bar_radius: Option<f64>,
    /// Axis/category/data-label font size in px (default 11.0).
    pub label_size: Option<f64>,
    /// Font family for all chart labels (default: the app's default font).
    pub font_family: Option<String>,
}

impl ChartStyle {
    /// A style with every slot at its theme default (same as [`ChartStyle::default`]).
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the grid-line color.
    pub fn grid_color(mut self, c: Color) -> Self {
        self.grid_color = Some(c);
        self
    }
    /// Set the zero-baseline line color.
    pub fn zero_line_color(mut self, c: Color) -> Self {
        self.zero_line_color = Some(c);
        self
    }
    /// Set the reference/target line + band color.
    pub fn reference_color(mut self, c: Color) -> Self {
        self.reference_color = Some(c);
        self
    }
    /// Set the axis / category / data-label text color.
    pub fn label_color(mut self, c: Color) -> Self {
        self.label_color = Some(c);
        self
    }
    /// Set the line/area stroke width (px).
    pub fn line_width(mut self, w: f64) -> Self {
        self.line_width = Some(w);
        self
    }
    /// Set the line/scatter point radius (px).
    pub fn point_radius(mut self, r: f64) -> Self {
        self.point_radius = Some(r);
        self
    }
    /// Set the flat area-fill alpha (0.0..1.0).
    pub fn area_alpha(mut self, a: f32) -> Self {
        self.area_alpha = Some(a);
        self
    }
    /// Set the max rounded-corner radius for bars (px).
    pub fn bar_radius(mut self, r: f64) -> Self {
        self.bar_radius = Some(r);
        self
    }
    /// Set the axis/category/data-label font size (px).
    pub fn label_size(mut self, px: f64) -> Self {
        self.label_size = Some(px);
        self
    }
    /// Set the font family for all chart labels.
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    // --- resolved accessors (theme default when unset) ---
    pub(crate) fn grid(&self) -> Color {
        self.grid_color.unwrap_or_else(|| with_alpha(theme().colors.muted_foreground, 0.16))
    }
    pub(crate) fn zero_line(&self) -> Color {
        self.zero_line_color.unwrap_or_else(|| with_alpha(theme().colors.muted_foreground, 0.34))
    }
    pub(crate) fn reference(&self) -> Color {
        self.reference_color.unwrap_or_else(|| with_alpha(theme().colors.muted_foreground, 0.56))
    }
    pub(crate) fn label(&self) -> Color {
        self.label_color.unwrap_or_else(|| theme().colors.muted_foreground)
    }
    pub(crate) fn line_w(&self) -> f64 {
        self.line_width.unwrap_or(2.0)
    }
    pub(crate) fn point_r(&self) -> f64 {
        self.point_radius.unwrap_or(2.6)
    }
    pub(crate) fn area_a(&self) -> f32 {
        self.area_alpha.unwrap_or(0.18)
    }
    pub(crate) fn bar_r(&self) -> f64 {
        self.bar_radius.unwrap_or(4.0)
    }
    pub(crate) fn label_px(&self) -> f32 {
        self.label_size.unwrap_or(11.0) as f32
    }
}
