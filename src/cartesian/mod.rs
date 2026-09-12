//! The cartesian chart family (bar / line / area and their variants): the `CartesianChart` builder, its constructors, and accessibility text. The painter lives in `draw`, geometry in `geometry`, hit-testing in `hit`, and label overlays in `labels`.

use crate::cartesian::{draw::*, geometry::*, hit::*, labels::*};
use crate::config::*;
use crate::data::*;
use crate::overlays::*;
use crate::scale::*;
use crate::style::*;
use pebbles::prelude::*;
use std::rc::Rc;

pub(crate) mod draw;
pub(crate) mod geometry;
pub(crate) mod hit;
pub(crate) mod labels;

/// A click handler for a cartesian datum: `(category_index, series_index, value)`.
pub(crate) type PointCallback = Rc<dyn Fn(usize, usize, f64)>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Bar,
    StackedBar,
    PercentStackedBar,
    HorizontalBar,
    Line,
    SteppedLine,
    Area,
    StackedArea,
    PercentStackedArea,
    Combo,
    Sparkline,
}

/// A bar / line / area chart over labelled categories. Built with [`bar_chart`],
/// [`line_chart`], or [`area_chart`].
#[derive(Clone)]
pub struct CartesianChart {
    kind: Kind,
    categories: Vec<String>,
    series: Vec<Series>,
    width: f64,
    height: f64,
    legend: bool,
    grid: bool,
    y_axis: bool,
    y_range: Option<(f64, f64)>,
    right_y_axis: Option<ValueAxis>,
    right_y_title: Option<String>,
    tick_count: usize,
    value_formatter: Option<Rc<dyn Fn(f64) -> String>>,
    right_value_formatter: Option<Rc<dyn Fn(f64) -> String>>,
    tooltip: bool,
    on_point: Option<PointCallback>,
    combo_kinds: Vec<SeriesKind>,
    category_window: Option<(usize, usize)>,
    curve: CurveInterpolation,
    vertical_grid: bool,
    x_axis_title: Option<String>,
    y_axis_title: Option<String>,
    category_label_mode: CategoryLabelMode,
    reference_lines: Vec<ReferenceLine>,
    reference_bands: Vec<ReferenceBand>,
    annotations: Vec<Annotation>,
    data_labels: bool,
    legend_position: LegendPosition,
    legend_values: bool,
    // None = auto (honor the OS reduced-motion preference); Some(_) = explicit override.
    animate: Option<bool>,
    animation_ms: u32,
    palette: Option<Vec<Color>>,
    area_gradient: bool,
    a11y_label: Option<String>,
    style: ChartStyle,
    aspect_ratio: Option<f64>,
    loading: bool,
    error: Option<String>,
    fill_width: bool,
    plot_padding: Option<EdgeInsets>,
}

/// Alias — a bar chart. See [`bar_chart`].
pub type BarChart = CartesianChart;
/// Alias — a stacked bar chart. See [`stacked_bar_chart`].
pub type StackedBarChart = CartesianChart;
/// Alias — a 100% stacked bar chart. See [`percent_stacked_bar_chart`].
pub type PercentStackedBarChart = CartesianChart;
/// Alias — a horizontal bar chart. See [`horizontal_bar_chart`].
pub type HorizontalBarChart = CartesianChart;
/// Alias — a line chart. See [`line_chart`].
pub type LineChart = CartesianChart;
/// Alias — a stepped line chart. See [`stepped_line_chart`].
pub type SteppedLineChart = CartesianChart;
/// Alias — an area chart. See [`area_chart`].
pub type AreaChart = CartesianChart;
/// Alias — a stacked area chart. See [`stacked_area_chart`].
pub type StackedAreaChart = CartesianChart;
/// Alias — a 100% stacked area chart. See [`percent_stacked_area_chart`].
pub type PercentStackedAreaChart = CartesianChart;
/// Alias — a combo chart. See [`combo_chart`].
pub type ComboChart = CartesianChart;
/// Alias — a sparkline. See [`sparkline`].
pub type Sparkline = CartesianChart;

