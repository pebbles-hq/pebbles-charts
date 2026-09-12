//! The shared, interactive legend widget (click a chip to toggle a series/slice).

use crate::style::*;
use pebbles::prelude::*;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Shared legend
// ---------------------------------------------------------------------------

/// One legend entry. `value` is an optional trailing figure (e.g. a percentage or a
/// series total); `hidden` dims the chip for a toggled-off series/slice.
pub(crate) struct LegendItem {
    pub(crate) label: String,
    pub(crate) color: Color,
    pub(crate) value: Option<String>,
    pub(crate) hidden: bool,
}

/// The shared legend. `vertical` stacks chips in a left-aligned column (for the Left /
/// Right positions); otherwise a centered wrap row. When `on_toggle` is `Some`, every
/// chip is tappable and calls back with its index — the caller flips visibility.
pub(crate) fn legend_widget(
    items: Vec<LegendItem>,
    vertical: bool,
    on_toggle: Option<Rc<dyn Fn(usize)>>,
) -> AnyWidget {
    let c = theme().colors;
    let chips: Vec<AnyWidget> = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let swatch = if item.hidden {
                with_alpha(item.color, 0.30)
            } else {
                item.color
            };
            let text_color = if item.hidden {
                with_alpha(c.muted_foreground, 0.5)
            } else {
                c.muted_foreground
            };
            let mut chip_children: Vec<AnyWidget> = vec![
                container()
                    .width(11.0)
                    .height(11.0)
                    .decoration(
                        BoxDecoration::new()
                            .color(swatch)
                            .radius(BorderRadius::all(3.0)),
                    )
                    .into_widget(),
                gap_w(7.0).into_widget(),
                text(item.label).size(12.0).color(text_color).into_widget(),
            ];
            if let Some(value) = item.value {
                chip_children.push(gap_w(6.0).into_widget());
                chip_children.push(
                    text(value)
                        .size(12.0)
                        .semibold()
                        .color(text_color)
                        .into_widget(),
                );
            }
            let chip = row(chip_children)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center);
            match &on_toggle {
                Some(cb) => {
                    let cb = cb.clone();
                    pressable(
                        container()
                            .padding(EdgeInsets::symmetric(4.0, 2.0))
                            .child(chip),
                    )
                    .radius(6.0)
                    .on_tap(move || cb(i))
                    .into_widget()
                }
                None => chip.into_widget(),
            }
        })
        .collect();
    if vertical {
        let mut rows: Vec<AnyWidget> = Vec::new();
        for (i, chip) in chips.into_iter().enumerate() {
            if i > 0 {
                rows.push(gap_h(8.0).into_widget());
            }
            rows.push(chip);
        }
        column(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .into_widget()
    } else {
        wrap(chips)
            .spacing(18.0)
            .run_spacing(8.0)
            .alignment(WrapAlignment::Center)
            .into_widget()
    }
}

/// A plain non-interactive horizontal legend (used by radial / flow charts).
pub(crate) fn legend_row(items: Vec<(String, Color)>) -> AnyWidget {
    legend_widget(
        items
            .into_iter()
            .map(|(label, color)| LegendItem {
                label,
                color,
                value: None,
                hidden: false,
            })
            .collect(),
        false,
        None,
    )
}
