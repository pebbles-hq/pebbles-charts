//! Radial charts: radar, and the progress-ring / gauge.

use crate::data::*;
use crate::legend::*;
use crate::style::*;
use pebbles::prelude::*;

// ---------------------------------------------------------------------------
// Radar / Gauge / Progress
// ---------------------------------------------------------------------------

/// A radar (spider) chart plotting each series as a polygon over shared category axes. Built with [`radar_chart`].
pub struct RadarChart {
    categories: Vec<String>,
    series: Vec<Series>,
    size: f64,
    legend: bool,
}

/// A **radar chart** over `categories`, one polygon per series.
pub fn radar_chart(categories: Vec<String>, series: Vec<Series>) -> RadarChart {
    RadarChart {
        categories,
        series,
        size: 280.0,
        legend: true,
    }
}

impl RadarChart {
    /// Set the chart's diameter in px.
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
    /// Show or hide the legend (default on).
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

/// A circular progress **ring** or **gauge**. Built with [`progress_ring`] / [`gauge_chart`].
pub struct RadialProgressChart {
    label: String,
    value: f64,
    max: f64,
    size: f64,
    thickness: f64,
    gauge: bool,
    color: Option<Color>,
}

/// Alias — a gauge arc. See [`gauge_chart`].
pub type GaugeChart = RadialProgressChart;

/// A **progress ring** showing `value` out of `max` as a full circular arc.
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

/// A **gauge** showing `value` out of `max` as an open arc.
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
    /// Set the diameter in px.
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }
    /// Set the ring/arc stroke thickness in px.
    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness.max(1.0);
        self
    }
    /// Set the progress-arc color (default: the first palette color).
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

pub(crate) fn arc_path(cx: f64, cy: f64, r: f64, a0: f64, a1: f64, steps: usize) -> BezPath {
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
