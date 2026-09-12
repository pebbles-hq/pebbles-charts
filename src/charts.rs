//! The chart widgets. Cartesian charts (bar / line / area) share axis, scaling, legend,
//! and category labels — they differ only in what they draw on the canvas. Pie / donut
//! are their own radial widget. Everything is drawn with the framework `Canvas`
//! (`fill_rrect`, `stroke_path`, `fill_path`, `fill_circle`), so it's GPU-rendered.

use std::collections::HashSet;
use std::rc::Rc;

use pebbles::prelude::*;

use crate::{ChartStyle, IntoSeriesValues, Series, Slice, palette_color, with_alpha};

const PLOT_TOP: f64 = 12.0;
const PLOT_RIGHT: f64 = 10.0;
const PLOT_BOTTOM: f64 = 8.0;
const PLOT_LEFT: f64 = 10.0;
const Y_AXIS_WIDTH: f64 = 42.0;
const Y_AXIS_GAP: f64 = 8.0;
const TOOLTIP_OFFSET: f64 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesKind {
    Bar,
    Line,
    Area,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    Linear,
    Log10,
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveInterpolation {
    Linear,
    Step,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryLabelMode {
    Auto,
    All,
    Skip(usize),
    Truncate(usize),
    Hidden,
}

/// Where the legend sits relative to the plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendPosition {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone)]
pub struct ReferenceLine {
    pub value: f64,
    pub label: Option<String>,
    pub color: Option<Color>,
}

pub fn reference_line(value: f64) -> ReferenceLine {
    ReferenceLine {
        value,
        label: None,
        color: None,
    }
}

impl ReferenceLine {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

#[derive(Clone)]
pub struct ReferenceBand {
    pub start: f64,
    pub end: f64,
    pub label: Option<String>,
    pub color: Option<Color>,
}

pub fn reference_band(start: f64, end: f64) -> ReferenceBand {
    ReferenceBand {
        start,
        end,
        label: None,
        color: None,
    }
}

impl ReferenceBand {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A callout pinned to one data point (category index + value): a dot plus a text label,
/// for marking an event or an outlier ("launch", "peak", …). Built with [`annotation`].
#[derive(Clone)]
pub struct Annotation {
    pub category: usize,
    pub value: f64,
    pub label: String,
    pub color: Option<Color>,
}

/// A point callout at category `category`, value `value`, showing `label`.
pub fn annotation(category: usize, value: f64, label: impl Into<String>) -> Annotation {
    Annotation { category, value, label: label.into(), color: None }
}

impl Annotation {
    /// Override the dot + label color (default: the reference color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveDatum {
    category: usize,
    series: Option<usize>,
}

#[derive(Clone, Copy)]
struct LinearScale {
    d0: f64,
    d1: f64,
    p0: f64,
    p1: f64,
}

impl LinearScale {
    fn new(d0: f64, d1: f64, p0: f64, p1: f64) -> Self {
        LinearScale { d0, d1, p0, p1 }
    }

    fn map(&self, value: f64) -> f64 {
        let span = self.d1 - self.d0;
        if span.abs() <= f64::EPSILON {
            return (self.p0 + self.p1) / 2.0;
        }
        let t = (value - self.d0) / span;
        self.p0 + (self.p1 - self.p0) * t
    }
}

#[derive(Clone)]
struct ValueAxis {
    min: f64,
    max: f64,
    ticks: Vec<f64>,
}

impl ValueAxis {
    fn from_values(vals: &[Vec<f64>], explicit: Option<(f64, f64)>, tick_count: usize) -> Self {
        let tick_count = tick_count.max(2);
        let is_explicit = explicit.is_some();
        let (mut lo, mut hi) = explicit.unwrap_or_else(|| data_domain(vals));
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        if !lo.is_finite() || !hi.is_finite() {
            lo = 0.0;
            hi = 1.0;
        }
        if !is_explicit {
            lo = lo.min(0.0);
            hi = hi.max(0.0);
        }
        if (hi - lo).abs() <= f64::EPSILON {
            let pad = if hi.abs() < 1.0 { 1.0 } else { hi.abs() * 0.1 };
            lo -= pad;
            hi += pad;
        }
        if is_explicit {
            return ValueAxis {
                min: lo,
                max: hi,
                ticks: linear_ticks(lo, hi, tick_count),
            };
        }
        let step = nice_step((hi - lo) / (tick_count - 1) as f64);
        let nice_lo = (lo / step).floor() * step;
        let nice_hi = (hi / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut v = nice_lo;
        let guard = tick_count.saturating_mul(4).max(16);
        for _ in 0..guard {
            if v > nice_hi + step * 0.5 {
                break;
            }
            ticks.push(clean_zero(v));
            v += step;
        }
        if ticks.len() < 2 {
            ticks = vec![nice_lo, nice_hi];
        }
        ValueAxis {
            min: nice_lo,
            max: nice_hi,
            ticks,
        }
    }

    fn scale(&self, top: f64, bottom: f64) -> LinearScale {
        LinearScale::new(self.min, self.max, bottom, top)
    }
}

#[derive(Clone)]
struct XValueAxis {
    min: f64,
    max: f64,
    ticks: Vec<f64>,
    scale: AxisScale,
}

impl XValueAxis {
    fn from_values(
        vals: &[f64],
        explicit: Option<(f64, f64)>,
        tick_count: usize,
        scale: AxisScale,
    ) -> Self {
        if scale == AxisScale::Log10 {
            let tick_count = tick_count.max(2);
            let (mut lo, mut hi) = explicit.unwrap_or_else(|| {
                let filtered = vals
                    .iter()
                    .copied()
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .collect::<Vec<_>>();
                data_domain(&[filtered])
            });
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            if !lo.is_finite() || !hi.is_finite() || hi <= 0.0 {
                lo = 1.0;
                hi = 10.0;
            }
            lo = lo.max(f64::MIN_POSITIVE);
            hi = hi.max(lo * 10.0);
            return XValueAxis {
                min: lo,
                max: hi,
                ticks: log_ticks(lo, hi, tick_count),
                scale,
            };
        }
        let filtered: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
        let source = vec![filtered];
        let axis = ValueAxis::from_values(&source, explicit, tick_count);
        XValueAxis {
            min: axis.min,
            max: axis.max,
            ticks: axis.ticks,
            scale,
        }
    }

    fn map(&self, value: f64, left: f64, right: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self.scale {
            AxisScale::Linear | AxisScale::Time => {
                Some(LinearScale::new(self.min, self.max, left, right).map(value))
            }
            AxisScale::Log10 => {
                if value <= 0.0 || self.min <= 0.0 || self.max <= 0.0 {
                    None
                } else {
                    Some(
                        LinearScale::new(self.min.log10(), self.max.log10(), left, right)
                            .map(value.log10()),
                    )
                }
            }
        }
    }
}

fn linear_ticks(lo: f64, hi: f64, tick_count: usize) -> Vec<f64> {
    let tick_count = tick_count.max(2);
    (0..tick_count)
        .map(|i| clean_zero(lo + (hi - lo) * i as f64 / (tick_count - 1) as f64))
        .collect()
}

fn log_ticks(lo: f64, hi: f64, tick_count: usize) -> Vec<f64> {
    let tick_count = tick_count.max(2);
    let start = lo.max(f64::MIN_POSITIVE).log10().floor() as i32;
    let end = hi.max(lo).log10().ceil() as i32;
    let mut ticks: Vec<f64> = (start..=end)
        .map(|exp| 10_f64.powi(exp))
        .filter(|v| *v >= lo && *v <= hi)
        .collect();
    if ticks.len() < 2 {
        ticks = linear_ticks(lo, hi, tick_count);
    }
    ticks
}

fn data_domain(vals: &[Vec<f64>]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in vals.iter().flat_map(|series| series.iter()) {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if lo.is_finite() && hi.is_finite() {
        (lo, hi)
    } else {
        (0.0, 1.0)
    }
}

fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10_f64.powf(exp);
    let frac = raw / base;
    let nice = if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

fn clean_zero(v: f64) -> f64 {
    if v.abs() <= f64::EPSILON { 0.0 } else { v }
}

fn compact_number(value: f64) -> String {
    let abs = value.abs();
    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (value / 1_000.0, "k")
    } else {
        (value, "")
    };
    let decimals = if scaled.abs() >= 100.0 || scaled.fract().abs() <= 0.001 {
        0
    } else {
        1
    };
    format!("{scaled:.decimals$}{suffix}")
}

fn hit_category(pos: Offset, width: f64, height: f64, ncat: usize) -> Option<usize> {
    let ncat = ncat.max(1);
    let pw = (width - PLOT_LEFT - PLOT_RIGHT).max(1.0);
    let y1 = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    if pos.x < PLOT_LEFT || pos.x > PLOT_LEFT + pw || pos.y < PLOT_TOP || pos.y > y1 {
        return None;
    }
    let slot = pw / ncat as f64;
    Some(
        ((pos.x - PLOT_LEFT) / slot)
            .floor()
            .clamp(0.0, (ncat - 1) as f64) as usize,
    )
}

fn hit_cartesian_datum(
    kind: Kind,
    pos: Offset,
    width: f64,
    height: f64,
    ncat: usize,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
) -> Option<ActiveDatum> {
    let category = hit_category(pos, width, height, ncat)?;
    let series = match kind {
        Kind::Bar => hit_bar_series(pos, width, ncat, category, vals),
        Kind::StackedBar | Kind::PercentStackedBar | Kind::HorizontalBar => {
            hit_bar_series(pos, width, ncat, category, vals)
        }
        Kind::Line
        | Kind::SteppedLine
        | Kind::Area
        | Kind::StackedArea
        | Kind::PercentStackedArea
        | Kind::Combo
        | Kind::Sparkline => hit_nearest_line_series(pos, height, category, vals, axis),
    };
    Some(ActiveDatum { category, series })
}

fn hit_bar_series(
    pos: Offset,
    width: f64,
    ncat: usize,
    category: usize,
    vals: &[Vec<f64>],
) -> Option<usize> {
    let nser = vals.len().max(1);
    let pw = (width - PLOT_LEFT - PLOT_RIGHT).max(1.0);
    let slot = pw / ncat.max(1) as f64;
    let group_w = slot * 0.7;
    let bw = group_w / nser as f64;
    let cx = PLOT_LEFT + pw * (category as f64 + 0.5) / ncat.max(1) as f64;
    let group_x = cx - group_w / 2.0;
    let local = ((pos.x - group_x) / bw).floor() as isize;
    let preferred = (local >= 0 && (local as usize) < vals.len()).then_some(local as usize);
    preferred
        .filter(|&si| {
            vals.get(si)
                .and_then(|sv| sv.get(category))
                .is_some_and(|v| v.is_finite())
        })
        .or_else(|| {
            vals.iter()
                .enumerate()
                .filter(|(_, sv)| sv.get(category).is_some_and(|v| v.is_finite()))
                .min_by(|(ai, _), (bi, _)| {
                    let ax = group_x + (*ai as f64 + 0.5) * bw;
                    let bx = group_x + (*bi as f64 + 0.5) * bw;
                    (pos.x - ax)
                        .abs()
                        .partial_cmp(&(pos.x - bx).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(si, _)| si)
        })
}

fn hit_nearest_line_series(
    pos: Offset,
    height: f64,
    category: usize,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
) -> Option<usize> {
    let bottom = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    let scale = axis.scale(PLOT_TOP, bottom);
    vals.iter()
        .enumerate()
        .filter_map(|(si, sv)| {
            let value = *sv.get(category)?;
            value
                .is_finite()
                .then_some((si, (pos.y - scale.map(value)).abs()))
        })
        .min_by(|(_, ad), (_, bd)| ad.partial_cmp(bd).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(si, _)| si)
}

fn plot_local_from_global(global: Offset, chart_bounds: Rect, plot_x: f64) -> Offset {
    Offset::new(
        global.x - chart_bounds.x0 - plot_x,
        global.y - chart_bounds.y0,
    )
}

fn tooltip_rows(
    series: &[Series],
    colors: &[Color],
    vals: &[Vec<f64>],
    active: ActiveDatum,
    format_value: &Rc<dyn Fn(f64) -> String>,
) -> Vec<(String, Color, String, bool)> {
    vals.iter()
        .enumerate()
        .filter_map(|(si, sv)| {
            let value = *sv.get(active.category)?;
            if !value.is_finite() {
                return None;
            }
            let label = series
                .get(si)
                .map(|s| s.label.as_str())
                .filter(|label| !label.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Series {}", si + 1));
            Some((
                label,
                colors.get(si).copied().unwrap_or_else(|| palette_color(si)),
                format_value(value),
                active.series == Some(si),
            ))
        })
        .collect()
}

fn cartesian_tooltip(title: String, rows: Vec<(String, Color, String, bool)>) -> AnyWidget {
    let c = theme().colors;
    let mut children = Vec::new();
    children.push(
        text(title)
            .size(12.0)
            .semibold()
            .color(c.popover_foreground)
            .into_widget(),
    );
    children.push(gap_h(6.0).into_widget());
    for (label, color, value, active) in rows {
        let value_text = if active {
            text(value)
                .size(12.0)
                .semibold()
                .color(c.popover_foreground)
        } else {
            text(value).size(12.0).color(c.popover_foreground)
        };
        children.push(
            row(children![
                container().width(9.0).height(9.0).decoration(
                    BoxDecoration::new()
                        .color(color)
                        .radius(BorderRadius::all(2.5))
                ),
                gap_w(7.0),
                text(label).size(12.0).color(c.muted_foreground),
                gap_w(14.0),
                spacer(),
                value_text,
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .into_widget(),
        );
    }
    container()
        .width(180.0)
        .decoration(
            BoxDecoration::new()
                .color(c.popover)
                .border(Border::new(c.border, 1.0))
                .radius(BorderRadius::all(7.0))
                .shadow(BoxShadow::new(
                    Color::from_rgba8(0, 0, 0, 38),
                    Offset::new(0.0, 8.0),
                    18.0,
                    -5.0,
                )),
        )
        .padding(EdgeInsets::symmetric(10.0, 8.0))
        .child(column(children).main_axis_size(MainAxisSize::Min))
        .into_widget()
}

fn show_cartesian_tooltip(
    global: Offset,
    categories: &[String],
    series: &[Series],
    colors: &[Color],
    vals: &[Vec<f64>],
    active: ActiveDatum,
    format_value: &Rc<dyn Fn(f64) -> String>,
) {
    let title = categories
        .get(active.category)
        .cloned()
        .unwrap_or_else(|| format!("Category {}", active.category + 1));
    let rows = tooltip_rows(series, colors, vals, active, format_value);
    if rows.is_empty() {
        hide_passive();
    } else {
        show_passive(
            cartesian_tooltip(title, rows),
            global.x + TOOLTIP_OFFSET,
            global.y + TOOLTIP_OFFSET,
        );
    }
}

fn stacked_domain_values(vals: &[Vec<f64>], percent: bool) -> Vec<Vec<f64>> {
    let ncat = vals.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = Vec::with_capacity(ncat);
    let mut positives = Vec::with_capacity(ncat);
    let mut negatives = Vec::with_capacity(ncat);
    for i in 0..ncat {
        let mut pos = 0.0;
        let mut neg = 0.0;
        for v in vals
            .iter()
            .filter_map(|series| series.get(i))
            .copied()
            .filter(|v| v.is_finite())
        {
            if v >= 0.0 {
                pos += v;
            } else {
                neg += v;
            }
        }
        if percent {
            positives.push(if pos > 0.0 { 100.0 } else { 0.0 });
            negatives.push(if neg < 0.0 { -100.0 } else { 0.0 });
        } else {
            positives.push(pos);
            negatives.push(neg);
        }
    }
    out.push(positives);
    out.push(negatives);
    out
}

fn axis_values_for_kind(kind: Kind, vals: &[Vec<f64>]) -> Vec<Vec<f64>> {
    match kind {
        Kind::StackedBar | Kind::StackedArea => stacked_domain_values(vals, false),
        Kind::PercentStackedBar | Kind::PercentStackedArea => stacked_domain_values(vals, true),
        _ => vals.to_vec(),
    }
}

fn percent_value(vals: &[Vec<f64>], category: usize, series: usize) -> Option<f64> {
    let value = *vals.get(series)?.get(category)?;
    if !value.is_finite() {
        return None;
    }
    let total: f64 = vals
        .iter()
        .filter_map(|sv| sv.get(category))
        .copied()
        .filter(|v| v.is_finite())
        .map(f64::abs)
        .sum();
    (total > 0.0).then_some(value / total * 100.0)
}

fn stacked_value(kind: Kind, vals: &[Vec<f64>], category: usize, series: usize) -> Option<f64> {
    match kind {
        Kind::PercentStackedBar | Kind::PercentStackedArea => percent_value(vals, category, series),
        _ => vals.get(series).and_then(|sv| sv.get(category)).copied(),
    }
}

fn finite_line_segments(
    values: &[f64],
    y_scale: &LinearScale,
    cx_of: &dyn Fn(usize) -> f64,
) -> Vec<Vec<(usize, f64, f64)>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for (i, &v) in values.iter().enumerate() {
        if v.is_finite() {
            current.push((i, cx_of(i), y_scale.map(v)));
        } else if !current.is_empty() {
            segments.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn append_points_path(path: &mut BezPath, pts: &[(usize, f64, f64)], curve: CurveInterpolation) {
    if pts.is_empty() {
        return;
    }
    path.line_to((pts[0].1, pts[0].2));
    match curve {
        CurveInterpolation::Linear => {
            for &(_, x, y) in &pts[1..] {
                path.line_to((x, y));
            }
        }
        CurveInterpolation::Step => {
            for pair in pts.windows(2) {
                let (_, _x0, y0) = pair[0];
                let (_, x1, y1) = pair[1];
                path.line_to((x1, y0));
                path.line_to((x1, y1));
            }
        }
        CurveInterpolation::Smooth => {
            for i in 0..pts.len().saturating_sub(1) {
                let p0 = if i == 0 { pts[i] } else { pts[i - 1] };
                let p1 = pts[i];
                let p2 = pts[i + 1];
                let p3 = pts.get(i + 2).copied().unwrap_or(p2);
                let c1 = (p1.1 + (p2.1 - p0.1) / 6.0, p1.2 + (p2.2 - p0.2) / 6.0);
                let c2 = (p2.1 - (p3.1 - p1.1) / 6.0, p2.2 - (p3.2 - p1.2) / 6.0);
                path.curve_to(c1, c2, (p2.1, p2.2));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn draw_line_area_series(
    c: &mut Canvas<'_>,
    values: &[f64],
    series_index: usize,
    fill_area: bool,
    area_gradient: bool,
    curve: CurveInterpolation,
    baseline: f64,
    y_scale: &LinearScale,
    cx_of: &dyn Fn(usize) -> f64,
    color: Color,
    active_datum: Option<ActiveDatum>,
    line_w: f64,
    point_r: f64,
    area_a: f32,
) {
    let segments = finite_line_segments(values, y_scale, cx_of);
    if segments.is_empty() {
        return;
    }
    for pts in segments {
        if fill_area {
            let mut fill = BezPath::new();
            fill.move_to((pts[0].1, baseline));
            append_points_path(&mut fill, &pts, curve);
            fill.line_to((pts[pts.len() - 1].1, baseline));
            fill.close_path();
            if area_gradient {
                // Series color at the top of the band fading to transparent at the baseline.
                let grad = Gradient::vertical([with_alpha(color, area_a * 1.9), with_alpha(color, 0.0)]);
                c.fill_path_gradient(&fill, &grad);
            } else {
                c.fill_path(&fill, with_alpha(color, area_a));
            }
        }

        let mut line = BezPath::new();
        line.move_to((pts[0].1, pts[0].2));
        append_points_path(&mut line, &pts[1..], curve);
        c.stroke_path(&line, line_w, color);
        for &(i, x, y) in &pts {
            let active =
                active_datum.is_some_and(|a| a.category == i && a.series == Some(series_index));
            c.fill_circle(Offset::new(x, y), if active { point_r * 1.73 } else { point_r }, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn hit_category_snaps_x_to_category_slot() {
        assert_eq!(
            hit_category(Offset::new(12.0, 40.0), 320.0, 180.0, 3),
            Some(0)
        );
        assert_eq!(
            hit_category(Offset::new(160.0, 40.0), 320.0, 180.0, 3),
            Some(1)
        );
        assert_eq!(
            hit_category(Offset::new(400.0, 40.0), 320.0, 180.0, 3),
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
    on_point: Option<Rc<dyn Fn(usize, usize, f64)>>,
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

fn cartesian(kind: Kind, categories: Vec<String>, series: Vec<Series>) -> CartesianChart {
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
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
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
        self.palette = if colors.is_empty() { None } else { Some(colors) };
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

fn category_total(series: &[Series], index: usize) -> f64 {
    series
        .iter()
        .filter_map(|s| s.values.get(index))
        .copied()
        .filter(|v| v.is_finite())
        .sum()
}

fn reorder_categories(categories: &mut Vec<String>, series: &mut [Series], order: &[usize]) {
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

fn apply_category_window(
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

/// A centered "no data" panel sized to the chart's footprint — shown when a chart is
/// handed no series/slices or only empty/non-finite values, so a live dashboard renders a
/// calm empty state instead of a blank or broken plot.
fn empty_placeholder(width: f64, height: f64, msg: &str) -> AnyWidget {
    let c = theme().colors;
    container()
        .width(width)
        .height(height)
        .child(center(text(msg.to_string()).size(13.0).color(c.muted_foreground)))
        .into_widget()
}

/// A screen-reader name for a cartesian chart kind.
fn cartesian_kind_name(kind: Kind) -> &'static str {
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
fn cartesian_a11y(chart: &CartesianChart) -> (String, String) {
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
    let fmt: Rc<dyn Fn(f64) -> String> =
        chart.value_formatter.clone().unwrap_or_else(|| Rc::new(compact_number));
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
    let value = if parts.is_empty() { "No data".to_string() } else { parts.join("; ") };
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
        semantics(SemanticsRole::Image, label, inner).value(value).into_widget()
    }
}

fn render_cartesian_chart(chart: &CartesianChart) -> AnyWidget {
    let kind = chart.kind;
    let mut categories = chart.categories.clone();
    let mut series = chart.series.clone();
    let width = chart.width;
    // Aspect-ratio lock: derive the height from the width so the chart keeps its shape.
    let height = chart.aspect_ratio.map(|r| width / r).unwrap_or(chart.height);
    // Loading / error take priority over the plot and over the empty state.
    if let Some(msg) = &chart.error {
        return empty_placeholder(width, height, msg);
    }
    if chart.loading {
        return empty_placeholder(width, height, "Loading…");
    }
    let legend = chart.legend;
    let grid = chart.grid;
    let y_axis = chart.y_axis;
    let right_y_axis = chart.right_y_axis.clone();
    let right_y_title = chart.right_y_title.clone();
    let y_range = chart.y_range;
    let tick_count = chart.tick_count;
    let value_formatter = chart.value_formatter.clone();
    let right_value_formatter = chart.right_value_formatter.clone();
    let tooltip = chart.tooltip;
    let on_point = chart.on_point.clone();
    let all_combo_kinds = chart.combo_kinds.clone();
    let legend_position = chart.legend_position;
    let legend_values = chart.legend_values;
    let animate = chart.animate.unwrap_or(!prefers_reduced_motion());
    let animation_ms = chart.animation_ms;
    let area_gradient = chart.area_gradient;
    let curve = chart.curve;
    let vertical_grid = chart.vertical_grid;
    let x_axis_title = chart.x_axis_title.clone();
    let y_axis_title = chart.y_axis_title.clone();
    let category_label_mode = chart.category_label_mode;
    let reference_lines = chart.reference_lines.clone();
    let reference_bands = chart.reference_bands.clone();
    let annotations = chart.annotations.clone();
    let data_labels = chart.data_labels;
    let palette_override = chart.palette.clone();
    let pal_color = move |i: usize| -> Color {
        match &palette_override {
            Some(p) if !p.is_empty() => p[i % p.len()],
            _ => palette_color(i),
        }
    };
    apply_category_window(&mut categories, &mut series, chart.category_window);

    // Empty state: no categories/series, or every value is missing/non-finite. Render a
    // calm "no data" panel at the chart's footprint rather than a blank or broken plot.
    let has_data = !categories.is_empty()
        && !series.is_empty()
        && series.iter().any(|s| s.values.iter().any(|v| v.is_finite()));
    if !has_data {
        return empty_placeholder(width, height, "No data");
    }

    let active = create_signal(None::<ActiveDatum>);
    let active_datum = active.get();
    // Entry animation: a 0→1 progress kicked once on mount (the same one-shot idiom the
    // framework's `animated` hook uses). The draw wipes the marks in left-to-right by `t`.
    let anim = create_signal(0.0_f64);
    let anim_kicked = create_signal(false);
    if animate && !anim_kicked.peek() {
        anim_kicked.set(true);
        pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
    }
    let anim_t = if animate { anim.get() } else { 1.0 };
    // Data-change tween: when the values change on a later render (same shape), morph the
    // marks from the previous values to the new ones instead of snapping. Snapshots are
    // kept by ORIGINAL series index (the full set) so a legend toggle — which only changes
    // the *visible* subset — never reads as a data change. Same one-shot idiom as the entry
    // kick: signal writes during render, guarded by `peek()`.
    let full_target: Vec<Vec<f64>> = series.iter().map(|s| s.values.clone()).collect();
    let data_t = create_signal(1.0_f64);
    let last_full = create_signal(None::<Vec<Vec<f64>>>);
    let from_full = create_signal(Vec::<Vec<f64>>::new());
    // Track series labels across renders to tell an ADD from a REMOVE on a shape change:
    // an add re-wipes (enter), a pure removal is handled by the exit-fade block below.
    let last_labels = create_signal(Vec::<String>::new());
    let cur_labels: Vec<String> = series.iter().map(|s| s.label.clone()).collect();
    let series_added = {
        let prev = last_labels.peek();
        !prev.is_empty() && cur_labels.iter().any(|l| !prev.contains(l))
    };
    last_labels.set(cur_labels);
    {
        let prev = last_full.peek();
        let same_shape = prev.as_ref().is_some_and(|p| {
            p.len() == full_target.len()
                && p.iter().zip(&full_target).all(|(a, b)| a.len() == b.len())
        });
        let changed = prev.as_ref().is_some_and(|p| p != &full_target);
        if changed && same_shape && animate {
            // Same shape, new values → morph each datum from its old value to the new one.
            from_full.set(prev.clone().unwrap());
            data_t.set(0.0);
            pebbles::core::animation::animate_to(data_t, 1.0, animation_ms as f64 / 1000.0);
        } else if prev.is_none() {
            // First mount → no morph (the entry wipe handles the reveal).
            from_full.set(full_target.clone());
        } else if changed {
            // Shape changed → snap the values. A series ADD re-runs the entry wipe (enter
            // animation); a pure REMOVE skips the wipe and fades the gone series out below.
            from_full.set(full_target.clone());
            data_t.set(1.0);
            if animate && series_added {
                anim.set(0.0);
                pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
            }
        }
        last_full.set(Some(full_target.clone()));
    }
    let data_t_val = if animate { data_t.get() } else { 1.0 };
    let morph_from = from_full.peek();
    // Interpolate a target datum from its held "from" snapshot by the tween progress.
    // Gaps (NaN) and a settled tween snap straight to the target.
    let morph = move |orig_series: usize, cat: usize, target: f64| -> f64 {
        let fv = morph_from.get(orig_series).and_then(|r| r.get(cat)).copied().unwrap_or(target);
        if data_t_val >= 1.0 || fv.is_nan() || target.is_nan() {
            target
        } else {
            fv + (target - fv) * data_t_val
        }
    };
    // Keyboard traversal: the chart is focusable (Tab / click), and Left/Right arrows
    // move the active category, highlighting it + drawing the crosshair. Uses the
    // framework's register_keys (non-editor key routing).
    let focus = create_focus();
    let ncat_for_keys = categories.len().max(1);
    focus.register(std::rc::Rc::new(|| {}), None, false);
    focus.register_keys(std::rc::Rc::new(move |key: KeyInput| {
        let step = match key {
            KeyInput::Move { motion: Motion::Left, .. } => -1i64,
            KeyInput::Move { motion: Motion::Right, .. } => 1,
            _ => return false,
        };
        let next = match active.peek() {
            None => 0,
            Some(a) => (a.category as i64 + step).rem_euclid(ncat_for_keys as i64) as usize,
        };
        active.set(Some(ActiveDatum { category: next, series: None }));
        true
    }));
    // Series toggled off from the legend. Kept by ORIGINAL series index so the legend
    // (which shows every series) and the palette color stay stable while the plot,
    // domain, tooltip, and hit-testing operate on only the visible subset.
    let hidden = create_signal(HashSet::<usize>::new());
    let hidden_set = hidden.get();
    let chart_bounds = use_bounds();

    // Colors by original index — the legend paints every series in its own color.
    let all_colors: Vec<Color> = series
        .iter()
        .enumerate()
        .map(|(i, s)| s.color.unwrap_or_else(|| pal_color(i)))
        .collect();

    // Per-series exit fade: when a series is removed (matched by label, same category
    // count), keep drawing its REAL last values as a fading ghost line for one animation
    // cycle, so a removed series recedes instead of vanishing. Enter is the shape-change
    // re-wipe above. Snapshots the last-rendered (label, values, color) to recover a gone
    // series' data. Values/hit-testing/scale are unaffected — the ghost is draw-only.
    let ncat_now = categories.len();
    let exiting = create_signal(Vec::<(Vec<f64>, Color)>::new());
    let exit_t = create_signal(1.0_f64);
    let prev_rendered = create_signal(Vec::<(String, Vec<f64>, Color)>::new());
    {
        let cur: Vec<(String, Vec<f64>, Color)> = series
            .iter()
            .enumerate()
            .map(|(i, s)| (s.label.clone(), s.values.clone(), all_colors[i]))
            .collect();
        let prev = prev_rendered.peek();
        let removed: Vec<(Vec<f64>, Color)> = prev
            .iter()
            .filter(|(l, v, _)| v.len() == ncat_now && !cur.iter().any(|(cl, _, _)| cl == l))
            .map(|(_, v, c)| (v.clone(), *c))
            .collect();
        if !removed.is_empty() && animate {
            exiting.set(removed);
            exit_t.set(0.0);
            pebbles::core::animation::animate_to(exit_t, 1.0, animation_ms as f64 / 1000.0);
        }
        prev_rendered.set(cur);
    }
    let exit_t_val = if animate { exit_t.get() } else { 1.0 };
    let exiting_series = if exit_t_val < 1.0 { exiting.peek() } else { Vec::new() };

    let visible: Vec<usize> = (0..series.len()).filter(|i| !hidden_set.contains(i)).collect();
    // Visible-only views: everything downstream (scale, draw, hit-test, tooltip) reads
    // these, so hiding a series rescales the axis and reflows grouped bars automatically.
    let colors: Vec<Color> = visible.iter().map(|&i| all_colors[i]).collect();
    let vals: Vec<Vec<f64>> = visible.iter().map(|&i| series[i].values.clone()).collect();
    let vis_series: Vec<Series> = visible.iter().map(|&i| series[i].clone()).collect();
    let combo_kinds: Vec<SeriesKind> = if all_combo_kinds.is_empty() {
        Vec::new()
    } else {
        visible
            .iter()
            .map(|&i| all_combo_kinds.get(i).copied().unwrap_or(SeriesKind::Line))
            .collect()
    };
    // Morphed views for the axis + marks: each visible datum interpolated from its held
    // snapshot (by original series index) toward the target. `vals` stays the target set,
    // so hit-testing, tooltips, and data labels always report the real (final) numbers.
    let anim_vals: Vec<Vec<f64>> = visible
        .iter()
        .map(|&oi| {
            series[oi].values.iter().enumerate().map(|(ci, &tv)| morph(oi, ci, tv)).collect()
        })
        .collect();
    let draw_vals = anim_vals.clone();
    let draw_colors = colors.clone();
    // Per-series ± error whiskers (visible order), drawn on each mark. None = no whiskers.
    let draw_errors: Vec<Option<Vec<f64>>> = visible.iter().map(|&i| series[i].errors.clone()).collect();
    let ncat = categories.len().max(1);
    let mut axis_values = axis_values_for_kind(kind, &anim_vals);
    // Error whiskers reach v ± e, so the domain must include those bounds (non-stacked
    // kinds; stacked/percent charts don't draw error bars). Add v+e and v-e as extra rows.
    let stacked_kind = matches!(
        kind,
        Kind::StackedBar | Kind::PercentStackedBar | Kind::StackedArea | Kind::PercentStackedArea
    );
    if !stacked_kind {
        for (si, errs) in draw_errors.iter().enumerate() {
            let Some(errs) = errs else { continue };
            let vals_si = draw_vals.get(si).map(Vec::as_slice).unwrap_or(&[]);
            let hi: Vec<f64> = vals_si
                .iter()
                .enumerate()
                .map(|(ci, &v)| v + errs.get(ci).copied().filter(|e| e.is_finite()).unwrap_or(0.0))
                .collect();
            let lo: Vec<f64> = vals_si
                .iter()
                .enumerate()
                .map(|(ci, &v)| v - errs.get(ci).copied().filter(|e| e.is_finite()).unwrap_or(0.0))
                .collect();
            axis_values.push(hi);
            axis_values.push(lo);
        }
    }
    let axis = ValueAxis::from_values(&axis_values, y_range, tick_count);
    let ticks = axis.ticks.clone();
    let format_value: Rc<dyn Fn(f64) -> String> =
        value_formatter.unwrap_or_else(|| Rc::new(compact_number));
    let tick_labels: Vec<String> = ticks.iter().map(|&tick| format_value(tick)).collect();
    let right_format_value: Rc<dyn Fn(f64) -> String> =
        right_value_formatter.unwrap_or_else(|| format_value.clone());
    let right_tick_labels: Vec<String> = right_y_axis
        .as_ref()
        .map(|axis| {
            axis.ticks
                .iter()
                .map(|&tick| right_format_value(tick))
                .collect()
        })
        .unwrap_or_default();
    // Per-slot style (grid/zero/reference/label colors, stroke/point/area/bar geometry,
    // label typography) — theme defaults unless overridden via `.style(ChartStyle)`.
    let style = chart.style.clone();
    let grid_c = style.grid();
    let zero_c = style.zero_line();
    let reference_c = style.reference();
    let active_band_c = with_alpha(theme().colors.muted_foreground, 0.07);
    let active_line_c = with_alpha(theme().colors.muted_foreground, 0.38);
    let label_c = style.label();
    let line_w = style.line_w();
    let point_r = style.point_r();
    let area_a = style.area_a();
    // Stacked areas overlap, so they default denser (0.32) than a lone area (0.18); an
    // explicit `.area_alpha` in the style overrides both.
    let stacked_area_a = style.area_alpha.unwrap_or(0.32);
    let bar_r_max = style.bar_r();
    let label_px = style.label_px();
    let font = style.font_family.clone();
    let left_axis_width = if y_axis {
        Y_AXIS_WIDTH + Y_AXIS_GAP
    } else {
        0.0
    };
    let right_axis_width = if right_y_axis.is_some() {
        Y_AXIS_GAP + Y_AXIS_WIDTH
    } else {
        0.0
    };
    let plot_width = (width - left_axis_width - right_axis_width).max(80.0);
    let plot_x = if y_axis {
        Y_AXIS_WIDTH + Y_AXIS_GAP
    } else {
        0.0
    };
    let axis_for_draw = axis.clone();
    let combo_kinds_for_draw = combo_kinds.clone();
    let reference_lines_for_draw = reference_lines.clone();
    let reference_bands_for_draw = reference_bands.clone();

    let plot = canvas(move |c: &mut Canvas<'_>| {
        let s = c.size();
        let (pl, pt, pr, pb) = (PLOT_LEFT, PLOT_TOP, PLOT_RIGHT, PLOT_BOTTOM);
        let pw = (s.width - pl - pr).max(1.0);
        let ph = (s.height - pt - pb).max(1.0);
        let x0 = pl;
        let y1 = pt + ph;
        let y_scale = axis_for_draw.scale(pt, y1);
        let baseline = y_scale.map(0.0).clamp(pt, y1);
        let cx_of = |i: usize| x0 + pw * (i as f64 + 0.5) / ncat as f64;

        for band in &reference_bands_for_draw {
            if !band.start.is_finite() || !band.end.is_finite() {
                continue;
            }
            let y0 = y_scale.map(band.start).clamp(pt, y1);
            let yb = y_scale.map(band.end).clamp(pt, y1);
            let color = band.color.unwrap_or(reference_c);
            c.fill_rect(
                Rect::new(x0, y0.min(yb), x0 + pw, y0.max(yb)),
                with_alpha(color, 0.14),
            );
        }

        if let Some(active) = active_datum {
            let slot = pw / ncat as f64;
            let band_x0 = (cx_of(active.category) - slot / 2.0).max(x0);
            let band_x1 = (band_x0 + slot).min(x0 + pw);
            c.fill_rect(Rect::new(band_x0, pt, band_x1, y1), active_band_c);
        }

        if grid {
            for &tick in &axis_for_draw.ticks {
                let y = y_scale.map(tick);
                c.stroke_line(Offset::new(x0, y), Offset::new(x0 + pw, y), 1.0, grid_c);
            }
        }
        if vertical_grid {
            for i in 0..ncat {
                let x = cx_of(i);
                c.stroke_line(Offset::new(x, pt), Offset::new(x, y1), 1.0, grid_c);
            }
        }
        c.stroke_line(
            Offset::new(x0, baseline),
            Offset::new(x0 + pw, baseline),
            1.25,
            zero_c,
        );
        for line in &reference_lines_for_draw {
            if !line.value.is_finite() {
                continue;
            }
            let y = y_scale.map(line.value).clamp(pt, y1);
            // Reference/target lines are drawn dashed — the conventional way to distinguish
            // an annotation from a data mark (matches Chart.js / D3 target lines).
            c.stroke_line_dashed(
                Offset::new(x0, y),
                Offset::new(x0 + pw, y),
                1.4,
                line.color.unwrap_or(reference_c),
                &[6.0, 4.0],
                0.0,
            );
        }

        if let Some(active) = active_datum {
            let x = cx_of(active.category);
            c.stroke_line(Offset::new(x, pt), Offset::new(x, y1), 1.0, active_line_c);
        }

        // Entry animation: reveal the marks left-to-right as `anim_t` goes 0→1. Only the
        // marks are clipped — grid, axes, baseline and reference lines stay static.
        let wiping = anim_t < 1.0;
        if wiping {
            c.push_clip(Rect::new(x0, pt, x0 + pw * anim_t, y1));
        }
        match kind {
            Kind::Bar => {
                let nser = draw_vals.len().max(1);
                let slot = pw / ncat as f64;
                let group_w = slot * 0.7;
                let bw = group_w / nser as f64;
                for (si, sv) in draw_vals.iter().enumerate() {
                    for (i, &v) in sv.iter().enumerate() {
                        if !v.is_finite() {
                            continue;
                        }
                        let gx = cx_of(i) - group_w / 2.0 + si as f64 * bw;
                        let y = y_scale.map(v);
                        let r =
                            Rect::new(gx + 1.0, y.min(baseline), gx + bw - 1.0, y.max(baseline));
                        c.fill_rrect(r, (bw / 3.0).min(bar_r_max), draw_colors[si]);
                        if active_datum.is_some_and(|a| a.category == i && a.series == Some(si)) {
                            c.fill_rrect(r, (bw / 3.0).min(bar_r_max), with_alpha(draw_colors[si], 0.32));
                        }
                    }
                }
            }
            Kind::StackedBar | Kind::PercentStackedBar => {
                let slot = pw / ncat as f64;
                let bw = slot * 0.64;
                for i in 0..ncat {
                    let mut positive = 0.0;
                    let mut negative = 0.0;
                    for (si, _) in draw_vals.iter().enumerate() {
                        let Some(v) = stacked_value(kind, &draw_vals, i, si) else {
                            continue;
                        };
                        if !v.is_finite() || v == 0.0 {
                            continue;
                        }
                        let base = if v >= 0.0 { positive } else { negative };
                        let next = base + v;
                        if v >= 0.0 {
                            positive = next;
                        } else {
                            negative = next;
                        }
                        let x = cx_of(i) - bw / 2.0;
                        let y0 = y_scale.map(base);
                        let y1s = y_scale.map(next);
                        let r = Rect::new(x, y0.min(y1s), x + bw, y0.max(y1s));
                        c.fill_rrect(r, (bw / 3.0).min(bar_r_max), draw_colors[si]);
                        if active_datum.is_some_and(|a| a.category == i && a.series == Some(si)) {
                            c.fill_rrect(r, (bw / 3.0).min(bar_r_max), with_alpha(draw_colors[si], 0.32));
                        }
                    }
                }
            }
            Kind::HorizontalBar => {
                let x_scale = LinearScale::new(axis_for_draw.min, axis_for_draw.max, x0, x0 + pw);
                let h_baseline = x_scale.map(0.0).clamp(x0, x0 + pw);
                let nser = draw_vals.len().max(1);
                let slot = ph / ncat as f64;
                let group_h = slot * 0.7;
                let bh = group_h / nser as f64;
                for (si, sv) in draw_vals.iter().enumerate() {
                    for (i, &v) in sv.iter().enumerate() {
                        if !v.is_finite() {
                            continue;
                        }
                        let cy = pt + slot * (i as f64 + 0.5);
                        let gy = cy - group_h / 2.0 + si as f64 * bh;
                        let x = x_scale.map(v);
                        let r = Rect::new(
                            x.min(h_baseline),
                            gy + 1.0,
                            x.max(h_baseline),
                            gy + bh - 1.0,
                        );
                        c.fill_rrect(r, (bh / 3.0).min(bar_r_max), draw_colors[si]);
                    }
                }
            }
            Kind::Combo => {
                let bar_count = combo_kinds_for_draw
                    .iter()
                    .filter(|&&k| k == SeriesKind::Bar)
                    .count()
                    .max(1);
                let slot = pw / ncat as f64;
                let group_w = slot * 0.48;
                let bw = group_w / bar_count as f64;
                let mut bar_seen = 0;
                for (si, sv) in draw_vals.iter().enumerate() {
                    match combo_kinds_for_draw
                        .get(si)
                        .copied()
                        .unwrap_or(SeriesKind::Line)
                    {
                        SeriesKind::Bar => {
                            let offset = bar_seen;
                            bar_seen += 1;
                            for (i, &v) in sv.iter().enumerate() {
                                if !v.is_finite() {
                                    continue;
                                }
                                let gx = cx_of(i) - group_w / 2.0 + offset as f64 * bw;
                                let y = y_scale.map(v);
                                let r = Rect::new(
                                    gx + 1.0,
                                    y.min(baseline),
                                    gx + bw - 1.0,
                                    y.max(baseline),
                                );
                                c.fill_rrect(
                                    r,
                                    (bw / 3.0).min(bar_r_max),
                                    with_alpha(draw_colors[si], 0.76),
                                );
                            }
                        }
                        SeriesKind::Area | SeriesKind::Line => {
                            draw_line_area_series(
                                c,
                                sv,
                                si,
                                SeriesKind::Area
                                    == combo_kinds_for_draw
                                        .get(si)
                                        .copied()
                                        .unwrap_or(SeriesKind::Line),
                                area_gradient,
                                curve,
                                baseline,
                                &y_scale,
                                &cx_of,
                                draw_colors[si],
                                active_datum,
                                line_w,
                                point_r,
                                area_a,
                            );
                        }
                    }
                }
            }
            Kind::StackedArea | Kind::PercentStackedArea => {
                let mut positive = vec![0.0; ncat];
                let mut negative = vec![0.0; ncat];
                for (si, _) in draw_vals.iter().enumerate() {
                    let mut top = Vec::new();
                    let mut bottom = Vec::new();
                    for i in 0..ncat {
                        let Some(v) = stacked_value(kind, &draw_vals, i, si) else {
                            continue;
                        };
                        if !v.is_finite() {
                            continue;
                        }
                        let base = if v >= 0.0 { positive[i] } else { negative[i] };
                        let next = base + v;
                        if v >= 0.0 {
                            positive[i] = next;
                        } else {
                            negative[i] = next;
                        }
                        bottom.push((cx_of(i), y_scale.map(base)));
                        top.push((cx_of(i), y_scale.map(next)));
                    }
                    if top.is_empty() {
                        continue;
                    }
                    let mut fill = BezPath::new();
                    fill.move_to(top[0]);
                    for &p in &top[1..] {
                        fill.line_to(p);
                    }
                    for &p in bottom.iter().rev() {
                        fill.line_to(p);
                    }
                    fill.close_path();
                    if area_gradient {
                        let grad = Gradient::vertical([
                            with_alpha(draw_colors[si], 0.5),
                            with_alpha(draw_colors[si], 0.05),
                        ]);
                        c.fill_path_gradient(&fill, &grad);
                    } else {
                        c.fill_path(&fill, with_alpha(draw_colors[si], stacked_area_a));
                    }
                    let mut line = BezPath::new();
                    line.move_to(top[0]);
                    for &p in &top[1..] {
                        line.line_to(p);
                    }
                    c.stroke_path(&line, line_w, draw_colors[si]);
                }
            }
            Kind::Area | Kind::Line | Kind::SteppedLine | Kind::Sparkline => {
                for (si, sv) in draw_vals.iter().enumerate() {
                    draw_line_area_series(
                        c,
                        sv,
                        si,
                        kind == Kind::Area,
                        area_gradient,
                        if kind == Kind::SteppedLine {
                            CurveInterpolation::Step
                        } else {
                            curve
                        },
                        baseline,
                        &y_scale,
                        &cx_of,
                        draw_colors[si],
                        active_datum,
                        line_w,
                        point_r,
                        area_a,
                    );
                }
            }
        }
        if wiping {
            c.pop_clip();
        }
        // Error bars: a ± whisker (with caps) on each mark for series that carry `errors`.
        // Positioned on the real mark — the grouped-bar center for bars, the point x
        // otherwise — using the current scale so it tracks the value tween.
        if draw_errors.iter().any(Option::is_some) {
            let slot = pw / ncat as f64;
            let group_w = slot * 0.7;
            let bw = group_w / (draw_vals.len().max(1)) as f64;
            for (si, errs) in draw_errors.iter().enumerate() {
                let Some(errs) = errs else { continue };
                let col = with_alpha(draw_colors.get(si).copied().unwrap_or(zero_c), 0.85);
                for (ci, &v) in draw_vals.get(si).map(Vec::as_slice).unwrap_or(&[]).iter().enumerate() {
                    let e = errs.get(ci).copied().unwrap_or(f64::NAN);
                    if !v.is_finite() || !e.is_finite() || e <= 0.0 {
                        continue;
                    }
                    let x = if kind == Kind::Bar {
                        cx_of(ci) - group_w / 2.0 + si as f64 * bw + bw / 2.0
                    } else {
                        cx_of(ci)
                    };
                    let y_hi = y_scale.map(v + e);
                    let y_lo = y_scale.map(v - e);
                    let cap = 4.0;
                    c.stroke_line(Offset::new(x, y_hi), Offset::new(x, y_lo), 1.4, col);
                    c.stroke_line(Offset::new(x - cap, y_hi), Offset::new(x + cap, y_hi), 1.4, col);
                    c.stroke_line(Offset::new(x - cap, y_lo), Offset::new(x + cap, y_lo), 1.4, col);
                }
            }
        }
        // Per-series exit fade: draw each removed series' real last values as a fading ghost
        // line (outside the entry-wipe clip), receding over the current scale.
        if !exiting_series.is_empty() {
            let a = ((1.0 - exit_t_val) as f32).clamp(0.0, 1.0);
            for (evals, ecolor) in &exiting_series {
                draw_line_area_series(
                    c,
                    evals,
                    usize::MAX,
                    false,
                    false,
                    curve,
                    baseline,
                    &y_scale,
                    &cx_of,
                    with_alpha(*ecolor, a * 0.9),
                    None,
                    line_w,
                    point_r * a as f64,
                    area_a,
                );
            }
        }
    })
    .width(plot_width)
    .height(height);

    let mut plot_layers = vec![plot.into_widget()];
    // Hold data labels until the entry wipe finishes, so they don't float over
    // not-yet-revealed marks.
    if data_labels && anim_t >= 0.999 && data_t_val >= 0.999 {
        plot_layers.push(cartesian_data_labels(
            kind,
            &vals,
            &axis,
            plot_width,
            height,
            &format_value,
            label_c,
            label_px,
            &font,
        ));
    }
    plot_layers.push(reference_labels(
        &reference_lines,
        &reference_bands,
        &axis,
        plot_width,
        height,
        label_c,
    ));
    if !annotations.is_empty() && anim_t >= 0.999 && data_t_val >= 0.999 {
        plot_layers.push(annotation_layer(
            &annotations,
            &axis,
            plot_width,
            height,
            ncat,
            reference_c,
            label_c,
            label_px,
            &font,
        ));
    }
    let plot_surface = sized_box(stack(plot_layers))
        .width(plot_width)
        .height(height)
        .into_widget();

    let activate = {
        let axis = axis.clone();
        let vals = vals.clone();
        let categories = categories.clone();
        // Visible series only — `hit`/`colors`/`vals` all index the visible subset.
        let series = vis_series.clone();
        let colors = colors.clone();
        let format_value = format_value.clone();
        Rc::new(move |local: Offset, global: Offset| {
            if let Some(hit) =
                hit_cartesian_datum(kind, local, plot_width, height, ncat, &vals, &axis)
            {
                active.set(Some(hit));
                if tooltip {
                    show_cartesian_tooltip(
                        global,
                        &categories,
                        &series,
                        &colors,
                        &vals,
                        hit,
                        &format_value,
                    );
                }
            } else {
                active.set(None);
                hide_passive();
            }
        })
    };

    let tap_point = {
        let vals = vals.clone();
        let on_point = on_point.clone();
        Rc::new(move |hit: ActiveDatum| {
            let Some(callback) = &on_point else {
                return;
            };
            let Some(series_idx) = hit.series else {
                return;
            };
            let Some(value) = vals
                .get(series_idx)
                .and_then(|sv| sv.get(hit.category))
                .copied()
            else {
                return;
            };
            if value.is_finite() {
                callback(hit.category, series_idx, value);
            }
        })
    };

    let plot = GestureDetector::new(plot_surface)
        .cursor(Cursor::Pointer)
        .on_hover_enter(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                let local = plot_local_from_global(e.global, chart_bounds, plot_x);
                activate(local, e.global);
            }
        }))
        // Continuous hover: the tooltip + crosshair FOLLOW the cursor across the plot
        // (not just on entry), via the framework's new on_hover_move callback.
        .on_hover_move(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                let local = plot_local_from_global(e.global, chart_bounds, plot_x);
                activate(local, e.global);
            }
        }))
        .on_hover_exit(move || {
            active.set(None);
            hide_passive();
        })
        .on_pointer_down(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                focus.request_focus(); // clicking focuses the chart so arrows traverse it
                activate(e.position, e.global);
            }
        }))
        .on_pan_start(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_update(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_end(move || hide_passive())
        .on_tap(action_event({
            let axis = axis.clone();
            let vals = vals.clone();
            let tap_point = tap_point.clone();
            move |e: PointerEvent| {
                if let Some(hit) =
                    hit_cartesian_datum(kind, e.position, plot_width, height, ncat, &vals, &axis)
                {
                    tap_point(hit);
                }
            }
        }));

    let chart_body = if y_axis || right_y_axis.is_some() {
        let mut axis_row = Vec::new();
        if y_axis {
            axis_row.push(y_axis_labels(
                &axis,
                tick_labels,
                height,
                label_c,
                TextAlign::Right,
                label_px,
                &font,
            ));
            axis_row.push(gap_w(Y_AXIS_GAP).into_widget());
        }
        axis_row.push(plot.into_widget());
        if let Some(axis) = &right_y_axis {
            axis_row.push(gap_w(Y_AXIS_GAP).into_widget());
            axis_row.push(y_axis_labels(
                axis,
                right_tick_labels,
                height,
                label_c,
                TextAlign::Left,
                label_px,
                &font,
            ));
        }
        row(axis_row).into_widget()
    } else {
        plot.into_widget()
    };

    let mut col: Vec<AnyWidget> = Vec::new();
    if y_axis_title.is_some() || right_y_title.is_some() {
        col.push(axis_title_row(
            y_axis_title.clone(),
            right_y_title.clone(),
            left_axis_width,
            plot_width,
            right_axis_width,
            label_c,
            label_px,
            &font,
        ));
        col.push(gap_h(4.0).into_widget());
    }
    col.push(chart_body);
    if kind != Kind::Sparkline {
        col.push(gap_h(6.0).into_widget());
        // The canvas insets its plot by PLOT_LEFT/PLOT_RIGHT, so category `i` is centered
        // at `PLOT_LEFT + (plot_width - PLOT_LEFT - PLOT_RIGHT)*(i+0.5)/ncat`. The label
        // cells must divide that SAME inset span — otherwise they fan out from the bars
        // (first label left of its bar, last label right of its bar). Add the plot padding
        // to the axis gutters and give the cells the inset width.
        let label_span = (plot_width - PLOT_LEFT - PLOT_RIGHT).max(1.0);
        col.push(
            row(children![
                gap_w(left_axis_width + PLOT_LEFT).into_widget(),
                expanded(row(category_label_widgets(
                    &categories,
                    label_span,
                    category_label_mode,
                    label_c,
                    label_px,
                    &font,
                ))),
                gap_w(right_axis_width + PLOT_RIGHT).into_widget(),
            ])
            .into_widget(),
        );
        if let Some(title) = x_axis_title {
            col.push(gap_h(4.0).into_widget());
            col.push(
                row(children![
                    gap_w(left_axis_width).into_widget(),
                    expanded(
                        apply_font(text(title).size(label_px + 0.5).color(label_c), &font)
                            .align(TextAlign::Center)
                    ),
                    gap_w(right_axis_width).into_widget(),
                ])
                .into_widget(),
            );
        }
    }
    // Build the legend once. It lists EVERY series (in its own color, dimmed if toggled
    // off); tapping a chip flips that series' visibility via the `hidden` signal.
    let legend_el = if legend && series.iter().any(|s| !s.label.is_empty()) {
        let items: Vec<LegendItem> = series
            .iter()
            .enumerate()
            .map(|(i, s)| LegendItem {
                label: s.label.clone(),
                color: all_colors[i],
                value: legend_values.then(|| {
                    let total: f64 = s.values.iter().filter(|v| v.is_finite()).sum();
                    format_value(total)
                }),
                hidden: hidden_set.contains(&i),
            })
            .collect();
        let toggle: Rc<dyn Fn(usize)> = Rc::new(move |i: usize| {
            hidden.update(|set| {
                if !set.remove(&i) {
                    set.insert(i);
                }
            });
        });
        let vertical = matches!(legend_position, LegendPosition::Left | LegendPosition::Right);
        Some(legend_widget(items, vertical, Some(toggle)))
    } else {
        None
    };

    // Left/Right sit the legend beside the plot column; Top/Bottom stack it above/below.
    if let Some(el) = legend_el {
        match legend_position {
            LegendPosition::Bottom => {
                col.push(gap_h(14.0).into_widget());
                col.push(el);
            }
            LegendPosition::Top => {
                let mut stacked = vec![el, gap_h(14.0).into_widget()];
                stacked.append(&mut col);
                col = stacked;
            }
            LegendPosition::Left | LegendPosition::Right => {
                let chart = container()
                    .width(width)
                    .child(
                        column(col)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .main_axis_size(MainAxisSize::Min),
                    )
                    .into_widget();
                let lane = if legend_position == LegendPosition::Left {
                    row(children![el, gap_w(20.0), chart])
                } else {
                    row(children![chart, gap_w(20.0), el])
                };
                return lane
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .main_axis_size(MainAxisSize::Min)
                    .into_widget();
            }
        }
    }

    container()
        .width(width)
        .child(
            column(col)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .main_axis_size(MainAxisSize::Min),
        )
        .into_widget()
}

/// Apply the chart's configured font family to a label, if one is set.
fn apply_font(t: Text, font: &Option<String>) -> Text {
    match font {
        Some(f) => t.font_family(f.clone()),
        None => t,
    }
}

fn axis_title_row(
    left: Option<String>,
    right: Option<String>,
    left_axis_width: f64,
    plot_width: f64,
    right_axis_width: f64,
    color: Color,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    row(children![
        sized_box(
            left.map(|title| {
                apply_font(text(title).size(label_px).semibold().color(color), font)
                    .align(TextAlign::Right)
                    .into_widget()
            })
                .unwrap_or_else(|| gap_w(0.0).into_widget())
        )
        .width(left_axis_width),
        gap_w(0.0),
        sized_box(gap_w(0.0)).width(plot_width),
        sized_box(
            right
                .map(|title| {
                    apply_font(text(title).size(label_px).semibold().color(color), font)
                        .align(TextAlign::Left)
                        .into_widget()
                })
                .unwrap_or_else(|| gap_w(0.0).into_widget())
        )
        .width(right_axis_width),
    ])
    .into_widget()
}

fn category_label_widgets(
    categories: &[String],
    width: f64,
    mode: CategoryLabelMode,
    color: Color,
    label_px: f32,
    font: &Option<String>,
) -> Vec<AnyWidget> {
    let n = categories.len().max(1);
    let slot = width / n as f64;
    let every = match mode {
        CategoryLabelMode::Hidden => 1,
        CategoryLabelMode::Skip(n) => n.max(1),
        CategoryLabelMode::Auto if slot < 34.0 => 3,
        CategoryLabelMode::Auto if slot < 52.0 => 2,
        _ => 1,
    };
    let max_chars = match mode {
        CategoryLabelMode::Truncate(n) => n.max(1),
        CategoryLabelMode::Auto => (slot / 6.5).floor().clamp(3.0, 14.0) as usize,
        _ => usize::MAX,
    };
    categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let label = if mode == CategoryLabelMode::Hidden {
                String::new()
            } else if i % every == 0 {
                truncate_label(cat, max_chars)
            } else {
                String::new()
            };
            expanded(apply_font(text(label).size(label_px).color(color), font).align(TextAlign::Center))
                .into_widget()
        })
        .collect()
}

fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let keep = max_chars.saturating_sub(2).max(1);
    format!("{}..", label.chars().take(keep).collect::<String>())
}

fn reference_labels(
    lines: &[ReferenceLine],
    bands: &[ReferenceBand],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    color: Color,
) -> AnyWidget {
    let bottom = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    let scale = axis.scale(PLOT_TOP, bottom);
    let mut items = Vec::new();
    for line in lines {
        let Some(label) = &line.label else {
            continue;
        };
        let y = scale.map(line.value).clamp(PLOT_TOP, bottom);
        items.push(
            positioned(
                sized_box(
                    text(label.clone())
                        .size(10.0)
                        .semibold()
                        .color(color)
                        .align(TextAlign::Right),
                )
                .width(58.0),
            )
                .right(6.0)
                .top((y - 15.0).clamp(0.0, height - 16.0))
                .into_widget(),
        );
    }
    for band in bands {
        let Some(label) = &band.label else {
            continue;
        };
        let y0 = scale.map(band.start).clamp(PLOT_TOP, bottom);
        let y1 = scale.map(band.end).clamp(PLOT_TOP, bottom);
        let y = (y0 + y1) / 2.0;
        items.push(
            positioned(
                sized_box(
                    text(label.clone())
                        .size(10.0)
                        .semibold()
                        .color(color)
                        .align(TextAlign::Right),
                )
                .width(58.0),
            )
                .right(6.0)
                .top((y - 7.0).clamp(0.0, height - 16.0))
                .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(width)
        .height(height)
        .into_widget()
}

#[allow(clippy::too_many_arguments)]
fn cartesian_data_labels(
    kind: Kind,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    format_value: &Rc<dyn Fn(f64) -> String>,
    color: Color,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    let bottom = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    let scale = axis.scale(PLOT_TOP, bottom);
    let pw = (width - PLOT_LEFT - PLOT_RIGHT).max(1.0);
    let ncat = vals.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let nser = vals.len().max(1);
    let label_w = 42.0;
    let label_h = 14.0;
    let mut items = Vec::new();
    for (si, sv) in vals.iter().enumerate() {
        for (i, &v) in sv.iter().enumerate() {
            if !v.is_finite() {
                continue;
            }
            let (x, y, align) = match kind {
                Kind::HorizontalBar => {
                    let x_scale = LinearScale::new(axis.min, axis.max, PLOT_LEFT, PLOT_LEFT + pw);
                    let slot = (bottom - PLOT_TOP).max(1.0) / ncat as f64;
                    let group_h = slot * 0.7;
                    let bh = group_h / nser as f64;
                    let x = x_scale.map(v).clamp(PLOT_LEFT, PLOT_LEFT + pw);
                    (
                        if v >= 0.0 { x + 4.0 } else { x - label_w - 4.0 },
                        PLOT_TOP + slot * (i as f64 + 0.5) - group_h / 2.0 + si as f64 * bh
                            + bh / 2.0
                            - label_h / 2.0,
                        if v >= 0.0 { TextAlign::Left } else { TextAlign::Right },
                    )
                }
                _ => {
                    let slot = pw / ncat as f64;
                    let x = PLOT_LEFT + pw * (i as f64 + 0.5) / ncat as f64;
                    let x = if kind == Kind::Bar {
                        let group_w = slot * 0.7;
                        let bw = group_w / nser as f64;
                        x - group_w / 2.0 + si as f64 * bw + bw / 2.0
                    } else {
                        x
                    };
                    let y = scale.map(v).clamp(PLOT_TOP, bottom);
                    (x - label_w / 2.0, y - label_h - 5.0, TextAlign::Center)
                }
            };
            items.push(
                positioned(
                    sized_box(
                        apply_font(
                            text(format_value(v)).size((label_px - 1.0).max(6.0)).semibold().color(color),
                            font,
                        )
                        .align(align),
                    )
                    .width(label_w)
                    .height(label_h),
                )
                .left(x.clamp(0.0, width - label_w))
                .top(y.clamp(0.0, height - label_h))
                .into_widget(),
            );
        }
    }
    sized_box(stack(items))
        .width(width)
        .height(height)
        .into_widget()
}

/// Point callouts (dot + label) pinned to data points — the annotation overlay.
#[allow(clippy::too_many_arguments)]
fn annotation_layer(
    annotations: &[Annotation],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    ncat: usize,
    ref_color: Color,
    label_color: Color,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    let bottom = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    let scale = axis.scale(PLOT_TOP, bottom);
    let pw = (width - PLOT_LEFT - PLOT_RIGHT).max(1.0);
    let ncatf = ncat.max(1) as f64;
    let dot = 8.0;
    let lw = 120.0;
    let mut items = Vec::new();
    for a in annotations {
        if !a.value.is_finite() {
            continue;
        }
        let color = a.color.unwrap_or(ref_color);
        let cx = PLOT_LEFT + pw * (a.category as f64 + 0.5) / ncatf;
        let cy = scale.map(a.value).clamp(PLOT_TOP, bottom);
        items.push(
            positioned(container().width(dot).height(dot).decoration(
                BoxDecoration::new().color(color).radius(BorderRadius::all(dot / 2.0)),
            ))
            .left(cx - dot / 2.0)
            .top(cy - dot / 2.0)
            .into_widget(),
        );
        items.push(
            positioned(
                sized_box(
                    apply_font(text(a.label.clone()).size(label_px).semibold().color(label_color), font)
                        .align(TextAlign::Center),
                )
                .width(lw)
                .height(16.0),
            )
            .left((cx - lw / 2.0).clamp(0.0, width - lw))
            .top((cy - dot / 2.0 - 18.0).clamp(0.0, height - 16.0))
            .into_widget(),
        );
    }
    sized_box(stack(items)).width(width).height(height).into_widget()
}

fn y_axis_labels(
    axis: &ValueAxis,
    labels: Vec<String>,
    height: f64,
    color: Color,
    align: TextAlign,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    let bottom = (height - PLOT_BOTTOM).max(PLOT_TOP + 1.0);
    let scale = axis.scale(PLOT_TOP, bottom);
    let mut items = Vec::new();
    for (&tick, label) in axis.ticks.iter().zip(labels.into_iter()) {
        let y = scale.map(tick);
        items.push(
            positioned(apply_font(text(label).size(label_px).color(color), font).align(align))
                .left(0.0)
                .right(0.0)
                .top((y - 7.0).clamp(0.0, height - 14.0))
                .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(Y_AXIS_WIDTH)
        .height(height)
        .into_widget()
}

// ---------------------------------------------------------------------------
// Scatter / Bubble
// ---------------------------------------------------------------------------

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

pub struct ScatterChart {
    series: Vec<PointSeries>,
    width: f64,
    height: f64,
    legend: bool,
    bubble: bool,
    x_range: Option<(f64, f64)>,
    y_range: Option<(f64, f64)>,
    x_scale: AxisScale,
    tick_count: usize,
}

pub type BubbleChart = ScatterChart;

pub fn scatter_chart(series: Vec<PointSeries>) -> ScatterChart {
    ScatterChart {
        series,
        width: 520.0,
        height: 260.0,
        legend: true,
        bubble: false,
        x_range: None,
        y_range: None,
        x_scale: AxisScale::Linear,
        tick_count: 5,
    }
}

pub fn bubble_chart(series: Vec<PointSeries>) -> BubbleChart {
    ScatterChart {
        series,
        width: 520.0,
        height: 260.0,
        legend: true,
        bubble: true,
        x_range: None,
        y_range: None,
        x_scale: AxisScale::Linear,
        tick_count: 5,
    }
}

impl ScatterChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.x_range = Some((min, max));
        self
    }
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }
    pub fn x_scale(mut self, scale: AxisScale) -> Self {
        self.x_scale = scale;
        self
    }
    pub fn x_log(mut self) -> Self {
        self.x_scale = AxisScale::Log10;
        self
    }
    pub fn x_time(mut self) -> Self {
        self.x_scale = AxisScale::Time;
        self
    }
    pub fn tick_count(mut self, count: usize) -> Self {
        self.tick_count = count.max(2);
        self
    }
}

impl IntoWidget for ScatterChart {
    fn into_widget(self) -> AnyWidget {
        let ScatterChart {
            series,
            width,
            height,
            legend,
            bubble,
            x_range,
            y_range,
            x_scale,
            tick_count,
        } = self;
        let colors: Vec<Color> = series
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or(palette_color(i)))
            .collect();
        let points: Vec<Vec<ScatterPoint>> = series.iter().map(|s| s.points.clone()).collect();
        let xs = points.iter().flatten().map(|p| p.x).collect::<Vec<_>>();
        let ys = points.iter().flatten().map(|p| p.y).collect::<Vec<_>>();
        let x_axis = XValueAxis::from_values(&xs, x_range, tick_count, x_scale);
        let y_axis = ValueAxis::from_values(&[ys], y_range, tick_count);
        let grid_c = with_alpha(theme().colors.muted_foreground, 0.16);
        let label_c = theme().colors.muted_foreground;
        let draw_colors = colors.clone();

        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let x0 = PLOT_LEFT;
            let x1 = s.width - PLOT_RIGHT;
            let y0 = PLOT_TOP;
            let y1 = s.height - PLOT_BOTTOM;
            let y_scale = y_axis.scale(y0, y1);
            for &tick in &y_axis.ticks {
                let y = y_scale.map(tick);
                c.stroke_line(Offset::new(x0, y), Offset::new(x1, y), 1.0, grid_c);
            }
            for &tick in &x_axis.ticks {
                if let Some(x) = x_axis.map(tick, x0, x1) {
                    c.stroke_line(Offset::new(x, y0), Offset::new(x, y1), 1.0, grid_c);
                }
            }
            for (si, sv) in points.iter().enumerate() {
                for p in sv {
                    if p.x.is_finite() && p.y.is_finite() {
                        let Some(x) = x_axis.map(p.x, x0, x1) else {
                            continue;
                        };
                        let r = if bubble { p.radius } else { 3.0 };
                        c.fill_circle(Offset::new(x, y_scale.map(p.y)), r, draw_colors[si]);
                    }
                }
            }
        })
        .width(width)
        .height(height);

        let mut col = vec![plot.into_widget()];
        if legend && series.iter().any(|s| !s.label.is_empty()) {
            col.push(gap_h(14.0).into_widget());
            col.push(legend_row(
                series
                    .iter()
                    .map(|s| s.label.clone())
                    .zip(colors.iter().cloned())
                    .collect(),
            ));
        } else {
            col.push(text("").size(1.0).color(label_c).into_widget());
        }
        container()
            .width(width)
            .child(column(col).main_axis_size(MainAxisSize::Min))
            .into_widget()
    }
}

// ---------------------------------------------------------------------------
// Radar / Gauge / Progress
// ---------------------------------------------------------------------------

pub struct RadarChart {
    categories: Vec<String>,
    series: Vec<Series>,
    size: f64,
    legend: bool,
}

pub fn radar_chart(categories: Vec<String>, series: Vec<Series>) -> RadarChart {
    RadarChart {
        categories,
        series,
        size: 280.0,
        legend: true,
    }
}

impl RadarChart {
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
}

impl IntoWidget for RadarChart {
    fn into_widget(self) -> AnyWidget {
        let RadarChart {
            categories,
            series,
            size,
            legend,
        } = self;
        let colors: Vec<Color> = series
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or(palette_color(i)))
            .collect();
        let vals: Vec<Vec<f64>> = series.iter().map(|s| s.values.clone()).collect();
        let max = vals
            .iter()
            .flat_map(|v| v.iter())
            .copied()
            .filter(|v| v.is_finite())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let n = categories.len().max(3);
        let draw_colors = colors.clone();
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let cx = s.width / 2.0;
            let cy = s.height / 2.0;
            let r = s.width.min(s.height) / 2.0 - 12.0;
            let grid_c = with_alpha(theme().colors.muted_foreground, 0.18);
            for ring in 1..=4 {
                let rr = r * ring as f64 / 4.0;
                let mut p = BezPath::new();
                for i in 0..n {
                    let a =
                        -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * i as f64 / n as f64;
                    let pt = (cx + rr * a.cos(), cy + rr * a.sin());
                    if i == 0 {
                        p.move_to(pt);
                    } else {
                        p.line_to(pt);
                    }
                }
                p.close_path();
                c.stroke_path(&p, 1.0, grid_c);
            }
            for i in 0..n {
                let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * i as f64 / n as f64;
                c.stroke_line(
                    Offset::new(cx, cy),
                    Offset::new(cx + r * a.cos(), cy + r * a.sin()),
                    1.0,
                    grid_c,
                );
            }
            for (si, sv) in vals.iter().enumerate() {
                let mut p = BezPath::new();
                for i in 0..n {
                    let value = sv.get(i).copied().unwrap_or(0.0).max(0.0);
                    let rr = r * (value / max).clamp(0.0, 1.0);
                    let a =
                        -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * i as f64 / n as f64;
                    let pt = (cx + rr * a.cos(), cy + rr * a.sin());
                    if i == 0 {
                        p.move_to(pt);
                    } else {
                        p.line_to(pt);
                    }
                }
                p.close_path();
                c.fill_path(&p, with_alpha(draw_colors[si], 0.18));
                c.stroke_path(&p, 2.0, draw_colors[si]);
            }
        })
        .width(size)
        .height(size);
        let mut col = vec![center(plot).into_widget()];
        if legend {
            col.push(gap_h(14.0).into_widget());
            col.push(legend_row(
                series
                    .iter()
                    .map(|s| s.label.clone())
                    .zip(colors.iter().cloned())
                    .collect(),
            ));
        }
        column(col)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min)
            .into_widget()
    }
}

pub struct RadialProgressChart {
    label: String,
    value: f64,
    max: f64,
    size: f64,
    thickness: f64,
    gauge: bool,
    color: Option<Color>,
}

pub type GaugeChart = RadialProgressChart;

pub fn progress_ring(label: impl Into<String>, value: f64, max: f64) -> RadialProgressChart {
    RadialProgressChart {
        label: label.into(),
        value,
        max,
        size: 220.0,
        thickness: 18.0,
        gauge: false,
        color: None,
    }
}

pub fn gauge_chart(label: impl Into<String>, value: f64, max: f64) -> GaugeChart {
    RadialProgressChart {
        label: label.into(),
        value,
        max,
        size: 220.0,
        thickness: 18.0,
        gauge: true,
        color: None,
    }
}

impl RadialProgressChart {
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness.max(1.0);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl IntoWidget for RadialProgressChart {
    fn into_widget(self) -> AnyWidget {
        let RadialProgressChart {
            label,
            value,
            max,
            size,
            thickness,
            gauge,
            color,
        } = self;
        let progress = if max > 0.0 {
            (value / max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let color = color.unwrap_or_else(|| palette_color(0));
        let track = with_alpha(theme().colors.muted_foreground, 0.16);
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let cx = s.width / 2.0;
            let cy = s.height / 2.0;
            let r = s.width.min(s.height) / 2.0 - thickness;
            let start = if gauge {
                std::f64::consts::PI * 0.82
            } else {
                -std::f64::consts::FRAC_PI_2
            };
            let total = if gauge {
                std::f64::consts::PI * 1.36
            } else {
                std::f64::consts::TAU
            };
            c.stroke_path(
                &arc_path(cx, cy, r, start, start + total, 96),
                thickness,
                track,
            );
            c.stroke_path(
                &arc_path(cx, cy, r, start, start + total * progress, 96),
                thickness,
                color,
            );
        })
        .width(size)
        .height(size);
        column(children![
            center(plot).into_widget(),
            gap_h(8.0),
            text(label)
                .size(12.0)
                .color(theme().colors.muted_foreground)
                .align(TextAlign::Center),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .main_axis_size(MainAxisSize::Min)
        .into_widget()
    }
}

fn arc_path(cx: f64, cy: f64, r: f64, a0: f64, a1: f64, steps: usize) -> BezPath {
    let mut p = BezPath::new();
    let sweep = a1 - a0;
    let steps = steps.max(2);
    for k in 0..=steps {
        let a = a0 + sweep * k as f64 / steps as f64;
        let pt = (cx + r * a.cos(), cy + r * a.sin());
        if k == 0 {
            p.move_to(pt);
        } else {
            p.line_to(pt);
        }
    }
    p
}

// ---------------------------------------------------------------------------
// Financial / Dense / Flow Charts
// ---------------------------------------------------------------------------

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

pub struct CandlestickChart {
    categories: Vec<String>,
    candles: Vec<Candle>,
    width: f64,
    height: f64,
    ohlc: bool,
}

pub type OhlcChart = CandlestickChart;

pub fn candlestick_chart(categories: Vec<String>, candles: Vec<Candle>) -> CandlestickChart {
    CandlestickChart {
        categories,
        candles,
        width: 520.0,
        height: 260.0,
        ohlc: false,
    }
}

pub fn ohlc_chart(categories: Vec<String>, candles: Vec<Candle>) -> OhlcChart {
    CandlestickChart {
        categories,
        candles,
        width: 520.0,
        height: 260.0,
        ohlc: true,
    }
}

impl CandlestickChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
}

impl IntoWidget for CandlestickChart {
    fn into_widget(self) -> AnyWidget {
        let CandlestickChart {
            categories,
            candles,
            width,
            height,
            ohlc,
        } = self;
        let vals = vec![
            candles
                .iter()
                .flat_map(|c| [c.open, c.high, c.low, c.close])
                .collect::<Vec<_>>(),
        ];
        let axis = ValueAxis::from_values(&vals, None, 5);
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let x0 = PLOT_LEFT;
            let x1 = s.width - PLOT_RIGHT;
            let y0 = PLOT_TOP;
            let y1 = s.height - PLOT_BOTTOM;
            let pw = (x1 - x0).max(1.0);
            let n = candles.len().max(1);
            let slot = pw / n as f64;
            let y_scale = axis.scale(y0, y1);
            let up = Color::from_rgba8(0x22, 0xC5, 0x5E, 0xFF);
            let down = Color::from_rgba8(0xF4, 0x3F, 0x5E, 0xFF);
            for (i, candle) in candles.iter().enumerate() {
                let x = x0 + slot * (i as f64 + 0.5);
                let color = if candle.close >= candle.open {
                    up
                } else {
                    down
                };
                c.stroke_line(
                    Offset::new(x, y_scale.map(candle.low)),
                    Offset::new(x, y_scale.map(candle.high)),
                    1.4,
                    color,
                );
                let open = y_scale.map(candle.open);
                let close = y_scale.map(candle.close);
                if ohlc {
                    c.stroke_line(
                        Offset::new(x - slot * 0.25, open),
                        Offset::new(x, open),
                        2.0,
                        color,
                    );
                    c.stroke_line(
                        Offset::new(x, close),
                        Offset::new(x + slot * 0.25, close),
                        2.0,
                        color,
                    );
                } else {
                    c.fill_rrect(
                        Rect::new(
                            x - slot * 0.24,
                            open.min(close),
                            x + slot * 0.24,
                            open.max(close).max(open.min(close) + 1.0),
                        ),
                        2.0,
                        color,
                    );
                }
            }
        })
        .width(width)
        .height(height);
        chart_with_categories(plot, categories, theme().colors.muted_foreground)
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

pub struct HeatmapChart {
    x_categories: Vec<String>,
    y_categories: Vec<String>,
    cells: Vec<HeatCell>,
    width: f64,
    height: f64,
}

pub fn heatmap_chart(
    x_categories: Vec<String>,
    y_categories: Vec<String>,
    cells: Vec<HeatCell>,
) -> HeatmapChart {
    HeatmapChart {
        x_categories,
        y_categories,
        cells,
        width: 520.0,
        height: 260.0,
    }
}

impl HeatmapChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
}

impl IntoWidget for HeatmapChart {
    fn into_widget(self) -> AnyWidget {
        let HeatmapChart {
            x_categories,
            y_categories,
            cells,
            width,
            height,
        } = self;
        let max = cells
            .iter()
            .map(|c| c.value)
            .filter(|v| v.is_finite())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let nx = x_categories.len().max(1);
        let ny = y_categories.len().max(1);
        let base = palette_color(0);
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let cell_w = s.width / nx as f64;
            let cell_h = s.height / ny as f64;
            for cell in &cells {
                if cell.x < nx && cell.y < ny && cell.value.is_finite() {
                    let a = (0.12 + 0.78 * (cell.value / max).clamp(0.0, 1.0)) as f32;
                    c.fill_rrect(
                        Rect::new(
                            cell.x as f64 * cell_w + 1.0,
                            cell.y as f64 * cell_h + 1.0,
                            (cell.x + 1) as f64 * cell_w - 1.0,
                            (cell.y + 1) as f64 * cell_h - 1.0,
                        ),
                        3.0,
                        with_alpha(base, a),
                    );
                }
            }
        })
        .width(width)
        .height(height);
        chart_with_categories(plot, x_categories, theme().colors.muted_foreground)
    }
}

pub struct FunnelChart {
    slices: Vec<Slice>,
    width: f64,
    height: f64,
    legend: bool,
}

pub fn funnel_chart(slices: Vec<Slice>) -> FunnelChart {
    FunnelChart {
        slices,
        width: 360.0,
        height: 260.0,
        legend: true,
    }
}

impl FunnelChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
}

impl IntoWidget for FunnelChart {
    fn into_widget(self) -> AnyWidget {
        let FunnelChart {
            slices,
            width,
            height,
            legend,
        } = self;
        let colors: Vec<Color> = slices
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or(palette_color(i)))
            .collect();
        let values: Vec<f64> = slices.iter().map(|s| s.value.max(0.0)).collect();
        let max = values.iter().copied().fold(0.0_f64, f64::max).max(1.0);
        let draw_colors = colors.clone();
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let n = values.len().max(1);
            let h = s.height / n as f64;
            for (i, value) in values.iter().enumerate() {
                let w0 = s.width * (value / max).clamp(0.08, 1.0);
                let w1 = values
                    .get(i + 1)
                    .map(|v| s.width * (v / max).clamp(0.08, 1.0))
                    .unwrap_or(w0 * 0.75);
                let y0 = i as f64 * h + 2.0;
                let y1 = (i + 1) as f64 * h - 2.0;
                let mut p = BezPath::new();
                p.move_to(((s.width - w0) / 2.0, y0));
                p.line_to(((s.width + w0) / 2.0, y0));
                p.line_to(((s.width + w1) / 2.0, y1));
                p.line_to(((s.width - w1) / 2.0, y1));
                p.close_path();
                c.fill_path(&p, draw_colors[i]);
            }
        })
        .width(width)
        .height(height);
        let mut col = vec![center(plot).into_widget()];
        if legend {
            col.push(gap_h(14.0).into_widget());
            col.push(legend_row(
                slices
                    .iter()
                    .map(|s| s.label.clone())
                    .zip(colors.iter().cloned())
                    .collect(),
            ));
        }
        column(col)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min)
            .into_widget()
    }
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

pub struct SankeyChart {
    links: Vec<SankeyLink>,
    width: f64,
    height: f64,
}

pub fn sankey_chart(links: Vec<SankeyLink>) -> SankeyChart {
    SankeyChart {
        links,
        width: 520.0,
        height: 260.0,
    }
}

impl SankeyChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
}

impl IntoWidget for SankeyChart {
    fn into_widget(self) -> AnyWidget {
        let SankeyChart {
            links,
            width,
            height,
        } = self;
        let max = links
            .iter()
            .map(|l| l.value)
            .filter(|v| v.is_finite())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let n = links.len().max(1);
            let gap = s.height / n as f64;
            for (i, link) in links.iter().enumerate() {
                if !link.value.is_finite() {
                    continue;
                }
                let y = gap * (i as f64 + 0.5);
                let stroke = 4.0 + 22.0 * (link.value / max).clamp(0.0, 1.0);
                let color = with_alpha(palette_color(i), 0.48);
                let mut p = BezPath::new();
                p.move_to((18.0, y));
                p.curve_to(
                    (s.width * 0.38, y),
                    (s.width * 0.62, y + gap * 0.22),
                    (s.width - 18.0, y + gap * 0.22),
                );
                c.stroke_path(&p, stroke, color);
                c.fill_rrect(
                    Rect::new(4.0, y - 12.0, 32.0, y + 12.0),
                    4.0,
                    palette_color(i),
                );
                c.fill_rrect(
                    Rect::new(
                        s.width - 32.0,
                        y + gap * 0.22 - 12.0,
                        s.width - 4.0,
                        y + gap * 0.22 + 12.0,
                    ),
                    4.0,
                    palette_color(i + 1),
                );
            }
        })
        .width(width)
        .height(height);
        center(plot).into_widget()
    }
}

