//! Shared configuration enums for charts (series kinds, axis scales, curves, label + legend
//! modes).

/// How a series is drawn in a [`combo_chart`](crate::combo_chart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesKind {
    /// Vertical bars.
    Bar,
    /// A line through the points.
    Line,
    /// A line with the area beneath it filled.
    Area,
}

/// The scale applied to a scatter/bubble chart's numeric **x** axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    /// Evenly spaced (the default).
    Linear,
    /// Base-10 logarithmic (positive values only).
    Log10,
    /// Treat x values as timestamps for tick placement.
    Time,
}

/// How a line/area path connects its points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveInterpolation {
    /// Straight segments between points (the default).
    Linear,
    /// Horizontal-then-vertical steps.
    Step,
    /// A smooth (monotone) curve through the points.
    Smooth,
}

/// How category (x-axis) labels are thinned when they don't all fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryLabelMode {
    /// Show as many as fit, skipping evenly when crowded (the default).
    Auto,
    /// Always show every label (may overlap on dense axes).
    All,
    /// Show every `n`-th label.
    Skip(usize),
    /// Truncate each label to `n` characters.
    Truncate(usize),
    /// Hide all category labels.
    Hidden,
}

/// Where the legend sits relative to the plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendPosition {
    /// Below the plot (the default).
    Bottom,
    /// Above the plot.
    Top,
    /// To the left of the plot.
    Left,
    /// To the right of the plot.
    Right,
}
