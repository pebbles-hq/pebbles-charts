//! Shared configuration enums for charts (series kinds, axis scales, curves, label + legend modes).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesKind {
    Bar,
    Line,
    Area,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisScale {
    Linear,
    Log10,
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveInterpolation {
    Linear,
    Step,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryLabelMode {
    Auto,
    All,
    Skip(usize),
    Truncate(usize),
    Hidden,
}

/// Where the legend sits relative to the plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendPosition {
    Bottom,
    Top,
    Left,
    Right,
}
