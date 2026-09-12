//! Flow charts: funnel stages and simple sankey links.

use crate::data::*;
use crate::legend::*;
use crate::style::*;
use pebbles::prelude::*;

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