pub(crate) fn cartesian(
    kind: Kind,
    categories: Vec<String>,
    series: Vec<Series>,
) -> CartesianChart {
    CartesianChart {
        kind,
        categories,
        series,
        width: 520.0,
        height: 260.0,
        legend: true,
        grid: true,
        y_axis: true,
        y_range: None,
        right_y_axis: None,
        right_y_title: None,
        tick_count: 5,
        value_formatter: None,
        right_value_formatter: None,
        tooltip: true,
        on_point: None,
        combo_kinds: Vec::new(),
        category_window: None,
        curve: CurveInterpolation::Linear,
        vertical_grid: false,
        x_axis_title: None,
        y_axis_title: None,
        category_label_mode: CategoryLabelMode::Auto,
        reference_lines: Vec::new(),
        reference_bands: Vec::new(),
        annotations: Vec::new(),
        data_labels: false,
        legend_position: LegendPosition::Bottom,
        legend_values: false,
        animate: None,
        animation_ms: 600,
        palette: None,
        area_gradient: false,
        a11y_label: None,
        style: ChartStyle::new(),
        aspect_ratio: None,
        loading: false,
        error: None,
        fill_width: false,
        plot_padding: None,
    }
}

/// A grouped **bar chart**.
pub fn bar_chart(categories: Vec<String>, series: Vec<Series>) -> BarChart {
    cartesian(Kind::Bar, categories, series)
}
/// A **stacked bar chart**.
pub fn stacked_bar_chart(categories: Vec<String>, series: Vec<Series>) -> StackedBarChart {
    cartesian(Kind::StackedBar, categories, series)
}
/// A **100% stacked bar chart**.
pub fn percent_stacked_bar_chart(
    categories: Vec<String>,
    series: Vec<Series>,
) -> PercentStackedBarChart {
    cartesian(Kind::PercentStackedBar, categories, series).y_range(0.0, 100.0)
}
/// A **horizontal bar chart**.
pub fn horizontal_bar_chart(categories: Vec<String>, series: Vec<Series>) -> HorizontalBarChart {
    cartesian(Kind::HorizontalBar, categories, series)
}
/// A **line chart**.
pub fn line_chart(categories: Vec<String>, series: Vec<Series>) -> LineChart {
    cartesian(Kind::Line, categories, series)
}
/// A **stepped line chart**.
pub fn stepped_line_chart(categories: Vec<String>, series: Vec<Series>) -> SteppedLineChart {
    cartesian(Kind::SteppedLine, categories, series).curve(CurveInterpolation::Step)
}
/// An **area chart** (filled line).
pub fn area_chart(categories: Vec<String>, series: Vec<Series>) -> AreaChart {
    cartesian(Kind::Area, categories, series)
}
/// A **stacked area chart**.
pub fn stacked_area_chart(categories: Vec<String>, series: Vec<Series>) -> StackedAreaChart {
    cartesian(Kind::StackedArea, categories, series)
}
/// A **100% stacked area chart**.
pub fn percent_stacked_area_chart(
    categories: Vec<String>,
    series: Vec<Series>,
) -> PercentStackedAreaChart {
    cartesian(Kind::PercentStackedArea, categories, series).y_range(0.0, 100.0)
}
/// A mixed **bar + line + area** chart.
pub fn combo_chart(categories: Vec<String>, series: Vec<ComboSeries>) -> ComboChart {
    let kinds = series.iter().map(|s| s.kind).collect();
    let mapped = series
        .into_iter()
        .map(|s| Series {
            label: s.label,
            values: s.values,
            color: s.color,
            errors: None,
        })
        .collect();
    let mut chart = cartesian(Kind::Combo, categories, mapped);
    chart.combo_kinds = kinds;
    chart
}
/// A compact chrome-less line chart.
pub fn sparkline(values: Vec<f64>) -> Sparkline {
    let categories = (0..values.len()).map(|i| i.to_string()).collect();
    cartesian(
        Kind::Sparkline,
        categories,
        vec![Series {
            label: String::new(),
            values,
            color: None,
            errors: None,
        }],
    )
    .height(64.0)
    .legend(false)
    .grid(false)
    .y_axis(false)
    .tooltip(false)
}

