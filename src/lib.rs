//! # pebbles-charts
//!
//! Composable, themeable **chart widgets** for the [Pebbles](https://github.com/pebbles-hq/pebbles)
//! GUI framework — **bar, line, area, pie, and donut** — drawn on the GPU canvas with a
//! config-driven palette and an auto legend. The chrome (grid, axis labels) follows the
//! app's `theme()`, so charts match light/dark for free; series colors come from a
//! [`palette`] you can override per series.
//!
//! ```ignore
//! use pebbles::prelude::*;
//! use pebbles_charts::{bar_chart, series};
//!
//! fn view() -> impl IntoWidget {
//!     bar_chart(
//!         vec!["Jan".into(), "Feb".into(), "Mar".into()],
//!         vec![
//!             series("Desktop", vec![186.0, 305.0, 237.0]),
//!             series("Mobile", vec![80.0, 200.0, 120.0]),
//!         ],
//!     )
//!     .width(520.0)
//!     .height(260.0)
//! }
//! ```

mod charts;

pub use charts::{AreaChart, BarChart, LineChart, PieChart, area_chart, bar_chart, donut_chart, line_chart, pie_chart};

use pebbles::prelude::*;

/// A named data series (one line / one bar color) with a value per category.
#[derive(Clone)]
pub struct Series {
    pub label: String,
    pub values: Vec<f64>,
    pub color: Option<Color>,
}

/// Create a [`Series`]. Give it a color with [`Series::color`], or let the palette pick.
pub fn series(label: impl Into<String>, values: Vec<f64>) -> Series {
    Series { label: label.into(), values, color: None }
}

impl Series {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
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
    Slice { label: label.into(), value, color: None }
}

impl Slice {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// The default categorical palette (distinct, legible on light and dark). Override any
/// series/slice with `.color(..)`. Indexed cyclically.
pub fn palette() -> [Color; 6] {
    let c = |r, g, b| Color::from_rgba8(r, g, b, 0xFF);
    [
        c(0x63, 0x66, 0xF1), // indigo
        c(0x2D, 0xD4, 0xBF), // teal
        c(0xF5, 0x9E, 0x0B), // amber
        c(0xF4, 0x3F, 0x5E), // rose
        c(0x8B, 0x5C, 0xF6), // violet
        c(0x22, 0xC5, 0x5E), // green
    ]
}

/// The color for series/slice `i` (palette, cycled).
pub fn palette_color(i: usize) -> Color {
    let p = palette();
    p[i % p.len()]
}

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    let [r, g, b, _] = c.components;
    Color::new([r, g, b, a])
}
