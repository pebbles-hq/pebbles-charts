//! The cartesian plot painter — `render_cartesian_chart` (grid, axes, marks, interaction wiring, animation).

use super::*;
use crate::cartesian::Kind;
use crate::legend::*;
use crate::render::*;
use crate::scale::*;
use crate::style::*;
use crate::tooltip::*;
use pebbles::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;

pub(crate) fn render_cartesian_chart(chart: &CartesianChart) -> AnyWidget {
    let kind = chart.kind;
    let mut categories = chart.categories.clone();
    let mut series = chart.series.clone();
    let width = chart.width;
    // Aspect-ratio lock: derive the height from the width so the chart keeps its shape.
    let height = chart
        .aspect_ratio
        .map(|r| width / r)
        .unwrap_or(chart.height);
    // Internal plot inset — one value shared by the draw, hit-testing, and every overlay so
    // they stay aligned. Defaults to the tuned 10/12/10/8.
    let pad = chart.plot_padding.unwrap_or(EdgeInsets {
        left: PLOT_LEFT,
        top: PLOT_TOP,
        right: PLOT_RIGHT,
        bottom: PLOT_BOTTOM,
    });
    // Loading / error take priority over the plot and over the empty state.
    if let Some(msg) = &chart.error {
        return empty_placeholder(width, height, msg);
    }
    if chart.loading {
        return empty_placeholder(width, height, "Loading…");
    }
    let legend = chart.legend;
    let grid = chart.grid;
    let y_axis = chart.y_axis;
    let right_y_axis = chart.right_y_axis.clone();
    let right_y_title = chart.right_y_title.clone();
    let y_range = chart.y_range;
    let tick_count = chart.tick_count;
    let value_formatter = chart.value_formatter.clone();
    let right_value_formatter = chart.right_value_formatter.clone();
    let tooltip = chart.tooltip;
    let on_point = chart.on_point.clone();
    let all_combo_kinds = chart.combo_kinds.clone();
    let legend_position = chart.legend_position;
    let legend_values = chart.legend_values;
    let animate = chart.animate.unwrap_or(!prefers_reduced_motion());
    let animation_ms = chart.animation_ms;
    let area_gradient = chart.area_gradient;
    let curve = chart.curve;
    let vertical_grid = chart.vertical_grid;
    let x_axis_title = chart.x_axis_title.clone();
    let y_axis_title = chart.y_axis_title.clone();
    let category_label_mode = chart.category_label_mode;
    let reference_lines = chart.reference_lines.clone();
    let reference_bands = chart.reference_bands.clone();
    let annotations = chart.annotations.clone();
    let data_labels = chart.data_labels;
    let palette_override = chart.palette.clone();
    let pal_color = move |i: usize| -> Color {
        match &palette_override {
            Some(p) if !p.is_empty() => p[i % p.len()],
            _ => palette_color(i),
        }
    };
    apply_category_window(&mut categories, &mut series, chart.category_window);

    // Empty state: no categories/series, or every value is missing/non-finite. Render a
    // calm "no data" panel at the chart's footprint rather than a blank or broken plot.
    let has_data = !categories.is_empty()
        && !series.is_empty()
        && series
            .iter()
            .any(|s| s.values.iter().any(|v| v.is_finite()));
    if !has_data {
        return empty_placeholder(width, height, "No data");
    }

    let active = create_signal(None::<ActiveDatum>);
    let active_datum = active.get();
    // Entry animation: a 0→1 progress kicked once on mount (the same one-shot idiom the
    // framework's `animated` hook uses). The draw wipes the marks in left-to-right by `t`.
    let anim = create_signal(0.0_f64);
    let anim_kicked = create_signal(false);
    if animate && !anim_kicked.peek() {
        anim_kicked.set(true);
        pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
    }
    let anim_t = if animate { anim.get() } else { 1.0 };
    // Data-change tween: when the values change on a later render (same shape), morph the
    // marks from the previous values to the new ones instead of snapping. Snapshots are
    // kept by ORIGINAL series index (the full set) so a legend toggle — which only changes
    // the *visible* subset — never reads as a data change. Same one-shot idiom as the entry
    // kick: signal writes during render, guarded by `peek()`.
    let full_target: Vec<Vec<f64>> = series.iter().map(|s| s.values.clone()).collect();
    let data_t = create_signal(1.0_f64);
    let last_full = create_signal(None::<Vec<Vec<f64>>>);
    let from_full = create_signal(Vec::<Vec<f64>>::new());
    // Track series labels across renders to tell an ADD from a REMOVE on a shape change:
    // an add re-wipes (enter), a pure removal is handled by the exit-fade block below.
    let last_labels = create_signal(Vec::<String>::new());
    let cur_labels: Vec<String> = series.iter().map(|s| s.label.clone()).collect();
    let series_added = {
        let prev = last_labels.peek();
        !prev.is_empty() && cur_labels.iter().any(|l| !prev.contains(l))
    };
    last_labels.set(cur_labels);
    {
        let prev = last_full.peek();
        let same_shape = prev.as_ref().is_some_and(|p| {
            p.len() == full_target.len()
                && p.iter().zip(&full_target).all(|(a, b)| a.len() == b.len())
        });
        let changed = prev.as_ref().is_some_and(|p| p != &full_target);
        if changed && same_shape && animate {
            // Same shape, new values → morph each datum from its old value to the new one.
            from_full.set(prev.clone().unwrap());
            data_t.set(0.0);
            pebbles::core::animation::animate_to(data_t, 1.0, animation_ms as f64 / 1000.0);
        } else if prev.is_none() {
            // First mount → no morph (the entry wipe handles the reveal).
            from_full.set(full_target.clone());
        } else if changed {
            // Shape changed → snap the values. A series ADD re-runs the entry wipe (enter
            // animation); a pure REMOVE skips the wipe and fades the gone series out below.
            from_full.set(full_target.clone());
            data_t.set(1.0);
            if animate && series_added {
                anim.set(0.0);
                pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
            }
        }
        last_full.set(Some(full_target.clone()));
    }
    let data_t_val = if animate { data_t.get() } else { 1.0 };
    let morph_from = from_full.peek();
    // Interpolate a target datum from its held "from" snapshot by the tween progress.
    // Gaps (NaN) and a settled tween snap straight to the target.
    let morph = move |orig_series: usize, cat: usize, target: f64| -> f64 {
        let fv = morph_from
            .get(orig_series)
            .and_then(|r| r.get(cat))
            .copied()
            .unwrap_or(target);
        if data_t_val >= 1.0 || fv.is_nan() || target.is_nan() {
            target
        } else {
            fv + (target - fv) * data_t_val
        }
    };
    // Keyboard traversal: the chart is focusable (Tab / click), and Left/Right arrows
    // move the active category, highlighting it + drawing the crosshair. Uses the
    // framework's register_keys (non-editor key routing).
    let focus = create_focus();
    let ncat_for_keys = categories.len().max(1);
    focus.register(std::rc::Rc::new(|| {}), None, false);
    focus.register_keys(std::rc::Rc::new(move |key: KeyInput| {
        let step = match key {
            KeyInput::Move {
                motion: Motion::Left,
                ..
            } => -1i64,
            KeyInput::Move {
                motion: Motion::Right,
                ..
            } => 1,
            _ => return false,
        };
        let next = match active.peek() {
            None => 0,
            Some(a) => (a.category as i64 + step).rem_euclid(ncat_for_keys as i64) as usize,
        };
        active.set(Some(ActiveDatum {
            category: next,
            series: None,
        }));
        true
    }));
    // Series toggled off from the legend. Kept by ORIGINAL series index so the legend
    // (which shows every series) and the palette color stay stable while the plot,
    // domain, tooltip, and hit-testing operate on only the visible subset.
    let hidden = create_signal(HashSet::<usize>::new());
    let hidden_set = hidden.get();
    let chart_bounds = use_bounds();

    // Colors by original index — the legend paints every series in its own color.
    let all_colors: Vec<Color> = series
        .iter()
        .enumerate()
        .map(|(i, s)| s.color.unwrap_or_else(|| pal_color(i)))
        .collect();

    // Per-series exit fade: when a series is removed (matched by label, same category
    // count), keep drawing its REAL last values as a fading ghost line for one animation
    // cycle, so a removed series recedes instead of vanishing. Enter is the shape-change
    // re-wipe above. Snapshots the last-rendered (label, values, color) to recover a gone
    // series' data. Values/hit-testing/scale are unaffected — the ghost is draw-only.
    let ncat_now = categories.len();
    let exiting = create_signal(Vec::<(Vec<f64>, Color)>::new());
    let exit_t = create_signal(1.0_f64);
    let prev_rendered = create_signal(Vec::<(String, Vec<f64>, Color)>::new());
    {
        let cur: Vec<(String, Vec<f64>, Color)> = series
            .iter()
            .enumerate()
            .map(|(i, s)| (s.label.clone(), s.values.clone(), all_colors[i]))
            .collect();
        let prev = prev_rendered.peek();
        let removed: Vec<(Vec<f64>, Color)> = prev
            .iter()
            .filter(|(l, v, _)| v.len() == ncat_now && !cur.iter().any(|(cl, _, _)| cl == l))
            .map(|(_, v, c)| (v.clone(), *c))
            .collect();
        if !removed.is_empty() && animate {
            exiting.set(removed);
            exit_t.set(0.0);
            pebbles::core::animation::animate_to(exit_t, 1.0, animation_ms as f64 / 1000.0);
        }
        prev_rendered.set(cur);
    }
    let exit_t_val = if animate { exit_t.get() } else { 1.0 };
    let exiting_series = if exit_t_val < 1.0 {
        exiting.peek()
    } else {
        Vec::new()
    };

    let visible: Vec<usize> = (0..series.len())
        .filter(|i| !hidden_set.contains(i))
        .collect();
    // Visible-only views: everything downstream (scale, draw, hit-test, tooltip) reads
    // these, so hiding a series rescales the axis and reflows grouped bars automatically.
    let colors: Vec<Color> = visible.iter().map(|&i| all_colors[i]).collect();
    let vals: Vec<Vec<f64>> = visible.iter().map(|&i| series[i].values.clone()).collect();
    let vis_series: Vec<Series> = visible.iter().map(|&i| series[i].clone()).collect();
    let combo_kinds: Vec<SeriesKind> = if all_combo_kinds.is_empty() {
        Vec::new()
    } else {
        visible
            .iter()
            .map(|&i| all_combo_kinds.get(i).copied().unwrap_or(SeriesKind::Line))
            .collect()
    };
    // Morphed views for the axis + marks: each visible datum interpolated from its held
    // snapshot (by original series index) toward the target. `vals` stays the target set,
    // so hit-testing, tooltips, and data labels always report the real (final) numbers.
    let anim_vals: Vec<Vec<f64>> = visible
        .iter()
        .map(|&oi| {
            series[oi]
                .values
                .iter()
                .enumerate()
                .map(|(ci, &tv)| morph(oi, ci, tv))
                .collect()
        })
        .collect();
    let draw_vals = anim_vals.clone();
    let draw_colors = colors.clone();
    // Per-series ± error whiskers (visible order), drawn on each mark. None = no whiskers.
    let draw_errors: Vec<Option<Vec<f64>>> =
        visible.iter().map(|&i| series[i].errors.clone()).collect();
    let ncat = categories.len().max(1);
    let mut axis_values = axis_values_for_kind(kind, &anim_vals);
    // Error whiskers reach v ± e, so the domain must include those bounds (non-stacked
    // kinds; stacked/percent charts don't draw error bars). Add v+e and v-e as extra rows.
    let stacked_kind = matches!(
        kind,
        Kind::StackedBar | Kind::PercentStackedBar | Kind::StackedArea | Kind::PercentStackedArea
    );
    if !stacked_kind {
        for (si, errs) in draw_errors.iter().enumerate() {
            let Some(errs) = errs else { continue };
            let vals_si = draw_vals.get(si).map(Vec::as_slice).unwrap_or(&[]);
            let hi: Vec<f64> = vals_si
                .iter()
                .enumerate()
                .map(|(ci, &v)| {
                    v + errs
                        .get(ci)
                        .copied()
                        .filter(|e| e.is_finite())
                        .unwrap_or(0.0)
                })
                .collect();
            let lo: Vec<f64> = vals_si
                .iter()
                .enumerate()
                .map(|(ci, &v)| {
                    v - errs
                        .get(ci)
                        .copied()
                        .filter(|e| e.is_finite())
                        .unwrap_or(0.0)
                })
                .collect();
            axis_values.push(hi);
            axis_values.push(lo);
        }
    }
    let axis = ValueAxis::from_values(&axis_values, y_range, tick_count);
    let ticks = axis.ticks.clone();
    let format_value: Rc<dyn Fn(f64) -> String> =
        value_formatter.unwrap_or_else(|| Rc::new(compact_number));
    let tick_labels: Vec<String> = ticks.iter().map(|&tick| format_value(tick)).collect();
    let right_format_value: Rc<dyn Fn(f64) -> String> =
        right_value_formatter.unwrap_or_else(|| format_value.clone());
    let right_tick_labels: Vec<String> = right_y_axis
        .as_ref()
        .map(|axis| {
            axis.ticks
                .iter()
                .map(|&tick| right_format_value(tick))
                .collect()
        })
        .unwrap_or_default();
    // Per-slot style (grid/zero/reference/label colors, stroke/point/area/bar geometry,
    // label typography) — theme defaults unless overridden via `.style(ChartStyle)`.
    let style = chart.style.clone();
    let grid_c = style.grid();
    let zero_c = style.zero_line();
    let reference_c = style.reference();
    let active_band_c = with_alpha(theme().colors.muted_foreground, 0.07);
    let active_line_c = with_alpha(theme().colors.muted_foreground, 0.38);
    let label_c = style.label();
    let line_w = style.line_w();
    let point_r = style.point_r();
    let area_a = style.area_a();
    // Stacked areas overlap, so they default denser (0.32) than a lone area (0.18); an
    // explicit `.area_alpha` in the style overrides both.
    let stacked_area_a = style.area_alpha.unwrap_or(0.32);
    let bar_r_max = style.bar_r();
    let label_px = style.label_px();
    let font = style.font_family.clone();
    let left_axis_width = if y_axis {
        Y_AXIS_WIDTH + Y_AXIS_GAP
    } else {
        0.0
    };
    let right_axis_width = if right_y_axis.is_some() {
        Y_AXIS_GAP + Y_AXIS_WIDTH
    } else {
        0.0
    };
    let plot_width = (width - left_axis_width - right_axis_width).max(80.0);
    let plot_x = if y_axis {
        Y_AXIS_WIDTH + Y_AXIS_GAP
    } else {
        0.0
    };
    let axis_for_draw = axis.clone();
    let combo_kinds_for_draw = combo_kinds.clone();
    let reference_lines_for_draw = reference_lines.clone();
    let reference_bands_for_draw = reference_bands.clone();

    let plot = canvas(move |c: &mut Canvas<'_>| {
        let s = c.size();
        let (pl, pt, pr, pb) = (pad.left, pad.top, pad.right, pad.bottom);
        let pw = (s.width - pl - pr).max(1.0);
        let ph = (s.height - pt - pb).max(1.0);
        let x0 = pl;
        let y1 = pt + ph;
        let y_scale = axis_for_draw.scale(pt, y1);
        let baseline = y_scale.map(0.0).clamp(pt, y1);
        let cx_of = |i: usize| x0 + pw * (i as f64 + 0.5) / ncat as f64;

        for band in &reference_bands_for_draw {
            if !band.start.is_finite() || !band.end.is_finite() {
                continue;
            }
            let y0 = y_scale.map(band.start).clamp(pt, y1);
            let yb = y_scale.map(band.end).clamp(pt, y1);
            let color = band.color.unwrap_or(reference_c);
            c.fill_rect(
                Rect::new(x0, y0.min(yb), x0 + pw, y0.max(yb)),
                with_alpha(color, 0.14),
            );
        }

        if let Some(active) = active_datum {
            let slot = pw / ncat as f64;
            let band_x0 = (cx_of(active.category) - slot / 2.0).max(x0);
            let band_x1 = (band_x0 + slot).min(x0 + pw);
            c.fill_rect(Rect::new(band_x0, pt, band_x1, y1), active_band_c);
        }

        if grid {
            for &tick in &axis_for_draw.ticks {
                let y = y_scale.map(tick);
                c.stroke_line(Offset::new(x0, y), Offset::new(x0 + pw, y), 1.0, grid_c);
            }
        }
        if vertical_grid {
            for i in 0..ncat {
                let x = cx_of(i);
                c.stroke_line(Offset::new(x, pt), Offset::new(x, y1), 1.0, grid_c);
            }
        }
        c.stroke_line(
            Offset::new(x0, baseline),
            Offset::new(x0 + pw, baseline),
            1.25,
            zero_c,
        );
        for line in &reference_lines_for_draw {
            if !line.value.is_finite() {
                continue;
            }
            let y = y_scale.map(line.value).clamp(pt, y1);
            // Reference/target lines are drawn dashed — the conventional way to distinguish
            // an annotation from a data mark (matches Chart.js / D3 target lines).
            c.stroke_line_dashed(
                Offset::new(x0, y),
                Offset::new(x0 + pw, y),
                1.4,
                line.color.unwrap_or(reference_c),
                &[6.0, 4.0],
                0.0,
            );
        }

        if let Some(active) = active_datum {
            let x = cx_of(active.category);
            c.stroke_line(Offset::new(x, pt), Offset::new(x, y1), 1.0, active_line_c);
        }

        // Entry animation: reveal the marks left-to-right as `anim_t` goes 0→1. Only the
        // marks are clipped — grid, axes, baseline and reference lines stay static.
        let wiping = anim_t < 1.0;
        if wiping {
            c.push_clip(Rect::new(x0, pt, x0 + pw * anim_t, y1));
        }
        match kind {
            Kind::Bar => {
                let nser = draw_vals.len().max(1);
                let slot = pw / ncat as f64;
                let group_w = slot * 0.7;
                let bw = group_w / nser as f64;
                for (si, sv) in draw_vals.iter().enumerate() {
                    for (i, &v) in sv.iter().enumerate() {
                        if !v.is_finite() {
                            continue;
                        }
                        let gx = cx_of(i) - group_w / 2.0 + si as f64 * bw;
                        let y = y_scale.map(v);
                        let r =
                            Rect::new(gx + 1.0, y.min(baseline), gx + bw - 1.0, y.max(baseline));
                        c.fill_rrect(r, (bw / 3.0).min(bar_r_max), draw_colors[si]);
                        if active_datum.is_some_and(|a| a.category == i && a.series == Some(si)) {
                            c.fill_rrect(
                                r,
                                (bw / 3.0).min(bar_r_max),
                                with_alpha(draw_colors[si], 0.32),
                            );
                        }
                    }
                }
            }
            Kind::StackedBar | Kind::PercentStackedBar => {
                let slot = pw / ncat as f64;
                let bw = slot * 0.64;
                for i in 0..ncat {
                    let mut positive = 0.0;
                    let mut negative = 0.0;
                    for (si, _) in draw_vals.iter().enumerate() {
                        let Some(v) = stacked_value(kind, &draw_vals, i, si) else {
                            continue;
                        };
                        if !v.is_finite() || v == 0.0 {
                            continue;
                        }
                        let base = if v >= 0.0 { positive } else { negative };
                        let next = base + v;
                        if v >= 0.0 {
                            positive = next;
                        } else {
                            negative = next;
                        }
                        let x = cx_of(i) - bw / 2.0;
                        let y0 = y_scale.map(base);
                        let y1s = y_scale.map(next);
                        let r = Rect::new(x, y0.min(y1s), x + bw, y0.max(y1s));
                        c.fill_rrect(r, (bw / 3.0).min(bar_r_max), draw_colors[si]);
                        if active_datum.is_some_and(|a| a.category == i && a.series == Some(si)) {
                            c.fill_rrect(
                                r,
                                (bw / 3.0).min(bar_r_max),
                                with_alpha(draw_colors[si], 0.32),
                            );
                        }
                    }
                }
            }
            Kind::HorizontalBar => {
                let x_scale = LinearScale::new(axis_for_draw.min, axis_for_draw.max, x0, x0 + pw);
                let h_baseline = x_scale.map(0.0).clamp(x0, x0 + pw);
                let nser = draw_vals.len().max(1);
                let slot = ph / ncat as f64;
                let group_h = slot * 0.7;
                let bh = group_h / nser as f64;
                for (si, sv) in draw_vals.iter().enumerate() {
                    for (i, &v) in sv.iter().enumerate() {
                        if !v.is_finite() {
                            continue;
                        }
                        let cy = pt + slot * (i as f64 + 0.5);
                        let gy = cy - group_h / 2.0 + si as f64 * bh;
                        let x = x_scale.map(v);
                        let r = Rect::new(
                            x.min(h_baseline),
                            gy + 1.0,
                            x.max(h_baseline),
                            gy + bh - 1.0,
                        );
                        c.fill_rrect(r, (bh / 3.0).min(bar_r_max), draw_colors[si]);
                    }
                }
            }
            Kind::Combo => {
                let bar_count = combo_kinds_for_draw
                    .iter()
                    .filter(|&&k| k == SeriesKind::Bar)
                    .count()
                    .max(1);
                let slot = pw / ncat as f64;
                let group_w = slot * 0.48;
                let bw = group_w / bar_count as f64;
                let mut bar_seen = 0;
                for (si, sv) in draw_vals.iter().enumerate() {
                    match combo_kinds_for_draw
                        .get(si)
                        .copied()
                        .unwrap_or(SeriesKind::Line)
                    {
                        SeriesKind::Bar => {
                            let offset = bar_seen;
                            bar_seen += 1;
                            for (i, &v) in sv.iter().enumerate() {
                                if !v.is_finite() {
                                    continue;
                                }
                                let gx = cx_of(i) - group_w / 2.0 + offset as f64 * bw;
                                let y = y_scale.map(v);
                                let r = Rect::new(
                                    gx + 1.0,
                                    y.min(baseline),
                                    gx + bw - 1.0,
                                    y.max(baseline),
                                );
                                c.fill_rrect(
                                    r,
                                    (bw / 3.0).min(bar_r_max),
                                    with_alpha(draw_colors[si], 0.76),
                                );
                            }
                        }
                        SeriesKind::Area | SeriesKind::Line => {
                            draw_line_area_series(
                                c,
                                sv,
                                si,
                                SeriesKind::Area
                                    == combo_kinds_for_draw
                                        .get(si)
                                        .copied()
                                        .unwrap_or(SeriesKind::Line),
                                area_gradient,
                                curve,
                                baseline,
                                &y_scale,
                                &cx_of,
                                draw_colors[si],
                                active_datum,
                                line_w,
                                point_r,
                                area_a,
                            );
                        }
                    }
                }
            }
            Kind::StackedArea | Kind::PercentStackedArea => {
                let mut positive = vec![0.0; ncat];
                let mut negative = vec![0.0; ncat];
                for (si, _) in draw_vals.iter().enumerate() {
                    let mut top = Vec::new();
                    let mut bottom = Vec::new();
                    for i in 0..ncat {
                        let Some(v) = stacked_value(kind, &draw_vals, i, si) else {
                            continue;
                        };
                        if !v.is_finite() {
                            continue;
                        }
                        let base = if v >= 0.0 { positive[i] } else { negative[i] };
                        let next = base + v;
                        if v >= 0.0 {
                            positive[i] = next;
                        } else {
                            negative[i] = next;
                        }
                        bottom.push((cx_of(i), y_scale.map(base)));
                        top.push((cx_of(i), y_scale.map(next)));
                    }
                    if top.is_empty() {
                        continue;
                    }
                    let mut fill = BezPath::new();
                    fill.move_to(top[0]);
                    for &p in &top[1..] {
                        fill.line_to(p);
                    }
                    for &p in bottom.iter().rev() {
                        fill.line_to(p);
                    }
                    fill.close_path();
                    if area_gradient {
                        let grad = Gradient::vertical([
                            with_alpha(draw_colors[si], 0.5),
                            with_alpha(draw_colors[si], 0.05),
                        ]);
                        c.fill_path_gradient(&fill, &grad);
                    } else {
                        c.fill_path(&fill, with_alpha(draw_colors[si], stacked_area_a));
                    }
                    let mut line = BezPath::new();
                    line.move_to(top[0]);
                    for &p in &top[1..] {
                        line.line_to(p);
                    }
                    c.stroke_path(&line, line_w, draw_colors[si]);
                }
            }
            Kind::Area | Kind::Line | Kind::SteppedLine | Kind::Sparkline => {
                for (si, sv) in draw_vals.iter().enumerate() {
                    draw_line_area_series(
                        c,
                        sv,
                        si,
                        kind == Kind::Area,
                        area_gradient,
                        if kind == Kind::SteppedLine {
                            CurveInterpolation::Step
                        } else {
                            curve
                        },
                        baseline,
                        &y_scale,
                        &cx_of,
                        draw_colors[si],
                        active_datum,
                        line_w,
                        point_r,
                        area_a,
                    );
                }
            }
        }
        if wiping {
            c.pop_clip();
        }
        // Error bars: a ± whisker (with caps) on each mark for series that carry `errors`.
        // Positioned on the real mark — the grouped-bar center for bars, the point x
        // otherwise — using the current scale so it tracks the value tween.
        if draw_errors.iter().any(Option::is_some) {
            let slot = pw / ncat as f64;
            let group_w = slot * 0.7;
            let bw = group_w / (draw_vals.len().max(1)) as f64;
            for (si, errs) in draw_errors.iter().enumerate() {
                let Some(errs) = errs else { continue };
                let col = with_alpha(draw_colors.get(si).copied().unwrap_or(zero_c), 0.85);
                for (ci, &v) in draw_vals
                    .get(si)
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
                    .iter()
                    .enumerate()
                {
                    let e = errs.get(ci).copied().unwrap_or(f64::NAN);
                    if !v.is_finite() || !e.is_finite() || e <= 0.0 {
                        continue;
                    }
                    let x = if kind == Kind::Bar {
                        cx_of(ci) - group_w / 2.0 + si as f64 * bw + bw / 2.0
                    } else {
                        cx_of(ci)
                    };
                    let y_hi = y_scale.map(v + e);
                    let y_lo = y_scale.map(v - e);
                    let cap = 4.0;
                    c.stroke_line(Offset::new(x, y_hi), Offset::new(x, y_lo), 1.4, col);
                    c.stroke_line(
                        Offset::new(x - cap, y_hi),
                        Offset::new(x + cap, y_hi),
                        1.4,
                        col,
                    );
                    c.stroke_line(
                        Offset::new(x - cap, y_lo),
                        Offset::new(x + cap, y_lo),
                        1.4,
                        col,
                    );
                }
            }
        }
        // Per-series exit fade: draw each removed series' real last values as a fading ghost
        // line (outside the entry-wipe clip), receding over the current scale.
        if !exiting_series.is_empty() {
            let a = ((1.0 - exit_t_val) as f32).clamp(0.0, 1.0);
            for (evals, ecolor) in &exiting_series {
                draw_line_area_series(
                    c,
                    evals,
                    usize::MAX,
                    false,
                    false,
                    curve,
                    baseline,
                    &y_scale,
                    &cx_of,
                    with_alpha(*ecolor, a * 0.9),
                    None,
                    line_w,
                    point_r * a as f64,
                    area_a,
                );
            }
        }
    })
    .width(plot_width)
    .height(height);

    let mut plot_layers = vec![plot.into_widget()];
    // Hold data labels until the entry wipe finishes, so they don't float over
    // not-yet-revealed marks.
    if data_labels && anim_t >= 0.999 && data_t_val >= 0.999 {
        plot_layers.push(cartesian_data_labels(
            kind,
            &vals,
            &axis,
            plot_width,
            height,
            &format_value,
            label_c,
            label_px,
            &font,
            pad,
        ));
    }
    plot_layers.push(reference_labels(
        &reference_lines,
        &reference_bands,
        &axis,
        plot_width,
        height,
        label_c,
        pad,
    ));
    if !annotations.is_empty() && anim_t >= 0.999 && data_t_val >= 0.999 {
        plot_layers.push(annotation_layer(
            &annotations,
            &axis,
            plot_width,
            height,
            ncat,
            reference_c,
            label_c,
            label_px,
            &font,
            pad,
        ));
    }
    let plot_surface = sized_box(stack(plot_layers))
        .width(plot_width)
        .height(height)
        .into_widget();

    let activate = {
        let axis = axis.clone();
        let vals = vals.clone();
        let categories = categories.clone();
        // Visible series only — `hit`/`colors`/`vals` all index the visible subset.
        let series = vis_series.clone();
        let colors = colors.clone();
        let format_value = format_value.clone();
        Rc::new(move |local: Offset, global: Offset| {
            if let Some(hit) =
                hit_cartesian_datum(kind, local, plot_width, height, ncat, &vals, &axis, pad)
            {
                active.set(Some(hit));
                if tooltip {
                    show_cartesian_tooltip(
                        global,
                        &categories,
                        &series,
                        &colors,
                        &vals,
                        hit,
                        &format_value,
                    );
                }
            } else {
                active.set(None);
                hide_passive();
            }
        })
    };

    let tap_point = {
        let vals = vals.clone();
        let on_point = on_point.clone();
        Rc::new(move |hit: ActiveDatum| {
            let Some(callback) = &on_point else {
                return;
            };
            let Some(series_idx) = hit.series else {
                return;
            };
            let Some(value) = vals
                .get(series_idx)
                .and_then(|sv| sv.get(hit.category))
                .copied()
            else {
                return;
            };
            if value.is_finite() {
                callback(hit.category, series_idx, value);
            }
        })
    };

    let plot = GestureDetector::new(plot_surface)
        .cursor(Cursor::Pointer)
        .on_hover_enter(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                let local = plot_local_from_global(e.global, chart_bounds, plot_x);
                activate(local, e.global);
            }
        }))
        // Continuous hover: the tooltip + crosshair FOLLOW the cursor across the plot
        // (not just on entry), via the framework's new on_hover_move callback.
        .on_hover_move(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                let local = plot_local_from_global(e.global, chart_bounds, plot_x);
                activate(local, e.global);
            }
        }))
        .on_hover_exit(move || {
            active.set(None);
            hide_passive();
        })
        .on_pointer_down(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| {
                focus.request_focus(); // clicking focuses the chart so arrows traverse it
                activate(e.position, e.global);
            }
        }))
        .on_pan_start(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_update(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_end(hide_passive)
        .on_tap(action_event({
            let axis = axis.clone();
            let vals = vals.clone();
            let tap_point = tap_point.clone();
            move |e: PointerEvent| {
                if let Some(hit) = hit_cartesian_datum(
                    kind, e.position, plot_width, height, ncat, &vals, &axis, pad,
                ) {
                    tap_point(hit);
                }
            }
        }));

    let chart_body = if y_axis || right_y_axis.is_some() {
        let mut axis_row = Vec::new();
        if y_axis {
            axis_row.push(y_axis_labels(
                &axis,
                tick_labels,
                height,
                label_c,
                TextAlign::Right,
                label_px,
                &font,
                pad,
            ));
            axis_row.push(gap_w(Y_AXIS_GAP).into_widget());
        }
        axis_row.push(plot.into_widget());
        if let Some(axis) = &right_y_axis {
            axis_row.push(gap_w(Y_AXIS_GAP).into_widget());
            axis_row.push(y_axis_labels(
                axis,
                right_tick_labels,
                height,
                label_c,
                TextAlign::Left,
                label_px,
                &font,
                pad,
            ));
        }
        row(axis_row).into_widget()
    } else {
        plot.into_widget()
    };

    let mut col: Vec<AnyWidget> = Vec::new();
    if y_axis_title.is_some() || right_y_title.is_some() {
        col.push(axis_title_row(
            y_axis_title.clone(),
            right_y_title.clone(),
            left_axis_width,
            plot_width,
            right_axis_width,
            label_c,
            label_px,
            &font,
        ));
        col.push(gap_h(4.0).into_widget());
    }
    col.push(chart_body);
    if kind != Kind::Sparkline {
        col.push(gap_h(6.0).into_widget());
        // The canvas insets its plot by PLOT_LEFT/PLOT_RIGHT, so category `i` is centered
        // at `PLOT_LEFT + (plot_width - PLOT_LEFT - PLOT_RIGHT)*(i+0.5)/ncat`. The label
        // cells must divide that SAME inset span — otherwise they fan out from the bars
        // (first label left of its bar, last label right of its bar). Add the plot padding
        // to the axis gutters and give the cells the inset width.
        let label_span = (plot_width - pad.left - pad.right).max(1.0);
        col.push(
            row(children![
                gap_w(left_axis_width + pad.left).into_widget(),
                expanded(row(category_label_widgets(
                    &categories,
                    label_span,
                    category_label_mode,
                    label_c,
                    label_px,
                    &font,
                ))),
                gap_w(right_axis_width + pad.right).into_widget(),
            ])
            .into_widget(),
        );
        if let Some(title) = x_axis_title {
            col.push(gap_h(4.0).into_widget());
            col.push(
                row(children![
                    gap_w(left_axis_width).into_widget(),
                    expanded(
                        apply_font(text(title).size(label_px + 0.5).color(label_c), &font)
                            .align(TextAlign::Center)
                    ),
                    gap_w(right_axis_width).into_widget(),
                ])
                .into_widget(),
            );
        }
    }
    // Build the legend once. It lists EVERY series (in its own color, dimmed if toggled
    // off); tapping a chip flips that series' visibility via the `hidden` signal.
    let legend_el = if legend && series.iter().any(|s| !s.label.is_empty()) {
        let items: Vec<LegendItem> = series
            .iter()
            .enumerate()
            .map(|(i, s)| LegendItem {
                label: s.label.clone(),
                color: all_colors[i],
                value: legend_values.then(|| {
                    let total: f64 = s.values.iter().filter(|v| v.is_finite()).sum();
                    format_value(total)
                }),
                hidden: hidden_set.contains(&i),
            })
            .collect();
        let toggle: Rc<dyn Fn(usize)> = Rc::new(move |i: usize| {
            hidden.update(|set| {
                if !set.remove(&i) {
                    set.insert(i);
                }
            });
        });
        let vertical = matches!(
            legend_position,
            LegendPosition::Left | LegendPosition::Right
        );
        Some(legend_widget(items, vertical, Some(toggle)))
    } else {
        None
    };

    // Left/Right sit the legend beside the plot column; Top/Bottom stack it above/below.
    if let Some(el) = legend_el {
        match legend_position {
            LegendPosition::Bottom => {
                col.push(gap_h(14.0).into_widget());
                col.push(el);
            }
            LegendPosition::Top => {
                let mut stacked = vec![el, gap_h(14.0).into_widget()];
                stacked.append(&mut col);
                col = stacked;
            }
            LegendPosition::Left | LegendPosition::Right => {
                let chart = container()
                    .width(width)
                    .child(
                        column(col)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .main_axis_size(MainAxisSize::Min),
                    )
                    .into_widget();
                let lane = if legend_position == LegendPosition::Left {
                    row(children![el, gap_w(20.0), chart])
                } else {
                    row(children![chart, gap_w(20.0), el])
                };
                return lane
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .main_axis_size(MainAxisSize::Min)
                    .into_widget();
            }
        }
    }

    container()
        .width(width)
        .child(
            column(col)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .main_axis_size(MainAxisSize::Min),
        )
        .into_widget()
}
