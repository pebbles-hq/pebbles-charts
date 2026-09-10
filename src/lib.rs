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

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    let [r, g, b, _] = c.components;
    Color::new([r, g, b, a])
}
