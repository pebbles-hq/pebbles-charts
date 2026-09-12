//! The dense heatmap grid chart.

use crate::data::*;
use crate::render::*;
use crate::style::*;
use pebbles::prelude::*;

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
