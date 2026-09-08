//! The chart widgets. Cartesian charts (bar / line / area) share axis, scaling, legend,
//! and category labels — they differ only in what they draw on the canvas. Pie / donut
//! are their own radial widget. Everything is drawn with the framework `Canvas`
//! (`fill_rrect`, `stroke_path`, `fill_path`, `fill_circle`), so it's GPU-rendered.

use pebbles::prelude::*;

use crate::{Series, Slice, palette_color, with_alpha};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bar,
    Line,
    Area,
}

/// A bar / line / area chart over labelled categories. Built with [`bar_chart`],
/// [`line_chart`], or [`area_chart`].
pub struct CartesianChart {
    kind: Kind,
    categories: Vec<String>,
    series: Vec<Series>,
    width: f64,
    height: f64,
    legend: bool,
    grid: bool,
}

/// Alias — a bar chart. See [`bar_chart`].
pub type BarChart = CartesianChart;
/// Alias — a line chart. See [`line_chart`].
pub type LineChart = CartesianChart;
/// Alias — an area chart. See [`area_chart`].
pub type AreaChart = CartesianChart;

fn cartesian(kind: Kind, categories: Vec<String>, series: Vec<Series>) -> CartesianChart {
    CartesianChart { kind, categories, series, width: 520.0, height: 260.0, legend: true, grid: true }
}

/// A grouped **bar chart**.
pub fn bar_chart(categories: Vec<String>, series: Vec<Series>) -> BarChart {
    cartesian(Kind::Bar, categories, series)
}
/// A **line chart**.
pub fn line_chart(categories: Vec<String>, series: Vec<Series>) -> LineChart {
    cartesian(Kind::Line, categories, series)
}
/// An **area chart** (filled line).
pub fn area_chart(categories: Vec<String>, series: Vec<Series>) -> AreaChart {
    cartesian(Kind::Area, categories, series)
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
    /// Draw horizontal grid lines (default true).
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }
}

impl IntoWidget for CartesianChart {
    fn into_widget(self) -> AnyWidget {
        let CartesianChart { kind, categories, series, width, height, legend, grid } = self;
        let colors: Vec<Color> = series.iter().enumerate().map(|(i, s)| s.color.unwrap_or(palette_color(i))).collect();
        let vals: Vec<Vec<f64>> = series.iter().map(|s| s.values.clone()).collect();
        let draw_colors = colors.clone();
        let ncat = categories.len().max(1);
        let max = vals
            .iter()
            .flat_map(|v| v.iter())
            .cloned()
            .fold(0.0_f64, f64::max)
            .max(1.0)
            * 1.08;
        let grid_c = with_alpha(theme().colors.muted_foreground, 0.16);
        let label_c = theme().colors.muted_foreground;

        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let (pl, pt, pr, pb) = (10.0, 12.0, 10.0, 8.0);
            let pw = (s.width - pl - pr).max(1.0);
            let ph = (s.height - pt - pb).max(1.0);
            let x0 = pl;
            let y1 = pt + ph; // baseline
            let y_of = |v: f64| y1 - (v / max) * ph;
            let cx_of = |i: usize| x0 + pw * (i as f64 + 0.5) / ncat as f64;

            // grid
            if grid {
                for k in 0..=4 {
                    let y = pt + ph * k as f64 / 4.0;
                    c.stroke_line(Offset::new(x0, y), Offset::new(x0 + pw, y), 1.0, grid_c);
                }
            }

            match kind {
                Kind::Bar => {
                    let nser = vals.len().max(1);
                    let slot = pw / ncat as f64;
                    let group_w = slot * 0.7;
                    let bw = group_w / nser as f64;
                    for (si, sv) in vals.iter().enumerate() {
                        for (i, &v) in sv.iter().enumerate() {
                            let gx = cx_of(i) - group_w / 2.0 + si as f64 * bw;
                            let top = y_of(v);
                            let r = Rect::new(gx + 1.0, top, gx + bw - 1.0, y1);
                            c.fill_rrect(r, (bw / 3.0).min(4.0), draw_colors[si]);
                        }
                    }
                }
                Kind::Area | Kind::Line => {
                    for (si, sv) in vals.iter().enumerate() {
                        if sv.is_empty() {
                            continue;
                        }
                        let pts: Vec<(f64, f64)> = sv.iter().enumerate().map(|(i, &v)| (cx_of(i), y_of(v))).collect();
                        if kind == Kind::Area {
                            let mut fill = BezPath::new();
                            fill.move_to((pts[0].0, y1));
                            for &(x, y) in &pts {
                                fill.line_to((x, y));
                            }
                            fill.line_to((pts[pts.len() - 1].0, y1));
                            fill.close_path();
                            c.fill_path(&fill, with_alpha(draw_colors[si], 0.18));
                        }
                        let mut line = BezPath::new();
                        line.move_to(pts[0]);
                        for &p in &pts[1..] {
                            line.line_to(p);
                        }
                        c.stroke_path(&line, 2.0, draw_colors[si]);
                        for &(x, y) in &pts {
                            c.fill_circle(Offset::new(x, y), 2.6, draw_colors[si]);
                        }
                    }
                }
            }
        })
        .width(width)
        .height(height);

        let mut col: Vec<AnyWidget> = vec![plot.into_widget()];
        // x-axis category labels: evenly-spaced cells aligned with the slots.
        col.push(gap_h(6.0).into_widget());
        col.push(
            row(categories
                .iter()
                .map(|cat| {
                    expanded(text(cat.clone()).size(11.0).color(label_c).align(TextAlign::Center)).into_widget()
                })
                .collect::<Vec<_>>())
            .into_widget(),
        );
        if legend && series.iter().any(|s| !s.label.is_empty()) {
            col.push(gap_h(14.0).into_widget());
            col.push(legend_row(series.iter().map(|s| s.label.clone()).zip(colors.iter().cloned()).collect()));
        }

        container()
            .width(width)
            .child(column(col).cross_axis_alignment(CrossAxisAlignment::Stretch).main_axis_size(MainAxisSize::Min))
            .into_widget()
    }
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
}