fn chart_with_categories(plot: CanvasWidget, categories: Vec<String>, color: Color) -> AnyWidget {
    column(children![
        plot,
        gap_h(6.0),
        row(categories
            .iter()
            .map(|cat| expanded(
                text(cat.clone())
                    .size(11.0)
                    .color(color)
                    .align(TextAlign::Center)
            )
            .into_widget())
            .collect::<Vec<_>>()),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .main_axis_size(MainAxisSize::Min)
    .into_widget()
}

// ---------------------------------------------------------------------------
// Pie / Donut
// ---------------------------------------------------------------------------

/// A **pie** or **donut** chart. Built with [`pie_chart`] / [`donut_chart`].
pub struct PieChart {
    slices: Vec<Slice>,
    size: f64,
    hole: f64, // 0.0 = pie; 0.0..1.0 = donut inner-radius fraction
    legend: bool,
    data_labels: bool,
    legend_position: LegendPosition,
    legend_values: bool,
    tooltip: bool,
    on_slice: Option<Rc<dyn Fn(usize, f64)>>,
    // None = auto (honor the OS reduced-motion preference); Some(_) = explicit override.
    animate: Option<bool>,
    animation_ms: u32,
    palette: Option<Vec<Color>>,
    a11y_label: Option<String>,
    style: ChartStyle,
    loading: bool,
    error: Option<String>,
}

/// A **pie chart**.
pub fn pie_chart(slices: Vec<Slice>) -> PieChart {
    PieChart {
        slices,
        size: 260.0,
        hole: 0.0,
        legend: true,
        data_labels: false,
        legend_position: LegendPosition::Bottom,
        legend_values: false,
        tooltip: true,
        on_slice: None,
        animate: None,
        animation_ms: 700,
        palette: None,
        a11y_label: None,
        style: ChartStyle::new(),
        loading: false,
        error: None,
    }
}
/// A **donut chart** (pie with a hole).
pub fn donut_chart(slices: Vec<Slice>) -> PieChart {
    PieChart {
        slices,
        size: 260.0,
        hole: 0.58,
        legend: true,
        data_labels: false,
        legend_position: LegendPosition::Bottom,
        legend_values: false,
        tooltip: true,
        on_slice: None,
        animate: None,
        animation_ms: 700,
        palette: None,
        a11y_label: None,
        style: ChartStyle::new(),
        loading: false,
        error: None,
    }
}

impl PieChart {
    pub fn size(mut self, px: f64) -> Self {
        self.size = px;
        self
    }
    /// The hole radius as a fraction of the outer radius (0 = solid pie).
    pub fn hole(mut self, frac: f64) -> Self {
        self.hole = frac.clamp(0.0, 0.95);
        self
    }
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
    pub fn data_labels(mut self, on: bool) -> Self {
        self.data_labels = on;
        self
    }
    /// Place the legend `Bottom` (default), `Top`, `Left`, or `Right` of the chart.
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }
    /// Show each slice's percentage next to its name in the legend.
    pub fn legend_values(mut self, on: bool) -> Self {
        self.legend_values = on;
        self
    }
    /// Show a value tooltip and pop the active slice out when hovered/tapped (default true).
    pub fn tooltip(mut self, on: bool) -> Self {
        self.tooltip = on;
        self
    }
    /// Run `callback(slice_index, value)` when a slice is tapped.
    pub fn on_slice(mut self, callback: impl Fn(usize, f64) + 'static) -> Self {
        self.on_slice = Some(Rc::new(callback));
        self
    }
    /// Override the categorical palette for this chart (cycled per slice). Pass
    /// [`cvd_palette`](crate::cvd_palette)`().to_vec()` for a colorblind-safe ramp, or any
    /// custom `Vec<Color>`. Per-slice `.color(..)` still wins over the palette.
    pub fn palette(mut self, colors: Vec<Color>) -> Self {
        self.palette = if colors.is_empty() { None } else { Some(colors) };
        self
    }
    /// Animate the wedges in on mount (a radial sweep) and tween on data change. By default
    /// this **auto-honors the OS reduced-motion preference**; call `.animate(true)`/`(false)`
    /// to force it regardless.
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = Some(on);
        self
    }
    /// Entry-animation duration in milliseconds (default 700).
    pub fn animation_ms(mut self, ms: u32) -> Self {
        self.animation_ms = ms;
        self
    }
    /// Set the accessible summary a screen reader announces (e.g. "Traffic by browser").
    /// If unset, a summary is generated from the chart type and slice count. The per-slice
    /// values + percentages are always read out as the node's value.
    pub fn a11y_label(mut self, label: impl Into<String>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }
    /// Override the per-slot visual style (label color/size/font, grid + track colors) — the
    /// theme-as-config surface. See [`ChartStyle`](crate::ChartStyle).
    pub fn style(mut self, style: ChartStyle) -> Self {
        self.style = style;
        self
    }
    /// Show a loading placeholder instead of the ring — for data that hasn't arrived yet.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// Show an error placeholder with `message` instead of the ring — for a failed load.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }
}

