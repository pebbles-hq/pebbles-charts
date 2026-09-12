//! Cartesian label overlays: axis titles, category labels, y-axis ticks, data labels, and the annotation layer.

use super::*;
use crate::cartesian::Kind;
use crate::render::*;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub(crate) fn axis_title_row(
    left: Option<String>,
    right: Option<String>,
    left_axis_width: f64,
    plot_width: f64,
    right_axis_width: f64,
    color: Color,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    row(children![
        sized_box(
            left.map(|title| {
                apply_font(text(title).size(label_px).semibold().color(color), font)
                    .align(TextAlign::Right)
                    .into_widget()
            })
            .unwrap_or_else(|| gap_w(0.0).into_widget())
        )
        .width(left_axis_width),
        gap_w(0.0),
        sized_box(gap_w(0.0)).width(plot_width),
        sized_box(
            right
                .map(|title| {
                    apply_font(text(title).size(label_px).semibold().color(color), font)
                        .align(TextAlign::Left)
                        .into_widget()
                })
                .unwrap_or_else(|| gap_w(0.0).into_widget())
        )
        .width(right_axis_width),
    ])
    .into_widget()
}

pub(crate) fn category_label_widgets(
    categories: &[String],
    width: f64,
    mode: CategoryLabelMode,
    color: Color,
    label_px: f32,
    font: &Option<String>,
) -> Vec<AnyWidget> {
    let n = categories.len().max(1);
    let slot = width / n as f64;
    let every = match mode {
        CategoryLabelMode::Hidden => 1,
        CategoryLabelMode::Skip(n) => n.max(1),
        CategoryLabelMode::Auto if slot < 34.0 => 3,
        CategoryLabelMode::Auto if slot < 52.0 => 2,
        _ => 1,
    };
    let max_chars = match mode {
        CategoryLabelMode::Truncate(n) => n.max(1),
        CategoryLabelMode::Auto => (slot / 6.5).floor().clamp(3.0, 14.0) as usize,
        _ => usize::MAX,
    };
    categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let label = if mode == CategoryLabelMode::Hidden {
                String::new()
            } else if i % every == 0 {
                truncate_label(cat, max_chars)
            } else {
                String::new()
            };
            expanded(
                apply_font(text(label).size(label_px).color(color), font).align(TextAlign::Center),
            )
            .into_widget()
        })
        .collect()
}

pub(crate) fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let keep = max_chars.saturating_sub(2).max(1);
    format!("{}..", label.chars().take(keep).collect::<String>())
}

