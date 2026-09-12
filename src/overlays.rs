//! Plot decorations shared across cartesian charts: reference lines, reference bands, and point annotations.

use pebbles::prelude::*;

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