/// Build the accessibility (role/label/value) for a pie/donut: a spoken summary plus a
/// per-slice value + percentage read-out — the chart's data-table fallback for a screen
/// reader.
fn pie_a11y(chart: &PieChart) -> (String, String) {
    let kind = if chart.hole > 0.0 { "Donut" } else { "Pie" };
    let total = chart
        .slices
        .iter()
        .map(|s| s.value.max(0.0))
        .filter(|v| v.is_finite())
        .sum::<f64>()
        .max(f64::MIN_POSITIVE);
    let label = chart
        .a11y_label
        .clone()
        .unwrap_or_else(|| format!("{kind} chart, {} slices", chart.slices.len()));
    let parts: Vec<String> = chart
        .slices
        .iter()
        .filter(|s| s.value.is_finite() && s.value > 0.0)
        .map(|s| format!("{} {} ({:.0}%)", s.label, compact_number(s.value), s.value / total * 100.0))
        .collect();
    let value = if parts.is_empty() { "No data".to_string() } else { parts.join(", ") };
    (label, value)
}

impl IntoWidget for PieChart {
    fn into_widget(self) -> AnyWidget {
        // Emit an accessibility node (role + summary + per-slice read-out) around the
        // canvas so the chart is legible to a screen reader. See `pie_a11y`.
        let (label, value) = pie_a11y(&self);
        // A component so the legend's toggle `create_signal` gets its own scope.
        let chart = component_props(render_pie_chart, self);
        semantics(SemanticsRole::Image, label, chart).value(value).into_widget()
    }
}