impl CartesianChart {
    /// Set the plot width in px (also the fallback width under `.fill_width(true)`).
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    /// Set the plot height in px.
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
    /// Show the series legend (default true; auto-hidden for a single unlabelled series).
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
    /// Place the legend `Bottom` (default), `Top`, `Left`, or `Right` of the plot.
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }
    /// Show each series' total next to its name in the legend.
    pub fn legend_values(mut self, on: bool) -> Self {
        self.legend_values = on;
        self
    }
    /// Animate the marks in on mount (a left-to-right reveal) and tween on data change.
    /// By default this **auto-honors the OS reduced-motion preference** (animations off
    /// when the user asked to minimize motion); call `.animate(true)` to force it on or
    /// `.animate(false)` to force it off regardless.
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = Some(on);
        self
    }
    /// Entry-animation duration in milliseconds (default 600).
    pub fn animation_ms(mut self, ms: u32) -> Self {
        self.animation_ms = ms;
        self
    }
    /// Override the categorical palette for this chart (cycled per series). Pass
    /// [`cvd_palette`](crate::cvd_palette)`().to_vec()` for a colorblind-safe ramp, or any
    /// custom `Vec<Color>`. Per-series `.color(..)` still wins over the palette.
    pub fn palette(mut self, colors: Vec<Color>) -> Self {
        self.palette = if colors.is_empty() {
            None
        } else {
            Some(colors)
        };
        self
    }
    /// Fill area/stacked-area series with a **vertical gradient** (the series color at the
    /// top fading to transparent at the baseline) instead of a flat translucent fill.
    /// No effect on non-area charts. Default off.
    pub fn area_gradient(mut self, on: bool) -> Self {
        self.area_gradient = on;
        self
    }
    /// Set the accessible summary a screen reader announces for this chart (e.g.
    /// "Monthly revenue by channel"). If unset, a summary is generated from the chart
    /// type, series, and categories. The per-point data is always read out as the node's
    /// value regardless, so the chart is never a silent blank to assistive tech.
    pub fn a11y_label(mut self, label: impl Into<String>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }
    /// Override the per-slot visual style (grid/zero/reference/label colors, stroke width,
    /// point radius, area alpha, bar radius, label size + font) — the theme-as-config
    /// surface. See [`ChartStyle`](crate::ChartStyle). Unset slots keep the theme default.
    pub fn style(mut self, style: ChartStyle) -> Self {
        self.style = style;
        self
    }
    /// Lock the chart to an aspect ratio (width ÷ height). When set, the height is derived
    /// from the width (`height = width / ratio`), so the chart keeps its shape as the width
    /// changes. Overrides `.height(..)`.
    pub fn aspect_ratio(mut self, ratio: f64) -> Self {
        self.aspect_ratio = if ratio > 0.0 { Some(ratio) } else { None };
        self
    }
    /// Fill the parent's available width instead of using a fixed `.width(..)`, and
    /// **re-layout automatically** when the container resizes. Pair with `.aspect_ratio(..)`
    /// to derive the height, or keep `.height(..)` for a fixed height. `.width(..)` becomes
    /// the fallback when the parent is unbounded.
    pub fn fill_width(mut self, on: bool) -> Self {
        self.fill_width = on;
        self
    }
    /// Override the internal plot inset (space between the axes and the plotted marks).
    /// Defaults to `10/12/10/8` (l/t/r/b). Increase it to give long value labels or tall
    /// data-labels more room. The draw, hit-testing, and every overlay share this value, so
    /// alignment stays correct.
    pub fn plot_padding(mut self, padding: EdgeInsets) -> Self {
        self.plot_padding = Some(padding);
        self
    }
    /// Show a loading placeholder (a "Loading…" panel at the chart's footprint) instead of
    /// the plot — for data that hasn't arrived yet. Distinct from the empty state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// Show an error placeholder with `message` instead of the plot — for a failed load.
    /// Distinct from the empty ("No data") state.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }
    /// Draw horizontal grid lines (default true).
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }
    /// Show y-axis value labels beside the plot (default true).
    pub fn y_axis(mut self, on: bool) -> Self {
        self.y_axis = on;
        self
    }
    /// Override the computed y-domain exactly.
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }
    /// Show a secondary right-side y-axis with an exact domain.
    pub fn right_y_axis(mut self, min: f64, max: f64) -> Self {
        self.right_y_axis = Some(ValueAxis::from_values(
            &[],
            Some((min, max)),
            self.tick_count,
        ));
        self
    }
    /// Set a title for the x-axis.
    pub fn x_axis_title(mut self, title: impl Into<String>) -> Self {
        self.x_axis_title = Some(title.into());
        self
    }
    /// Set a title for the left y-axis.
    pub fn y_axis_title(mut self, title: impl Into<String>) -> Self {
        self.y_axis_title = Some(title.into());
        self
    }
    /// Set a title for the right y-axis.
    pub fn right_y_axis_title(mut self, title: impl Into<String>) -> Self {
        self.right_y_title = Some(title.into());
        self
    }
    /// Set the number of y-axis ticks/grid lines (default 5; minimum 2).
    pub fn tick_count(mut self, count: usize) -> Self {
        self.tick_count = count.max(2);
        if let Some(axis) = self.right_y_axis.clone() {
            self.right_y_axis = Some(ValueAxis::from_values(
                &[],
                Some((axis.min, axis.max)),
                self.tick_count,
            ));
        }
        self
    }
    /// Format y-axis values and chart tooltips.
    pub fn value_formatter(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.value_formatter = Some(Rc::new(format));
        self
    }
    /// Format the right-side y-axis when [`Self::right_y_axis`] is enabled.
    pub fn right_value_formatter(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.right_value_formatter = Some(Rc::new(format));
        self
    }
    /// Draw vertical grid lines at category centers.
    pub fn vertical_grid(mut self, on: bool) -> Self {
        self.vertical_grid = on;
        self
    }
    /// Configure category labels for crowded axes.
    pub fn category_label_mode(mut self, mode: CategoryLabelMode) -> Self {
        self.category_label_mode = mode;
        self
    }
    /// Draw value labels on marks.
    pub fn data_labels(mut self, on: bool) -> Self {
        self.data_labels = on;
        self
    }
    /// Add a horizontal reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }
    /// Add a horizontal reference band.
    pub fn reference_band(mut self, band: ReferenceBand) -> Self {
        self.reference_bands.push(band);
        self
    }
    /// Pin a point callout (dot + label) to a data point. See [`annotation`](crate::annotation).
    pub fn annotation(mut self, annotation: Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }
    /// Show a value tooltip when the chart is hovered, tapped, or dragged (default true).
    pub fn tooltip(mut self, on: bool) -> Self {
        self.tooltip = on;
        self
    }
    /// Run `callback(category_index, series_index, value)` when a chart point/bar is tapped.
    pub fn on_point(mut self, callback: impl Fn(usize, usize, f64) + 'static) -> Self {
        self.on_point = Some(Rc::new(callback));
        self
    }
    /// Set line/area interpolation (`Linear`, `Step`, or `Smooth`).
    pub fn curve(mut self, curve: CurveInterpolation) -> Self {
        self.curve = curve;
        self
    }
    /// Keep categories in the half-open range `start..end`.
    pub fn category_window(mut self, start: usize, end: usize) -> Self {
        self.category_window = Some((start.min(end), end.max(start)));
        self
    }
    /// Alias for [`Self::category_window`] when using chart-side data zoom.
    pub fn zoom_categories(self, start: usize, end: usize) -> Self {
        self.category_window(start, end)
    }
    /// Shift the current category window by `offset` slots.
    pub fn pan_category_window(mut self, offset: isize) -> Self {
        let Some((start, end)) = self.category_window else {
            return self;
        };
        let len = end.saturating_sub(start);
        let max_start = self.categories.len().saturating_sub(len);
        let shifted = (start as isize + offset).clamp(0, max_start as isize) as usize;
        self.category_window = Some((shifted, shifted + len));
        self
    }
    /// Sort categories by the sum of finite values across all series.
    pub fn sort_by_total_desc(mut self) -> Self {
        let mut order: Vec<usize> = (0..self.categories.len()).collect();
        order.sort_by(|&a, &b| {
            category_total(&self.series, b)
                .partial_cmp(&category_total(&self.series, a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        reorder_categories(&mut self.categories, &mut self.series, &order);
        self
    }
    /// Sort categories by the sum of finite values across all series.
    pub fn sort_by_total_asc(mut self) -> Self {
        let mut order: Vec<usize> = (0..self.categories.len()).collect();
        order.sort_by(|&a, &b| {
            category_total(&self.series, a)
                .partial_cmp(&category_total(&self.series, b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        reorder_categories(&mut self.categories, &mut self.series, &order);
        self
    }
    /// Keep the top `n` categories by total finite value.
    pub fn top_n(self, n: usize) -> Self {
        self.sort_by_total_desc().category_window(0, n)
    }
    /// Sum consecutive category buckets into larger buckets.
    pub fn aggregate_every(mut self, bucket_size: usize) -> Self {
        let bucket_size = bucket_size.max(1);
        if bucket_size == 1 || self.categories.is_empty() {
            return self;
        }
        self.categories = self
            .categories
            .chunks(bucket_size)
            .map(|chunk| match (chunk.first(), chunk.last()) {
                (Some(first), Some(last)) if first != last => format!("{first}-{last}"),
                (Some(label), _) => label.clone(),
                _ => String::new(),
            })
            .collect();
        for series in &mut self.series {
            series.values = series
                .values
                .chunks(bucket_size)
                .map(|chunk| {
                    let mut any = false;
                    let sum = chunk
                        .iter()
                        .copied()
                        .filter(|v| v.is_finite())
                        .inspect(|_| any = true)
                        .sum();
                    if any { sum } else { f64::NAN }
                })
                .collect();
        }
        self
    }
}

pub(crate) fn category_total(series: &[Series], index: usize) -> f64 {
    series
        .iter()
        .filter_map(|s| s.values.get(index))
        .copied()
        .filter(|v| v.is_finite())
        .sum()
}

pub(crate) fn reorder_categories(
    categories: &mut Vec<String>,
    series: &mut [Series],
    order: &[usize],
) {
    let old_categories = categories.clone();
    *categories = order
        .iter()
        .filter_map(|&i| old_categories.get(i).cloned())
        .collect();
    for s in series {
        let old_values = s.values.clone();
        s.values = order
            .iter()
            .filter_map(|&i| old_values.get(i).copied())
            .collect();
    }
}

pub(crate) fn apply_category_window(
    categories: &mut Vec<String>,
    series: &mut [Series],
    window: Option<(usize, usize)>,
) {
    let Some((start, end)) = window else {
        return;
    };
    let start = start.min(categories.len());
    let end = end.min(categories.len()).max(start);
    *categories = categories[start..end].to_vec();
    for s in series {
        s.values = s.values.get(start..end).unwrap_or(&[]).to_vec();
    }
}

pub(crate) fn cartesian_kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Bar => "Bar",
        Kind::StackedBar => "Stacked bar",
        Kind::PercentStackedBar => "100% stacked bar",
        Kind::HorizontalBar => "Horizontal bar",
        Kind::Line => "Line",
        Kind::SteppedLine => "Stepped line",
        Kind::Area => "Area",
        Kind::StackedArea => "Stacked area",
        Kind::PercentStackedArea => "100% stacked area",
        Kind::Combo => "Combo",
        Kind::Sparkline => "Sparkline",
    }
}

/// Build the accessibility (role/label/value) for a cartesian chart: a spoken summary plus
/// a per-point data read-out, so a screen reader announces the chart AND its data rather
/// than hitting an opaque canvas. This is the chart's data-table fallback.
pub(crate) fn cartesian_a11y(chart: &CartesianChart) -> (String, String) {
    let kind = cartesian_kind_name(chart.kind);
    let ncat = chart.categories.len();
    let nser = chart.series.len();
    let label = chart.a11y_label.clone().unwrap_or_else(|| {
        let mut s = format!("{kind} chart");
        match (nser, ncat) {
            (0, _) | (_, 0) => {}
            (1, _) => s.push_str(&format!(", {ncat} points")),
            _ => s.push_str(&format!(", {nser} series over {ncat} categories")),
        }
        if let Some(t) = &chart.x_axis_title {
            s.push_str(&format!(", x axis {t}"));
        }
        if let Some(t) = &chart.y_axis_title {
            s.push_str(&format!(", y axis {t}"));
        }
        s
    });
    let fmt: Rc<dyn Fn(f64) -> String> = chart
        .value_formatter
        .clone()
        .unwrap_or_else(|| Rc::new(compact_number));
    let mut parts: Vec<String> = Vec::new();
    for s in &chart.series {
        let points: Vec<String> = chart
            .categories
            .iter()
            .zip(&s.values)
            .filter(|(_, v)| v.is_finite())
            .map(|(cat, v)| format!("{cat} {}", fmt(*v)))
            .collect();
        if points.is_empty() {
            continue;
        }
        parts.push(format!("{}: {}", s.label, points.join(", ")));
    }
    let value = if parts.is_empty() {
        "No data".to_string()
    } else {
        parts.join("; ")
    };
    (label, value)
}

impl IntoWidget for CartesianChart {
    fn into_widget(self) -> AnyWidget {
        // Emit an accessibility node (role + summary + data read-out) around the canvas, so
        // the chart isn't a silent blank to a screen reader. See `cartesian_a11y`.
        let (label, value) = cartesian_a11y(&self);
        let inner = if self.fill_width {
            // Responsive: rebuild at the parent's available width on every layout pass
            // (auto-resize). Falls back to the fixed `.width` when the parent is unbounded.
            let chart = self;
            layout_builder(move |size: Size| {
                let mut ch = chart.clone();
                if size.width.is_finite() && size.width > 1.0 {
                    ch.width = size.width;
                }
                component_props(render_cartesian_chart, ch)
            })
            .into_widget()
        } else {
            component_props(render_cartesian_chart, self).into_widget()
        };
        semantics(SemanticsRole::Image, label, inner)
            .value(value)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn value_axis_includes_zero_and_negative_values() {
        let axis = ValueAxis::from_values(&[vec![-12.0, 4.0, 18.0]], None, 5);

        assert!(axis.min <= -12.0);
        assert!(axis.max >= 18.0);
        assert!(axis.ticks.contains(&0.0));
    }

    #[test]
    fn linear_scale_maps_zero_between_negative_and_positive_values() {
        let axis = ValueAxis {
            min: -50.0,
            max: 50.0,
            ticks: vec![-50.0, 0.0, 50.0],
        };
        let scale = axis.scale(10.0, 110.0);

        assert_eq!(scale.map(-50.0), 110.0);
        assert_eq!(scale.map(0.0), 60.0);
        assert_eq!(scale.map(50.0), 10.0);
    }

    #[test]
    fn compact_number_formats_large_values() {
        assert_eq!(compact_number(950.0), "950");
        assert_eq!(compact_number(1_250.0), "1.2k");
        assert_eq!(compact_number(-2_500_000.0), "-2.5M");
    }

    #[test]
    fn explicit_value_axis_domain_is_exact() {
        let axis = ValueAxis::from_values(&[vec![10.0, 20.0, 30.0]], Some((10.0, 20.0)), 3);

        assert_eq!(axis.min, 10.0);
        assert_eq!(axis.max, 20.0);
        assert_eq!(axis.ticks, vec![10.0, 15.0, 20.0]);
    }

    #[test]
    fn optional_series_values_create_finite_segments() {
        let axis = ValueAxis {
            min: 0.0,
            max: 10.0,
            ticks: vec![0.0, 5.0, 10.0],
        };
        let scale = axis.scale(0.0, 100.0);
        let cx = |i| i as f64;
        let values = crate::series_with_gaps(
            "Gap",
            vec![Some(1.0), Some(2.0), None, Some(4.0), Some(5.0)],
        )
        .values;

        let segments = finite_line_segments(&values, &scale, &cx);

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0].iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            segments[1].iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn log_x_axis_maps_values_by_decade() {
        let axis =
            XValueAxis::from_values(&[1.0, 10.0, 100.0], Some((1.0, 100.0)), 3, AxisScale::Log10);

        assert_eq!(axis.map(1.0, 0.0, 200.0), Some(0.0));
        assert_eq!(axis.map(10.0, 0.0, 200.0), Some(100.0));
        assert_eq!(axis.map(100.0, 0.0, 200.0), Some(200.0));
        assert_eq!(axis.map(0.0, 0.0, 200.0), None);
    }

    #[test]
    fn automatic_log_x_axis_stays_positive() {
        let axis = XValueAxis::from_values(&[10.0, 24.0, 65.0], None, 5, AxisScale::Log10);

        assert!(axis.min > 0.0);
        assert!(axis.map(10.0, 0.0, 100.0).is_some());
        assert!(axis.map(65.0, 0.0, 100.0).is_some());
    }

    #[test]
    fn line_and_area_charts_accept_smooth_interpolation() {
        let chart = line_chart(
            vec!["A".into(), "B".into(), "C".into()],
            vec![crate::series("Smooth", vec![1.0, 4.0, 2.0])],
        )
        .curve(CurveInterpolation::Smooth);

        assert_eq!(chart.curve, CurveInterpolation::Smooth);
    }

    #[test]
    fn cartesian_axes_labels_and_reference_options_are_configurable() {
        let chart = bar_chart(
            vec!["A very long label".into(), "B".into()],
            vec![crate::series("Values", vec![8.0, 12.0])],
        )
        .x_axis_title("Month")
        .y_axis_title("Revenue")
        .right_y_axis(0.0, 1.0)
        .right_y_axis_title("Ratio")
        .right_value_formatter(|v| format!("{:.0}%", v * 100.0))
        .vertical_grid(true)
        .category_label_mode(CategoryLabelMode::Truncate(4))
        .reference_line(reference_line(10.0).label("Target"))
        .reference_band(reference_band(4.0, 6.0).label("Range"))
        .data_labels(true);

        assert_eq!(chart.x_axis_title.as_deref(), Some("Month"));
        assert_eq!(chart.y_axis_title.as_deref(), Some("Revenue"));
        assert_eq!(chart.right_y_title.as_deref(), Some("Ratio"));
        assert!(chart.right_y_axis.is_some());
        assert!(chart.right_value_formatter.is_some());
        assert!(chart.vertical_grid);
        assert_eq!(chart.category_label_mode, CategoryLabelMode::Truncate(4));
        assert_eq!(chart.reference_lines.len(), 1);
        assert_eq!(chart.reference_bands.len(), 1);
        assert!(chart.data_labels);
    }

    #[test]
    fn category_label_auto_can_skip_and_truncate() {
        assert_eq!(truncate_label("January", 4), "Ja..");
        let labels = category_label_widgets(
            &["January".into(), "February".into(), "March".into()],
            90.0,
            CategoryLabelMode::Auto,
            Color::BLACK,
            11.0,
            &None,
        );

        assert_eq!(labels.len(), 3);
    }

    #[test]
    fn cartesian_data_helpers_sort_window_and_aggregate() {
        let categories = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let chart = bar_chart(
            categories,
            vec![
                crate::series("One", vec![1.0, 10.0, 3.0, 7.0]),
                crate::series("Two", vec![2.0, 1.0, 4.0, 2.0]),
            ],
        )
        .sort_by_total_desc()
        .category_window(0, 2)
        .pan_category_window(1);

        assert_eq!(chart.categories, vec!["B", "D", "C", "A"]);
        assert_eq!(chart.category_window, Some((1, 3)));

        let aggregated = bar_chart(
            vec!["Jan".into(), "Feb".into(), "Mar".into(), "Apr".into()],
            vec![crate::series("Revenue", vec![1.0, f64::NAN, 3.0, 4.0])],
        )
        .aggregate_every(2);

        assert_eq!(aggregated.categories, vec!["Jan-Feb", "Mar-Apr"]);
        assert_eq!(aggregated.series[0].values, vec![1.0, 7.0]);
    }

    fn default_pad() -> EdgeInsets {
        EdgeInsets {
            left: PLOT_LEFT,
            top: PLOT_TOP,
            right: PLOT_RIGHT,
            bottom: PLOT_BOTTOM,
        }
    }

    #[test]
    fn hit_category_snaps_x_to_category_slot() {
        let pad = default_pad();
        assert_eq!(
            hit_category(Offset::new(12.0, 40.0), 320.0, 180.0, 3, pad),
            Some(0)
        );
        assert_eq!(
            hit_category(Offset::new(160.0, 40.0), 320.0, 180.0, 3, pad),
            Some(1)
        );
        assert_eq!(
            hit_category(Offset::new(400.0, 40.0), 320.0, 180.0, 3, pad),
            None
        );
    }

    #[test]
    fn custom_plot_padding_shifts_hit_test_consistently() {
        // With a big left inset, a point that was category 0 at the default inset must still
        // resolve correctly under the wider padding — proving draw/hit share one value.
        let pad = EdgeInsets {
            left: 60.0,
            top: 12.0,
            right: 10.0,
            bottom: 8.0,
        };
        // x just inside the left inset → first category; x left of it → no hit.
        assert_eq!(
            hit_category(Offset::new(62.0, 40.0), 320.0, 180.0, 3, pad),
            Some(0)
        );
        assert_eq!(
            hit_category(Offset::new(40.0, 40.0), 320.0, 180.0, 3, pad),
            None
        );
    }

    #[test]
    fn line_hit_test_picks_nearest_series_at_category() {
        let vals = vec![vec![10.0, 20.0], vec![30.0, 40.0]];
        let axis = ValueAxis::from_values(&vals, None, 5);
        let scale = axis.scale(PLOT_TOP, 180.0 - PLOT_BOTTOM);
        let hit = hit_cartesian_datum(
            Kind::Line,
            Offset::new(240.0, scale.map(40.0) + 1.0),
            320.0,
            180.0,
            2,
            &vals,
            &axis,
            default_pad(),
        );

        assert_eq!(
            hit,
            Some(ActiveDatum {
                category: 1,
                series: Some(1)
            })
        );
    }

    #[test]
    fn chart_type_constructors_cover_public_families() {
        let categories = vec!["A".into(), "B".into(), "C".into()];
        let series = vec![
            crate::series("Alpha", vec![10.0, 20.0, 30.0]),
            crate::series("Beta", vec![8.0, 16.0, 24.0]),
        ];

        assert_eq!(
            bar_chart(categories.clone(), series.clone()).kind,
            Kind::Bar
        );
        assert_eq!(
            stacked_bar_chart(categories.clone(), series.clone()).kind,
            Kind::StackedBar
        );
        assert_eq!(
            percent_stacked_bar_chart(categories.clone(), series.clone()).kind,
            Kind::PercentStackedBar
        );
        assert_eq!(
            horizontal_bar_chart(categories.clone(), series.clone()).kind,
            Kind::HorizontalBar
        );
        assert_eq!(
            line_chart(categories.clone(), series.clone()).kind,
            Kind::Line
        );
        assert_eq!(
            stepped_line_chart(categories.clone(), series.clone()).kind,
            Kind::SteppedLine
        );
        assert_eq!(
            area_chart(categories.clone(), series.clone()).kind,
            Kind::Area
        );
        assert_eq!(
            stacked_area_chart(categories.clone(), series.clone()).kind,
            Kind::StackedArea
        );
        assert_eq!(
            percent_stacked_area_chart(categories.clone(), series.clone()).kind,
            Kind::PercentStackedArea
        );
        assert_eq!(sparkline(vec![1.0, 2.0, 3.0]).kind, Kind::Sparkline);

        let combo = combo_chart(
            categories.clone(),
            vec![
                combo_series("Bars", vec![1.0, 2.0, 3.0], SeriesKind::Bar),
                combo_series("Line", vec![3.0, 2.0, 1.0], SeriesKind::Line),
            ],
        );
        assert_eq!(combo.kind, Kind::Combo);

        let _scatter = scatter_chart(vec![point_series(
            "Points",
            vec![point(1.0, 2.0), point(2.0, 3.0)],
        )]);
        let _bubble = bubble_chart(vec![point_series(
            "Bubbles",
            vec![bubble_point(1.0, 2.0, 6.0)],
        )]);
        let _radar = radar_chart(categories.clone(), series.clone());
        let _ring = progress_ring("Progress", 72.0, 100.0);
        let _gauge = gauge_chart("Gauge", 42.0, 100.0);
        let _candle = candlestick_chart(categories.clone(), vec![candle(10.0, 14.0, 8.0, 12.0)]);
        let _ohlc = ohlc_chart(categories.clone(), vec![candle(10.0, 14.0, 8.0, 12.0)]);
        let _heatmap = heatmap_chart(categories, vec!["One".into()], vec![heat_cell(0, 0, 4.0)]);
        let _funnel = funnel_chart(vec![crate::slice("Step", 20.0)]);
        let _sankey = sankey_chart(vec![sankey_link("Source", "Target", 12.0)]);
    }
}