pub(crate) fn reference_labels(
    lines: &[ReferenceLine],
    bands: &[ReferenceBand],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    color: Color,
    pad: EdgeInsets,
) -> AnyWidget {
    let bottom = (height - pad.bottom).max(pad.top + 1.0);
    let scale = axis.scale(pad.top, bottom);
    let mut items = Vec::new();
    for line in lines {
        let Some(label) = &line.label else {
            continue;
        };
        let y = scale.map(line.value).clamp(pad.top, bottom);
        items.push(
            positioned(
                sized_box(
                    text(label.clone())
                        .size(10.0)
                        .semibold()
                        .color(color)
                        .align(TextAlign::Right),
                )
                .width(58.0),
            )
            .right(6.0)
            .top((y - 15.0).clamp(0.0, (height - 16.0).max(0.0)))
            .into_widget(),
        );
    }
    for band in bands {
        let Some(label) = &band.label else {
            continue;
        };
        let y0 = scale.map(band.start).clamp(pad.top, bottom);
        let y1 = scale.map(band.end).clamp(pad.top, bottom);
        let y = (y0 + y1) / 2.0;
        items.push(
            positioned(
                sized_box(
                    text(label.clone())
                        .size(10.0)
                        .semibold()
                        .color(color)
                        .align(TextAlign::Right),
                )
                .width(58.0),
            )
            .right(6.0)
            .top((y - 7.0).clamp(0.0, (height - 16.0).max(0.0)))
            .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(width)
        .height(height)
        .into_widget()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cartesian_data_labels(
    kind: Kind,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    format_value: &Rc<dyn Fn(f64) -> String>,
    color: Color,
    label_px: f32,
    font: &Option<String>,
    pad: EdgeInsets,
) -> AnyWidget {
    let bottom = (height - pad.bottom).max(pad.top + 1.0);
    let scale = axis.scale(pad.top, bottom);
    let pw = (width - pad.left - pad.right).max(1.0);
    let ncat = vals.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let nser = vals.len().max(1);
    let label_w = 42.0;
    let label_h = 14.0;
    let mut items = Vec::new();
    for (si, sv) in vals.iter().enumerate() {
        for (i, &v) in sv.iter().enumerate() {
            if !v.is_finite() {
                continue;
            }
            let (x, y, align) = match kind {
                Kind::HorizontalBar => {
                    let x_scale = LinearScale::new(axis.min, axis.max, pad.left, pad.left + pw);
                    let slot = (bottom - pad.top).max(1.0) / ncat as f64;
                    let group_h = slot * 0.7;
                    let bh = group_h / nser as f64;
                    let x = x_scale.map(v).clamp(pad.left, pad.left + pw);
                    (
                        if v >= 0.0 { x + 4.0 } else { x - label_w - 4.0 },
                        pad.top + slot * (i as f64 + 0.5) - group_h / 2.0
                            + si as f64 * bh
                            + bh / 2.0
                            - label_h / 2.0,
                        if v >= 0.0 {
                            TextAlign::Left
                        } else {
                            TextAlign::Right
                        },
                    )
                }
                _ => {
                    let slot = pw / ncat as f64;
                    let x = pad.left + pw * (i as f64 + 0.5) / ncat as f64;
                    let x = if kind == Kind::Bar {
                        let group_w = slot * 0.7;
                        let bw = group_w / nser as f64;
                        x - group_w / 2.0 + si as f64 * bw + bw / 2.0
                    } else {
                        x
                    };
                    let y = scale.map(v).clamp(pad.top, bottom);
                    (x - label_w / 2.0, y - label_h - 5.0, TextAlign::Center)
                }
            };
            items.push(
                positioned(
                    sized_box(
                        apply_font(
                            text(format_value(v))
                                .size((label_px - 1.0).max(6.0))
                                .semibold()
                                .color(color),
                            font,
                        )
                        .align(align),
                    )
                    .width(label_w)
                    .height(label_h),
                )
                .left(x.clamp(0.0, (width - label_w).max(0.0)))
                .top(y.clamp(0.0, (height - label_h).max(0.0)))
                .into_widget(),
            );
        }
    }
    sized_box(stack(items))
        .width(width)
        .height(height)
        .into_widget()
}

/// Point callouts (dot + label) pinned to data points — the annotation overlay.
#[allow(clippy::too_many_arguments)]
pub(crate) fn annotation_layer(
    annotations: &[Annotation],
    axis: &ValueAxis,
    width: f64,
    height: f64,
    ncat: usize,
    ref_color: Color,
    label_color: Color,
    label_px: f32,
    font: &Option<String>,
    pad: EdgeInsets,
) -> AnyWidget {
    let bottom = (height - pad.bottom).max(pad.top + 1.0);
    let scale = axis.scale(pad.top, bottom);
    let pw = (width - pad.left - pad.right).max(1.0);
    let ncatf = ncat.max(1) as f64;
    let dot = 8.0;
    let lw = 120.0;
    let mut items = Vec::new();
    for a in annotations {
        if !a.value.is_finite() {
            continue;
        }
        let color = a.color.unwrap_or(ref_color);
        let cx = pad.left + pw * (a.category as f64 + 0.5) / ncatf;
        let cy = scale.map(a.value).clamp(pad.top, bottom);
        items.push(
            positioned(
                container().width(dot).height(dot).decoration(
                    BoxDecoration::new()
                        .color(color)
                        .radius(BorderRadius::all(dot / 2.0)),
                ),
            )
            .left(cx - dot / 2.0)
            .top(cy - dot / 2.0)
            .into_widget(),
        );
        items.push(
            positioned(
                sized_box(
                    apply_font(
                        text(a.label.clone())
                            .size(label_px)
                            .semibold()
                            .color(label_color),
                        font,
                    )
                    .align(TextAlign::Center),
                )
                .width(lw)
                .height(16.0),
            )
            .left((cx - lw / 2.0).clamp(0.0, (width - lw).max(0.0)))
            .top((cy - dot / 2.0 - 18.0).clamp(0.0, (height - 16.0).max(0.0)))
            .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(width)
        .height(height)
        .into_widget()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn y_axis_labels(
    axis: &ValueAxis,
    labels: Vec<String>,
    height: f64,
    color: Color,
    align: TextAlign,
    label_px: f32,
    font: &Option<String>,
    pad: EdgeInsets,
) -> AnyWidget {
    let bottom = (height - pad.bottom).max(pad.top + 1.0);
    let scale = axis.scale(pad.top, bottom);
    let mut items = Vec::new();
    for (&tick, label) in axis.ticks.iter().zip(labels) {
        let y = scale.map(tick);
        items.push(
            positioned(apply_font(text(label).size(label_px).color(color), font).align(align))
                .left(0.0)
                .right(0.0)
                .top((y - 7.0).clamp(0.0, (height - 14.0).max(0.0)))
                .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(Y_AXIS_WIDTH)
        .height(height)
        .into_widget()
}