/// Props for the interactive pie plot sub-component. It exists so `use_bounds()` reports
/// the PLOT's own rect — letting a hover's global position map to slice-local coords.
struct PiePlotProps {
    plot: AnyWidget,
    size: f64,
    activate: Rc<dyn Fn(Offset, Offset)>, // (local, global) — hover + press + drag
    tap: Rc<dyn Fn(Offset)>,              // (local) — click
    clear: Rc<dyn Fn()>,                  // hover-exit / drag-end
}

fn render_pie_plot(p: &PiePlotProps) -> AnyWidget {
    let bounds = use_bounds(); // this component IS the sized plot → its screen rect
    let size = p.size;
    let (activate, tap, clear) = (p.activate.clone(), p.tap.clone(), p.clear.clone());
    // Hover position arrives in window space; subtract the plot's origin for local coords.
    let hover = {
        let activate = activate.clone();
        move |e: PointerEvent| {
            let local = Offset::new(e.global.x - bounds.x0, e.global.y - bounds.y0);
            activate(local, e.global);
        }
    };
    GestureDetector::new(sized_box(p.plot.clone()).width(size).height(size))
        .cursor(Cursor::Pointer)
        .on_hover_enter(action_event(hover.clone()))
        .on_hover_move(action_event(hover))
        .on_hover_exit({
            let clear = clear.clone();
            move || clear()
        })
        .on_pointer_down(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_start(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_update(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_end({
            let clear = clear.clone();
            move || clear()
        })
        .on_tap(action_event(move |e: PointerEvent| tap(e.position)))
        .into_widget()
}

/// Which slice (by VISIBLE index) the local pointer falls in, or None if it's outside
/// the ring. Walks the same start angle / sweep order the draw uses, so the hit matches
/// exactly what's painted.
fn hit_pie_slice(local: Offset, size: f64, hole: f64, values: &[f64], total: f64) -> Option<usize> {
    let center = size / 2.0;
    let r = size / 2.0 - 6.0;
    let ir = r * hole;
    let (dx, dy) = (local.x - center, local.y - center);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > r || dist < ir {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let rel = (dy.atan2(dx) - (-std::f64::consts::FRAC_PI_2)).rem_euclid(tau);
    let mut acc = 0.0;
    for (i, &v) in values.iter().enumerate() {
        let sweep = v.max(0.0) / total * tau;
        if rel >= acc && rel < acc + sweep {
            return Some(i);
        }
        acc += sweep;
    }
    (!values.is_empty()).then(|| values.len() - 1)
}

/// A passive value tooltip for a hovered/tapped pie or donut slice.
fn show_pie_tooltip(global: Offset, label: &str, color: Color, value_str: String, pct: f64) {
    let title = if label.is_empty() {
        "Slice".to_string()
    } else {
        label.to_string()
    };
    show_passive(
        cartesian_tooltip(
            title,
            vec![(String::new(), color, format!("{value_str}  ·  {pct:.0}%"), true)],
        ),
        global.x + TOOLTIP_OFFSET,
        global.y + TOOLTIP_OFFSET,
    );
}

fn render_pie_chart(chart: &PieChart) -> AnyWidget {
    let slices = chart.slices.clone();
    let size = chart.size;
    // Loading / error take priority over the ring and the empty state.
    if let Some(msg) = &chart.error {
        return empty_placeholder(size, size, msg);
    }
    if chart.loading {
        return empty_placeholder(size, size, "Loading…");
    }
    let hole = chart.hole;
    let legend = chart.legend;
    let data_labels = chart.data_labels;
    let legend_position = chart.legend_position;
    let legend_values = chart.legend_values;
    let tooltip = chart.tooltip;
    let on_slice = chart.on_slice.clone();
    let animate = chart.animate.unwrap_or(!prefers_reduced_motion());
    let animation_ms = chart.animation_ms;
    let palette_override = chart.palette.clone();
    let pal_color = move |i: usize| -> Color {
        match &palette_override {
            Some(p) if !p.is_empty() => p[i % p.len()],
            _ => palette_color(i),
        }
    };

    // Empty state: no slices, or every slice is zero/negative (nothing to draw). Render a
    // calm "no data" panel at the chart's footprint rather than an empty ring.
    let has_data = slices.iter().any(|s| s.value.is_finite() && s.value > 0.0);
    if !has_data {
        return empty_placeholder(size, size, "No data");
    }

    // Entry animation: a 0→1 progress kicked once on mount; the wedges sweep in radially
    // (each slice's swept angle scales by `anim_t`).
    let anim = create_signal(0.0_f64);
    let anim_kicked = create_signal(false);
    if animate && !anim_kicked.peek() {
        anim_kicked.set(true);
        pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
    }
    let anim_t = if animate { anim.get() } else { 1.0 };

    // Slices toggled off from the legend (by original index). Hiding a slice drops it
    // from the pie and the remaining slices re-proportion to fill (total = visible sum),
    // so the drawn angles and the legend percentages always agree.
    let hidden = create_signal(HashSet::<usize>::new());
    let hidden_set = hidden.get();
    // The slice under the pointer (by visible index): pops out + drives the tooltip.
    let active_slice = create_signal(None::<usize>);
    let active_slice_val = active_slice.get();

    let all_colors: Vec<Color> = slices
        .iter()
        .enumerate()
        .map(|(i, s)| s.color.unwrap_or_else(|| pal_color(i)))
        .collect();
    let visible: Vec<usize> = (0..slices.len())
        .filter(|&i| !hidden_set.contains(&i) && slices[i].value.max(0.0) > 0.0)
        .collect();
    let vis_slices: Vec<Slice> = visible.iter().map(|&i| slices[i].clone()).collect();
    let vis_colors: Vec<Color> = visible.iter().map(|&i| all_colors[i]).collect();

    // Data-change tween: when the slice values change on a later render (same slice count),
    // morph each wedge from its previous value to the new one so the ring re-proportions
    // smoothly. Snapshots are kept by ORIGINAL slice index so a legend toggle (which only
    // changes the *visible* set) never reads as a data change. Same one-shot idiom as the
    // entry sweep. Labels + hit-testing always read the target values.
    let full_target: Vec<f64> = slices.iter().map(|s| s.value.max(0.0)).collect();
    let data_t = create_signal(1.0_f64);
    let last_full = create_signal(None::<Vec<f64>>);
    let from_full = create_signal(Vec::<f64>::new());
    {
        let prev = last_full.peek();
        let same_shape = prev.as_ref().is_some_and(|p| p.len() == full_target.len());
        let changed = prev.as_ref().is_some_and(|p| p != &full_target);
        if changed && same_shape && animate {
            from_full.set(prev.clone().unwrap());
            data_t.set(0.0);
            pebbles::core::animation::animate_to(data_t, 1.0, animation_ms as f64 / 1000.0);
        } else if prev.is_none() {
            from_full.set(full_target.clone());
        } else if changed {
            from_full.set(full_target.clone());
            data_t.set(1.0);
            if animate {
                anim.set(0.0);
                pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
            }
        }
        last_full.set(Some(full_target.clone()));
    }
    let data_t_val = if animate { data_t.get() } else { 1.0 };
    let morph_from = from_full.peek();

    let target_values: Vec<f64> = visible.iter().map(|&oi| full_target[oi]).collect();
    // Morphed values drive the drawn wedges + total; labels/hit-tests use the target set.
    let values: Vec<f64> = visible
        .iter()
        .map(|&oi| {
            let tv = full_target[oi];
            let fv = morph_from.get(oi).copied().unwrap_or(tv);
            if data_t_val >= 1.0 { tv } else { fv + (tv - fv) * data_t_val }
        })
        .collect();
    let draw_colors = vis_colors.clone();
    let total = values.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let label_values = target_values.clone();
    let label_slices = vis_slices.clone();
    let label_colors = vis_colors.clone();
    let hit_values = target_values.clone();

    let plot = canvas(move |c: &mut Canvas<'_>| {
        let s = c.size();
        let cx0 = s.width / 2.0;
        let cy0 = s.height / 2.0;
        let r = (s.width.min(s.height) / 2.0) - 6.0;
        let ir = r * hole;
        let mut a0 = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
        for (i, &v) in values.iter().enumerate() {
            // Entry sweep: scale each wedge's angle by anim_t so the ring unfolds 0→full.
            let sweep = v / total * std::f64::consts::TAU * anim_t;
            let a1 = a0 + sweep;
            // The active slice pops OUT along its mid-angle for emphasis.
            let mid = a0 + sweep / 2.0;
            let (cx, cy) = if active_slice_val == Some(i) {
                (cx0 + 8.0 * mid.cos(), cy0 + 8.0 * mid.sin())
            } else {
                (cx0, cy0)
            };
            let steps = ((sweep / std::f64::consts::TAU) * 96.0).ceil().max(2.0) as usize;
            let mut p = BezPath::new();
            if ir <= 0.5 {
                p.move_to((cx, cy));
                for k in 0..=steps {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    p.line_to((cx + r * a.cos(), cy + r * a.sin()));
                }
            } else {
                // annulus sector: outer arc forward, inner arc back
                for k in 0..=steps {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    let pt = (cx + r * a.cos(), cy + r * a.sin());
                    if k == 0 {
                        p.move_to(pt);
                    } else {
                        p.line_to(pt);
                    }
                }
                for k in (0..=steps).rev() {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    p.line_to((cx + ir * a.cos(), cy + ir * a.sin()));
                }
            }
            p.close_path();
            c.fill_path(&p, draw_colors[i]);
            a0 = a1;
        }
    })
    .width(size)
    .height(size);

    // Hold slice labels until the sweep finishes (they're placed by full mid-angle).
    let plot = if data_labels && anim_t >= 0.999 && data_t_val >= 0.999 {
        sized_box(stack(children![
            plot.into_widget(),
            pie_data_labels(
                &label_slices,
                &label_values,
                &label_colors,
                total,
                size,
                hole,
                chart.style.label_px(),
                &chart.style.font_family,
            )
        ]))
        .width(size)
        .height(size)
        .into_widget()
    } else {
        plot.into_widget()
    };

    // Interaction: hover (continuous) / press / drag / tap. Hits a slice → pops it out +
    // shows a value tooltip; tap also fires `on_slice`. The gesture lives in its own
    // sub-component (`render_pie_plot`) so `use_bounds()` gives the plot's exact screen
    // rect — needed to turn a hover's global position into slice-local coordinates.
    let activate: Rc<dyn Fn(Offset, Offset)> = {
        let vis_slices = vis_slices.clone();
        let vis_colors = vis_colors.clone();
        Rc::new(move |local: Offset, global: Offset| {
            match hit_pie_slice(local, size, hole, &hit_values, total) {
                Some(si) => {
                    active_slice.set(Some(si));
                    if tooltip {
                        if let Some(s) = vis_slices.get(si) {
                            let value = s.value.max(0.0);
                            show_pie_tooltip(
                                global,
                                &s.label,
                                vis_colors.get(si).copied().unwrap_or(palette_color(si)),
                                compact_number(value),
                                value / total * 100.0,
                            );
                        }
                    }
                }
                None => {
                    active_slice.set(None);
                    hide_passive();
                }
            }
        })
    };
    let clear: Rc<dyn Fn()> = Rc::new(move || {
        active_slice.set(None);
        hide_passive();
    });
    let tap: Rc<dyn Fn(Offset)> = {
        let tap_values: Vec<f64> = vis_slices.iter().map(|s| s.value.max(0.0)).collect();
        let tap_slices = vis_slices.clone();
        Rc::new(move |local: Offset| {
            if let Some(si) = hit_pie_slice(local, size, hole, &tap_values, total) {
                if let Some(cb) = &on_slice {
                    if let Some(&orig) = visible.get(si) {
                        cb(orig, tap_slices[si].value);
                    }
                }
            }
        })
    };
    let plot = center(component_props(
        render_pie_plot,
        PiePlotProps {
            plot,
            size,
            activate,
            tap,
            clear,
        },
    ))
    .into_widget();

    // The legend lists every named slice (dimmed if hidden); tapping a chip toggles it.
    let legend_el = if legend && slices.iter().any(|s| !s.label.is_empty()) {
        let vis_total: f64 = slices
            .iter()
            .enumerate()
            .filter(|(i, _)| !hidden_set.contains(i))
            .map(|(_, s)| s.value.max(0.0))
            .sum::<f64>()
            .max(f64::MIN_POSITIVE);
        let items: Vec<LegendItem> = slices
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let is_hidden = hidden_set.contains(&i);
                LegendItem {
                    label: s.label.clone(),
                    color: all_colors[i],
                    value: (legend_values && !is_hidden)
                        .then(|| format!("{:.0}%", s.value.max(0.0) / vis_total * 100.0)),
                    hidden: is_hidden,
                }
            })
            .collect();
        let toggle: Rc<dyn Fn(usize)> = Rc::new(move |i: usize| {
            hidden.update(|set| {
                if !set.remove(&i) {
                    set.insert(i);
                }
            });
        });
        let vertical = matches!(legend_position, LegendPosition::Left | LegendPosition::Right);
        Some(legend_widget(items, vertical, Some(toggle)))
    } else {
        None
    };

    let Some(el) = legend_el else {
        return column(children![plot])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min)
            .into_widget();
    };
    match legend_position {
        LegendPosition::Bottom => column(children![plot, gap_h(14.0), el]),
        LegendPosition::Top => column(children![el, gap_h(14.0), plot]),
        LegendPosition::Left => {
            return row(children![el, gap_w(20.0), plot])
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .main_axis_size(MainAxisSize::Min)
                .into_widget();
        }
        LegendPosition::Right => {
            return row(children![plot, gap_w(20.0), el])
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .main_axis_size(MainAxisSize::Min)
                .into_widget();
        }
    }
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_size(MainAxisSize::Min)
    .into_widget()
}

