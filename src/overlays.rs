//! Plot decorations shared across cartesian charts: reference lines, reference bands, and
//! point annotations.

use pebbles::prelude::*;

/// A horizontal marker at a fixed value (a target/threshold), drawn dashed across the plot.
/// Built with [`reference_line`] and added via `.reference_line(..)` on a cartesian chart.
#[derive(Clone)]
pub struct ReferenceLine {
    /// The y value the line sits at.
    pub value: f64,
    /// Optional text label drawn beside the line.
    pub label: Option<String>,
    /// Explicit color; `None` uses the theme's reference color.
    pub color: Option<Color>,
}

/// Create a [`ReferenceLine`] at `value`.
pub fn reference_line(value: f64) -> ReferenceLine {
    ReferenceLine {
        value,
        label: None,
        color: None,
    }
}

impl ReferenceLine {
    /// Set a label drawn beside the line.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set an explicit color instead of the theme default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A shaded horizontal band spanning `start..end` on the value axis (a "normal range"
/// backdrop). Built with [`reference_band`] and added via `.reference_band(..)`.
#[derive(Clone)]
pub struct ReferenceBand {
    /// Lower edge of the band (value axis).
    pub start: f64,
    /// Upper edge of the band (value axis).
    pub end: f64,
    /// Optional text label drawn beside the band.
    pub label: Option<String>,
    /// Explicit color; `None` uses the theme's reference color (translucent).
    pub color: Option<Color>,
}

/// Create a [`ReferenceBand`] spanning `start..end`.
pub fn reference_band(start: f64, end: f64) -> ReferenceBand {
    ReferenceBand {
        start,
        end,
        label: None,
        color: None,
    }
}

impl ReferenceBand {
    /// Set a label drawn beside the band.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set an explicit color instead of the theme default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A callout pinned to one data point (category index + value): a dot plus a text label,
/// for marking an event or an outlier ("launch", "peak", …). Built with [`annotation`].
#[derive(Clone)]
pub struct Annotation {
    /// Category index the callout points at.
    pub category: usize,
    /// Value (y position) the callout points at.
    pub value: f64,
    /// The callout text.
    pub label: String,
    /// Explicit dot + label color; `None` uses the theme's reference color.
    pub color: Option<Color>,
}

/// A point callout at category `category`, value `value`, showing `label`.
pub fn annotation(category: usize, value: f64, label: impl Into<String>) -> Annotation {
    Annotation {
        category,
        value,
        label: label.into(),
        color: None,
    }
}

impl Annotation {
    /// Override the dot + label color (default: the reference color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
