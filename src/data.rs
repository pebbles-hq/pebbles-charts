//! Input data types: the series/slice/point/candle/cell/link structs a caller builds a chart from.

use crate::config::*;
use pebbles::prelude::*;

#[derive(Clone)]
pub struct ComboSeries {
    pub label: String,
    pub values: Vec<f64>,
    pub kind: SeriesKind,
    pub color: Option<Color>,
}

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
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Clone, Copy)]
pub struct ScatterPoint {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

pub fn point(x: f64, y: f64) -> ScatterPoint {
    ScatterPoint { x, y, radius: 3.0 }
}

pub fn bubble_point(x: f64, y: f64, radius: f64) -> ScatterPoint {
    ScatterPoint {
        x,
        y,
        radius: radius.max(1.0),
    }
}

#[derive(Clone)]
pub struct PointSeries {
    pub label: String,
    pub points: Vec<ScatterPoint>,
    pub color: Option<Color>,
}

pub fn point_series(label: impl Into<String>, points: Vec<ScatterPoint>) -> PointSeries {
    PointSeries {
        label: label.into(),
        points,
        color: None,
    }
}

impl PointSeries {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        open,
        high,
        low,
        close,
    }
}

#[derive(Clone, Copy)]
pub struct HeatCell {
    pub x: usize,
    pub y: usize,
    pub value: f64,
}

pub fn heat_cell(x: usize, y: usize, value: f64) -> HeatCell {
    HeatCell { x, y, value }
}

#[derive(Clone)]
pub struct SankeyLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

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
    pub label: String,
    pub values: Vec<f64>,
    pub color: Option<Color>,
    /// Optional symmetric ± error / uncertainty per value (same length as `values`); drawn
    /// as an error-bar whisker on each mark. `NaN`/missing entries draw no whisker.
    pub errors: Option<Vec<f64>>,
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