/// Readable label color for text drawn ON TOP of a filled slice: dark on light
/// fills, white on dark ones (relative luminance).
fn contrast_on(fill: Color) -> Color {
    let [r, g, b, _] = fill.components;
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    if lum > 0.6 {
        Color::from_rgba8(0x1A, 0x1A, 0x1A, 0xFF)
    } else {
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)
    }
}

#[allow(clippy::too_many_arguments)]
fn pie_data_labels(
    slices: &[Slice],
    values: &[f64],
    colors: &[Color],
    total: f64,
    size: f64,
    hole: f64,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    let r = size / 2.0 - 6.0;
    let label_r = r * if hole > 0.0 { (1.0 + hole) / 2.0 } else { 0.6 };
    let mut a0 = -std::f64::consts::FRAC_PI_2;
    let mut items = Vec::new();
    for (i, &value) in values.iter().enumerate() {
        if !value.is_finite() || value <= 0.0 {
            continue;
        }
        let sweep = value / total * std::f64::consts::TAU;
        let mid = a0 + sweep / 2.0;
        a0 += sweep; // advance for EVERY slice, even the ones we skip below
        let pct = value / total * 100.0;
        // A label must fit inside its own slice, or it overruns the neighbours (the old
        // bug: wide "name %" labels on thin slices scattered everywhere). `label_r*sweep`
        // is the slice's tangential room at the label radius. Fit the fullest label that
        // room allows; skip slices too thin for even a percentage (the legend names them).
        let arc = label_r * sweep;
        let name = slices
            .get(i)
            .map(|s| s.label.clone())
            .filter(|label| !label.is_empty());
        let (label, w) = match name {
            Some(name) if arc >= 78.0 => (format!("{name} {:.0}%", pct), 88.0),
            _ if arc >= 26.0 => (format!("{:.0}%", pct), 34.0),
            _ => continue,
        };
        let color = colors
            .get(i)
            .copied()
            .map(contrast_on)
            .unwrap_or(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF));
        // Center the fixed-width label box ON the slice centroid so it actually sits
        // where the slice is (the child text is centered within that box).
        let cx = size / 2.0 + label_r * mid.cos();
        let cy = size / 2.0 + label_r * mid.sin();
        items.push(
            positioned(
                sized_box(
                    apply_font(text(label).size((label_px - 0.5).max(6.0)).semibold().color(color), font)
                        .align(TextAlign::Center),
                )
                .width(w)
                .height(14.0),
            )
            .left((cx - w / 2.0).clamp(0.0, size - w))
            .top((cy - 7.0).clamp(0.0, size - 14.0))
            .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(size)
        .height(size)
        .into_widget()
}

// ---------------------------------------------------------------------------
// Shared legend
// ---------------------------------------------------------------------------

/// One legend entry. `value` is an optional trailing figure (e.g. a percentage or a
/// series total); `hidden` dims the chip for a toggled-off series/slice.
struct LegendItem {
    label: String,
    color: Color,
    value: Option<String>,
    hidden: bool,
}

/// The shared legend. `vertical` stacks chips in a left-aligned column (for the Left /
/// Right positions); otherwise a centered wrap row. When `on_toggle` is `Some`, every
/// chip is tappable and calls back with its index — the caller flips visibility.
fn legend_widget(
    items: Vec<LegendItem>,
    vertical: bool,
    on_toggle: Option<Rc<dyn Fn(usize)>>,
) -> AnyWidget {
    let c = theme().colors;
    let chips: Vec<AnyWidget> = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let swatch = if item.hidden {
                with_alpha(item.color, 0.30)
            } else {
                item.color
            };
            let text_color = if item.hidden {
                with_alpha(c.muted_foreground, 0.5)
            } else {
                c.muted_foreground
            };
            let mut chip_children: Vec<AnyWidget> = vec![
                container()
                    .width(11.0)
                    .height(11.0)
                    .decoration(BoxDecoration::new().color(swatch).radius(BorderRadius::all(3.0)))
                    .into_widget(),
                gap_w(7.0).into_widget(),
                text(item.label).size(12.0).color(text_color).into_widget(),
            ];
            if let Some(value) = item.value {
                chip_children.push(gap_w(6.0).into_widget());
                chip_children.push(text(value).size(12.0).semibold().color(text_color).into_widget());
            }
            let chip = row(chip_children)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center);
            match &on_toggle {
                Some(cb) => {
                    let cb = cb.clone();
                    pressable(container().padding(EdgeInsets::symmetric(4.0, 2.0)).child(chip))
                        .radius(6.0)
                        .on_tap(move || cb(i))
                        .into_widget()
                }
                None => chip.into_widget(),
            }
        })
        .collect();
    if vertical {
        let mut rows: Vec<AnyWidget> = Vec::new();
        for (i, chip) in chips.into_iter().enumerate() {
            if i > 0 {
                rows.push(gap_h(8.0).into_widget());
            }
            rows.push(chip);
        }
        column(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .into_widget()
    } else {
        wrap(chips)
            .spacing(18.0)
            .run_spacing(8.0)
            .alignment(WrapAlignment::Center)
            .into_widget()
    }
}

/// A plain non-interactive horizontal legend (used by radial / flow charts).
fn legend_row(items: Vec<(String, Color)>) -> AnyWidget {
    legend_widget(
        items
            .into_iter()
            .map(|(label, color)| LegendItem {
                label,
                color,
                value: None,
                hidden: false,
            })
            .collect(),
        false,
        None,
    )
}