/// A **pie chart**.
pub fn pie_chart(slices: Vec<Slice>) -> PieChart {
    PieChart { slices, size: 260.0, hole: 0.0, legend: true }
}
/// A **donut chart** (pie with a hole).
pub fn donut_chart(slices: Vec<Slice>) -> PieChart {
    PieChart { slices, size: 260.0, hole: 0.58, legend: true }
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
}

impl IntoWidget for PieChart {
    fn into_widget(self) -> AnyWidget {
        let PieChart { slices, size, hole, legend } = self;
        let colors: Vec<Color> = slices.iter().enumerate().map(|(i, s)| s.color.unwrap_or(palette_color(i))).collect();
        let values: Vec<f64> = slices.iter().map(|s| s.value.max(0.0)).collect();
        let draw_colors = colors.clone();
        let total = values.iter().sum::<f64>().max(f64::MIN_POSITIVE);

        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let cx = s.width / 2.0;
            let cy = s.height / 2.0;
            let r = (s.width.min(s.height) / 2.0) - 6.0;
            let ir = r * hole;
            let mut a0 = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
            for (i, &v) in values.iter().enumerate() {
                let sweep = v / total * std::f64::consts::TAU;
                let a1 = a0 + sweep;
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

        let mut col: Vec<AnyWidget> = vec![center(plot).into_widget()];
        if legend {
            col.push(gap_h(14.0).into_widget());
            col.push(legend_row(slices.iter().map(|s| s.label.clone()).zip(colors.iter().cloned()).collect()));
        }
        column(col).cross_axis_alignment(CrossAxisAlignment::Center).main_axis_size(MainAxisSize::Min).into_widget()
    }
}

// ---------------------------------------------------------------------------
// Shared legend
// ---------------------------------------------------------------------------

fn legend_row(items: Vec<(String, Color)>) -> AnyWidget {
    let c = theme().colors;
    let chips: Vec<AnyWidget> = items
        .into_iter()
        .map(|(label, color)| {
            row(children![
                container()
                    .width(11.0)
                    .height(11.0)
                    .decoration(BoxDecoration::new().color(color).radius(BorderRadius::all(3.0))),
                gap_w(7.0),
                text(label).size(12.0).color(c.muted_foreground),
            ])
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .into_widget()
        })
        .collect();
    wrap(chips).spacing(18.0).run_spacing(8.0).alignment(WrapAlignment::Center).into_widget()
}
