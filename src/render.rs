//! Small drawing helpers shared by every chart family (empty-state panel, label font,
//! on-fill contrast color, category strip).

use pebbles::prelude::*;

/// A centered message panel sized to the chart's footprint — used for the empty ("No
/// data"), loading, and error states so a live dashboard shows something calm instead of a
/// blank or broken plot.
pub(crate) fn empty_placeholder(width: f64, height: f64, msg: &str) -> AnyWidget {
    let c = theme().colors;
    container()
        .width(width)
        .height(height)
        .child(center(
            text(msg.to_string()).size(13.0).color(c.muted_foreground),
        ))
        .into_widget()
}

/// Apply the chart's configured font family to a label, if one is set.
pub(crate) fn apply_font(t: Text, font: &Option<String>) -> Text {
    match font {
        Some(f) => t.font_family(f.clone()),
        None => t,
    }
}

/// Lay a canvas plot above a centered, evenly-spaced category label strip — the shared
/// x-axis footer for the specialized charts (candlestick, heatmap, …).
pub(crate) fn chart_with_categories(
    plot: CanvasWidget,
    categories: Vec<String>,
    color: Color,
) -> AnyWidget {
    column(children![
        plot,
        gap_h(6.0),
        row(categories
            .iter()
            .map(|cat| expanded(
                text(cat.clone())
                    .size(11.0)
                    .color(color)
                    .align(TextAlign::Center)
            )
            .into_widget())
            .collect::<Vec<_>>()),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .main_axis_size(MainAxisSize::Min)
    .into_widget()
}

/// Readable label color for text drawn ON TOP of a filled slice/bar: dark on light fills,
/// white on dark ones (by relative luminance).
pub(crate) fn contrast_on(fill: Color) -> Color {
    let [r, g, b, _] = fill.components;
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    if lum > 0.6 {
        Color::from_rgba8(0x1A, 0x1A, 0x1A, 0xFF)
    } else {
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)
    }
}
