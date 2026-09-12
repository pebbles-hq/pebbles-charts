//! Cartesian value + stacking math and the line/area path builders.

use super::*;
use crate::cartesian::Kind;
use crate::style::*;
use pebbles::prelude::*;

pub(crate) fn stacked_domain_values(vals: &[Vec<f64>], percent: bool) -> Vec<Vec<f64>> {
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

pub(crate) fn axis_values_for_kind(kind: Kind, vals: &[Vec<f64>]) -> Vec<Vec<f64>> {
    match kind {
        Kind::StackedBar | Kind::StackedArea => stacked_domain_values(vals, false),
        Kind::PercentStackedBar | Kind::PercentStackedArea => stacked_domain_values(vals, true),
        _ => vals.to_vec(),
    }
}

pub(crate) fn percent_value(vals: &[Vec<f64>], category: usize, series: usize) -> Option<f64> {
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

pub(crate) fn stacked_value(
    kind: Kind,
    vals: &[Vec<f64>],
    category: usize,
    series: usize,
) -> Option<f64> {
    match kind {
        Kind::PercentStackedBar | Kind::PercentStackedArea => percent_value(vals, category, series),
        _ => vals.get(series).and_then(|sv| sv.get(category)).copied(),
    }
}

pub(crate) fn finite_line_segments(
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

pub(crate) fn append_points_path(
    path: &mut BezPath,
    pts: &[(usize, f64, f64)],
    curve: CurveInterpolation,
) {
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
pub(crate) fn draw_line_area_series(
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
                let grad =
                    Gradient::vertical([with_alpha(color, area_a * 1.9), with_alpha(color, 0.0)]);
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
            c.fill_circle(
                Offset::new(x, y),
                if active { point_r * 1.73 } else { point_r },
                color,
            );
        }
    }
}
