//! The shared cursor-following tooltip: row model, rendering, and the show/position helpers.

use crate::cartesian::hit::*;
use crate::data::*;
use crate::scale::*;
use crate::style::*;
use pebbles::prelude::*;
use std::rc::Rc;

pub(crate) fn plot_local_from_global(global: Offset, chart_bounds: Rect, plot_x: f64) -> Offset {
    Offset::new(
        global.x - chart_bounds.x0 - plot_x,
        global.y - chart_bounds.y0,
    )
}

pub(crate) fn tooltip_rows(
    series: &[Series],
    colors: &[Color],
    vals: &[Vec<f64>],
    active: ActiveDatum,
    format_value: &Rc<dyn Fn(f64) -> String>,
) -> Vec<(String, Color, String, bool)> {
    vals.iter()
        .enumerate()
        .filter_map(|(si, sv)| {
            let value = *sv.get(active.category)?;
            if !value.is_finite() {
                return None;
            }
            let label = series
                .get(si)
                .map(|s| s.label.as_str())
                .filter(|label| !label.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Series {}", si + 1));
            Some((
                label,
                colors.get(si).copied().unwrap_or_else(|| palette_color(si)),
                format_value(value),
                active.series == Some(si),
            ))
        })
        .collect()
}

pub(crate) fn cartesian_tooltip(
    title: String,
    rows: Vec<(String, Color, String, bool)>,
) -> AnyWidget {
    let c = theme().colors;
    let mut children = Vec::new();
    children.push(
        text(title)
            .size(12.0)
            .semibold()
            .color(c.popover_foreground)
            .into_widget(),
    );
    children.push(gap_h(6.0).into_widget());
    for (label, color, value, active) in rows {
        let value_text = if active {
            text(value)
                .size(12.0)
                .semibold()
                .color(c.popover_foreground)
        } else {
            text(value).size(12.0).color(c.popover_foreground)
        };
        children.push(
            row(children![
                container().width(9.0).height(9.0).decoration(
                    BoxDecoration::new()
                        .color(color)
                        .radius(BorderRadius::all(2.5))
                ),
                gap_w(7.0),
                text(label).size(12.0).color(c.muted_foreground),
                gap_w(14.0),
                spacer(),
                value_text,
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .into_widget(),
        );
    }
    container()
        .width(180.0)
        .decoration(
            BoxDecoration::new()
                .color(c.popover)
                .border(Border::new(c.border, 1.0))
                .radius(BorderRadius::all(7.0))
                .shadow(BoxShadow::new(
                    Color::from_rgba8(0, 0, 0, 38),
                    Offset::new(0.0, 8.0),
                    18.0,
                    -5.0,
                )),
        )
        .padding(EdgeInsets::symmetric(10.0, 8.0))
        .child(column(children).main_axis_size(MainAxisSize::Min))
        .into_widget()
}

pub(crate) fn show_cartesian_tooltip(
    global: Offset,
    categories: &[String],
    series: &[Series],
    colors: &[Color],
    vals: &[Vec<f64>],
    active: ActiveDatum,
    format_value: &Rc<dyn Fn(f64) -> String>,
) {
    let title = categories
        .get(active.category)
        .cloned()
        .unwrap_or_else(|| format!("Category {}", active.category + 1));
    let rows = tooltip_rows(series, colors, vals, active, format_value);
    if rows.is_empty() {
        hide_passive();
    } else {
        show_passive(
            cartesian_tooltip(title, rows),
            global.x + TOOLTIP_OFFSET,
            global.y + TOOLTIP_OFFSET,
        );
    }
}
