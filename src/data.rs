//! Input data types — the series/slice/point/candle/cell/link structs a caller builds a
//! chart from, plus their constructor helpers.

use crate::config::*;
use pebbles::prelude::*;

/// One series of a [`combo_chart`](crate::combo_chart), tagged with how it draws
/// ([`SeriesKind::Bar`] / `Line` / `Area`) so a single chart can mix mark types.
#[derive(Clone)]
pub struct ComboSeries {
    /// Legend label / tooltip name.
    pub label: String,
    /// One value per category (`NaN` = gap).
    pub values: Vec<f64>,
    /// How this series is drawn (bar, line, or area).
    pub kind: SeriesKind,
    /// Explicit color; `None` takes the next palette color.
    pub color: Option<Color>,
}

/// Create a [`ComboSeries`] drawn as `kind`. See [`combo_chart`](crate::combo_chart).
pub fn combo_series(
    label: impl Into<String>,
    values: impl IntoSeriesValues,
    kind: SeriesKind,
) -> ComboSeries {
    ComboSeries {
        label: label.into(),
        values: values.into_series_values(),
        kind,
        color: None,
    }
}

impl ComboSeries {
    /// Set an explicit color instead of the palette default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A single point for a [`scatter_chart`](crate::scatter_chart) /
/// [`bubble_chart`](crate::bubble_chart): numeric x/y with an optional radius.
#[derive(Clone, Copy)]
pub struct ScatterPoint {
    /// Position on the numeric x-axis.
    pub x: f64,
    /// Position on the value (y) axis.
    pub y: f64,
    /// Mark radius in px (bubble size; a plain scatter uses the default).
    pub radius: f64,
}

/// A scatter point at `(x, y)` with the default radius.
pub fn point(x: f64, y: f64) -> ScatterPoint {
    ScatterPoint { x, y, radius: 3.0 }
}

/// A bubble point at `(x, y)` sized by `radius` (clamped to at least 1px).
pub fn bubble_point(x: f64, y: f64, radius: f64) -> ScatterPoint {
    ScatterPoint {
        x,
        y,
        radius: radius.max(1.0),
    }
}

/// A named set of [`ScatterPoint`]s — one series of a scatter / bubble chart.
#[derive(Clone)]
pub struct PointSeries {
    /// Legend label / tooltip name.
    pub label: String,
    /// The points in this series.
    pub points: Vec<ScatterPoint>,
    /// Explicit color; `None` takes the next palette color.
    pub color: Option<Color>,
}

/// Create a [`PointSeries`] from a set of points. See [`scatter_chart`](crate::scatter_chart).
pub fn point_series(label: impl Into<String>, points: Vec<ScatterPoint>) -> PointSeries {
    PointSeries {
        label: label.into(),
        points,
        color: None,
    }
}

impl PointSeries {
    /// Set an explicit color instead of the palette default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// One OHLC bar for a [`candlestick_chart`](crate::candlestick_chart) /
/// [`ohlc_chart`](crate::ohlc_chart). A candle is drawn green when `close >= open`, red
/// otherwise, with a wick spanning `low..high`.
#[derive(Clone, Copy)]
pub struct Candle {
    /// Opening price.
    pub open: f64,
    /// Session high (top of the wick).
    pub high: f64,
    /// Session low (bottom of the wick).
    pub low: f64,
    /// Closing price.
    pub close: f64,
}

/// Create a [`Candle`] from open / high / low / close.
pub fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        open,
        high,
        low,
        close,
    }
}

/// One cell of a [`heatmap_chart`](crate::heatmap_chart): a value at grid position
/// `(x, y)` (column, row indices).
#[derive(Clone, Copy)]
pub struct HeatCell {
    /// Column index (into the x-category list).
    pub x: usize,
    /// Row index (into the y-category list).
    pub y: usize,
    /// The cell's value (drives its color intensity).
    pub value: f64,
}

/// Create a [`HeatCell`] at column `x`, row `y` with `value`.
pub fn heat_cell(x: usize, y: usize, value: f64) -> HeatCell {
    HeatCell { x, y, value }
}

/// One link of a [`sankey_chart`](crate::sankey_chart): a weighted flow from `source` to
/// `target` node (nodes are created implicitly from the link labels).
#[derive(Clone)]
pub struct SankeyLink {
    /// Source node label.
    pub source: String,
    /// Target node label.
    pub target: String,
    /// Flow magnitude (sets the link thickness).
    pub value: f64,
}

/// Create a [`SankeyLink`] carrying `value` from `source` to `target`.
pub fn sankey_link(source: impl Into<String>, target: impl Into<String>, value: f64) -> SankeyLink {
    SankeyLink {
        source: source.into(),
        target: target.into(),
        value,
    }
}

// ---------------------------------------------------------------------------
// Series / Slice — the generic cartesian + radial inputs
// ---------------------------------------------------------------------------

/// A named data series (one line / one bar color) with a value per category.
#[derive(Clone)]
pub struct Series {
    /// Legend label / tooltip name.
    pub label: String,
    /// One value per category, aligned to the chart's category list (`NaN` = gap).
    pub values: Vec<f64>,
    /// Explicit color; `None` takes the next palette color.
    pub color: Option<Color>,
    /// Optional symmetric ± error / uncertainty per value (same length as `values`); drawn
    /// as an error-bar whisker on each mark. `NaN`/missing entries draw no whisker.
    pub errors: Option<Vec<f64>>,
}

/// Values accepted by [`series`]. `None` becomes a visible gap for line/area charts and
/// an omitted mark for bars/tooltips.
pub trait IntoSeriesValues {
    /// Convert into the internal `Vec<f64>` representation (`None` → `NaN`).
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
        errors: None,
    }
}

/// Create a [`Series`] with optional values; `None` draws a gap / omitted mark.
pub fn series_with_gaps(label: impl Into<String>, values: Vec<Option<f64>>) -> Series {
    series(label, values)
}

/// Create a [`Series`] with a symmetric ± error per value — drawn as error-bar whiskers.
pub fn series_with_errors(
    label: impl Into<String>,
    values: impl IntoSeriesValues,
    errors: Vec<f64>,
) -> Series {
    series(label, values).errors(errors)
}

impl Series {
    /// Set an explicit color instead of the palette default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// Attach a symmetric ± error per value (error-bar whiskers). Length should match
    /// `values`; extra/short entries are ignored, and `NaN` draws no whisker.
    pub fn errors(mut self, errors: Vec<f64>) -> Self {
        self.errors = Some(errors);
        self
    }
}

/// A single wedge of a pie / donut chart.
#[derive(Clone)]
pub struct Slice {
    /// Legend label / slice name.
    pub label: String,
    /// The slice's value (its share of the total sets the wedge angle).
    pub value: f64,
    /// Explicit color; `None` takes the next palette color.
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
    /// Set an explicit color instead of the palette default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
