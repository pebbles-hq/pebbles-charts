//! Scatter and bubble charts (numeric x/y, optional per-point radius).

use crate::config::*;
use crate::data::*;
use crate::legend::*;
use crate::scale::*;
use crate::style::*;
use pebbles::prelude::*;

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
