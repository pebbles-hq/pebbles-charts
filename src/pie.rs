//! Pie and donut charts, with an interactive legend, slice labels, and hover pop-out.

use crate::config::*;
use crate::data::*;
use crate::legend::*;
use crate::render::*;
use crate::scale::*;
use crate::style::*;
use crate::tooltip::*;
use pebbles::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Pie / Donut
// ---------------------------------------------------------------------------

/// A **pie** or **donut** chart. Built with [`pie_chart`] / [`donut_chart`].
pub struct PieChart {
    slices: Vec<Slice>,
    size: f64,
    hole: f64, // 0.0 = pie; 0.0..1.0 = donut inner-radius fraction
    legend: bool,
    data_labels: bool,
    legend_position: LegendPosition,
    legend_values: bool,
    tooltip: bool,
    on_slice: Option<Rc<dyn Fn(usize, f64)>>,
    // None = auto (honor the OS reduced-motion preference); Some(_) = explicit override.
    animate: Option<bool>,
    animation_ms: u32,
    palette: Option<Vec<Color>>,
    a11y_label: Option<String>,
    style: ChartStyle,
    loading: bool,
    error: Option<String>,
}

/// A **pie chart**.
pub fn pie_chart(slices: Vec<Slice>) -> PieChart {
    PieChart {
        slices,
        size: 260.0,
        hole: 0.0,
        legend: true,
        data_labels: false,
        legend_position: LegendPosition::Bottom,
        legend_values: false,
        tooltip: true,
        on_slice: None,
        animate: None,
        animation_ms: 700,
        palette: None,
        a11y_label: None,
        style: ChartStyle::new(),
        loading: false,
        error: None,
    }
}
/// A **donut chart** (pie with a hole).
pub fn donut_chart(slices: Vec<Slice>) -> PieChart {
    PieChart {
        slices,
        size: 260.0,
        hole: 0.58,
        legend: true,
        data_labels: false,
        legend_position: LegendPosition::Bottom,
        legend_values: false,
        tooltip: true,
        on_slice: None,
        animate: None,
        animation_ms: 700,
        palette: None,
        a11y_label: None,
        style: ChartStyle::new(),
        loading: false,
        error: None,
    }
}

impl PieChart {
    pub fn size(mut self, px: f64) -> Self {
        self.size = px;
        self
    }
    /// The hole radius as a fraction of the outer radius (0 = solid pie).
    pub fn hole(mut self, frac: f64) -> Self {
        self.hole = frac.clamp(0.0, 0.95);
        self
    }
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
    pub fn data_labels(mut self, on: bool) -> Self {
        self.data_labels = on;
        self
    }
    /// Place the legend `Bottom` (default), `Top`, `Left`, or `Right` of the chart.
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self
    }
    /// Show each slice's percentage next to its name in the legend.
    pub fn legend_values(mut self, on: bool) -> Self {
        self.legend_values = on;
        self
    }
    /// Show a value tooltip and pop the active slice out when hovered/tapped (default true).
    pub fn tooltip(mut self, on: bool) -> Self {
        self.tooltip = on;
        self
    }
    /// Run `callback(slice_index, value)` when a slice is tapped.
    pub fn on_slice(mut self, callback: impl Fn(usize, f64) + 'static) -> Self {
        self.on_slice = Some(Rc::new(callback));
        self
    }
    /// Override the categorical palette for this chart (cycled per slice). Pass
    /// [`cvd_palette`](crate::cvd_palette)`().to_vec()` for a colorblind-safe ramp, or any
    /// custom `Vec<Color>`. Per-slice `.color(..)` still wins over the palette.
    pub fn palette(mut self, colors: Vec<Color>) -> Self {
        self.palette = if colors.is_empty() {
            None
        } else {
            Some(colors)
        };
        self
    }
    /// Animate the wedges in on mount (a radial sweep) and tween on data change. By default
    /// this **auto-honors the OS reduced-motion preference**; call `.animate(true)`/`(false)`
    /// to force it regardless.
    pub fn animate(mut self, on: bool) -> Self {
        self.animate = Some(on);
        self
    }
    /// Entry-animation duration in milliseconds (default 700).
    pub fn animation_ms(mut self, ms: u32) -> Self {
        self.animation_ms = ms;
        self
    }
    /// Set the accessible summary a screen reader announces (e.g. "Traffic by browser").
    /// If unset, a summary is generated from the chart type and slice count. The per-slice
    /// values + percentages are always read out as the node's value.
    pub fn a11y_label(mut self, label: impl Into<String>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }
    /// Override the per-slot visual style (label color/size/font, grid + track colors) — the
    /// theme-as-config surface. See [`ChartStyle`](crate::ChartStyle).
    pub fn style(mut self, style: ChartStyle) -> Self {
        self.style = style;
        self
    }
    /// Show a loading placeholder instead of the ring — for data that hasn't arrived yet.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// Show an error placeholder with `message` instead of the ring — for a failed load.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }
}

/// Build the accessibility (role/label/value) for a pie/donut: a spoken summary plus a
/// per-slice value + percentage read-out — the chart's data-table fallback for a screen
/// reader.
pub(crate) fn pie_a11y(chart: &PieChart) -> (String, String) {
    let kind = if chart.hole > 0.0 { "Donut" } else { "Pie" };
    let total = chart
        .slices
        .iter()
        .map(|s| s.value.max(0.0))
        .filter(|v| v.is_finite())
        .sum::<f64>()
        .max(f64::MIN_POSITIVE);
    let label = chart
        .a11y_label
        .clone()
        .unwrap_or_else(|| format!("{kind} chart, {} slices", chart.slices.len()));
    let parts: Vec<String> = chart
        .slices
        .iter()
        .filter(|s| s.value.is_finite() && s.value > 0.0)
        .map(|s| {
            format!(
                "{} {} ({:.0}%)",
                s.label,
                compact_number(s.value),
                s.value / total * 100.0
            )
        })
        .collect();
    let value = if parts.is_empty() {
        "No data".to_string()
    } else {
        parts.join(", ")
    };
    (label, value)
}

impl IntoWidget for PieChart {
    fn into_widget(self) -> AnyWidget {
        // Emit an accessibility node (role + summary + per-slice read-out) around the
        // canvas so the chart is legible to a screen reader. See `pie_a11y`.
        let (label, value) = pie_a11y(&self);
        // A component so the legend's toggle `create_signal` gets its own scope.
        let chart = component_props(render_pie_chart, self);
        semantics(SemanticsRole::Image, label, chart)
            .value(value)
            .into_widget()
    }
}

/// Props for the interactive pie plot sub-component. It exists so `use_bounds()` reports
/// the PLOT's own rect — letting a hover's global position map to slice-local coords.
pub(crate) struct PiePlotProps {
    plot: AnyWidget,
    size: f64,
    activate: Rc<dyn Fn(Offset, Offset)>, // (local, global) — hover + press + drag
    tap: Rc<dyn Fn(Offset)>,              // (local) — click
    clear: Rc<dyn Fn()>,                  // hover-exit / drag-end
}

pub(crate) fn render_pie_plot(p: &PiePlotProps) -> AnyWidget {
    let bounds = use_bounds(); // this component IS the sized plot → its screen rect
    let size = p.size;
    let (activate, tap, clear) = (p.activate.clone(), p.tap.clone(), p.clear.clone());
    // Hover position arrives in window space; subtract the plot's origin for local coords.
    let hover = {
        let activate = activate.clone();
        move |e: PointerEvent| {
            let local = Offset::new(e.global.x - bounds.x0, e.global.y - bounds.y0);
            activate(local, e.global);
        }
    };
    GestureDetector::new(sized_box(p.plot.clone()).width(size).height(size))
        .cursor(Cursor::Pointer)
        .on_hover_enter(action_event(hover.clone()))
        .on_hover_move(action_event(hover))
        .on_hover_exit({
            let clear = clear.clone();
            move || clear()
        })
        .on_pointer_down(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_start(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_update(action_event({
            let activate = activate.clone();
            move |e: PointerEvent| activate(e.position, e.global)
        }))
        .on_pan_end({
            let clear = clear.clone();
            move || clear()
        })
        .on_tap(action_event(move |e: PointerEvent| tap(e.position)))
        .into_widget()
}

/// Which slice (by VISIBLE index) the local pointer falls in, or None if it's outside
/// the ring. Walks the same start angle / sweep order the draw uses, so the hit matches
/// exactly what's painted.
pub(crate) fn hit_pie_slice(
    local: Offset,
    size: f64,
    hole: f64,
    values: &[f64],
    total: f64,
) -> Option<usize> {
    let center = size / 2.0;
    let r = size / 2.0 - 6.0;
    let ir = r * hole;
    let (dx, dy) = (local.x - center, local.y - center);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > r || dist < ir {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let rel = (dy.atan2(dx) - (-std::f64::consts::FRAC_PI_2)).rem_euclid(tau);
    let mut acc = 0.0;
    for (i, &v) in values.iter().enumerate() {
        let sweep = v.max(0.0) / total * tau;
        if rel >= acc && rel < acc + sweep {
            return Some(i);
        }
        acc += sweep;
    }
    (!values.is_empty()).then(|| values.len() - 1)
}

/// A passive value tooltip for a hovered/tapped pie or donut slice.
pub(crate) fn show_pie_tooltip(
    global: Offset,
    label: &str,
    color: Color,
    value_str: String,
    pct: f64,
) {
    let title = if label.is_empty() {
        "Slice".to_string()
    } else {
        label.to_string()
    };
    show_passive(
        cartesian_tooltip(
            title,
            vec![(
                String::new(),
                color,
                format!("{value_str}  ·  {pct:.0}%"),
                true,
            )],
        ),
        global.x + TOOLTIP_OFFSET,
        global.y + TOOLTIP_OFFSET,
    );
}

pub(crate) fn render_pie_chart(chart: &PieChart) -> AnyWidget {
    let slices = chart.slices.clone();
    let size = chart.size;
    // Loading / error take priority over the ring and the empty state.
    if let Some(msg) = &chart.error {
        return empty_placeholder(size, size, msg);
    }
    if chart.loading {
        return empty_placeholder(size, size, "Loading…");
    }
    let hole = chart.hole;
    let legend = chart.legend;
    let data_labels = chart.data_labels;
    let legend_position = chart.legend_position;
    let legend_values = chart.legend_values;
    let tooltip = chart.tooltip;
    let on_slice = chart.on_slice.clone();
    let animate = chart.animate.unwrap_or(!prefers_reduced_motion());
    let animation_ms = chart.animation_ms;
    let palette_override = chart.palette.clone();
    let pal_color = move |i: usize| -> Color {
        match &palette_override {
            Some(p) if !p.is_empty() => p[i % p.len()],
            _ => palette_color(i),
        }
    };

    // Empty state: no slices, or every slice is zero/negative (nothing to draw). Render a
    // calm "no data" panel at the chart's footprint rather than an empty ring.
    let has_data = slices.iter().any(|s| s.value.is_finite() && s.value > 0.0);
    if !has_data {
        return empty_placeholder(size, size, "No data");
    }

    // Entry animation: a 0→1 progress kicked once on mount; the wedges sweep in radially
    // (each slice's swept angle scales by `anim_t`).
    let anim = create_signal(0.0_f64);
    let anim_kicked = create_signal(false);
    if animate && !anim_kicked.peek() {
        anim_kicked.set(true);
        pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
    }
    let anim_t = if animate { anim.get() } else { 1.0 };

    // Slices toggled off from the legend (by original index). Hiding a slice drops it
    // from the pie and the remaining slices re-proportion to fill (total = visible sum),
    // so the drawn angles and the legend percentages always agree.
    let hidden = create_signal(HashSet::<usize>::new());
    let hidden_set = hidden.get();
    // The slice under the pointer (by visible index): pops out + drives the tooltip.
    let active_slice = create_signal(None::<usize>);
    let active_slice_val = active_slice.get();

    let all_colors: Vec<Color> = slices
        .iter()
        .enumerate()
        .map(|(i, s)| s.color.unwrap_or_else(|| pal_color(i)))
        .collect();
    let visible: Vec<usize> = (0..slices.len())
        .filter(|&i| !hidden_set.contains(&i) && slices[i].value.max(0.0) > 0.0)
        .collect();
    let vis_slices: Vec<Slice> = visible.iter().map(|&i| slices[i].clone()).collect();
    let vis_colors: Vec<Color> = visible.iter().map(|&i| all_colors[i]).collect();

    // Data-change tween: when the slice values change on a later render (same slice count),
    // morph each wedge from its previous value to the new one so the ring re-proportions
    // smoothly. Snapshots are kept by ORIGINAL slice index so a legend toggle (which only
    // changes the *visible* set) never reads as a data change. Same one-shot idiom as the
    // entry sweep. Labels + hit-testing always read the target values.
    let full_target: Vec<f64> = slices.iter().map(|s| s.value.max(0.0)).collect();
    let data_t = create_signal(1.0_f64);
    let last_full = create_signal(None::<Vec<f64>>);
    let from_full = create_signal(Vec::<f64>::new());
    match last_full.peek().as_ref() {
        None => from_full.set(full_target.clone()),
        Some(prev) => {
            let same_shape = prev.len() == full_target.len();
            let changed = prev != &full_target;
            if changed && same_shape && animate {
                from_full.set(prev.clone());
                data_t.set(0.0);
                pebbles::core::animation::animate_to(data_t, 1.0, animation_ms as f64 / 1000.0);
            } else if changed {
                from_full.set(full_target.clone());
                data_t.set(1.0);
                if animate {
                    anim.set(0.0);
                    pebbles::core::animation::animate_to(anim, 1.0, animation_ms as f64 / 1000.0);
                }
            }
        }
    }
    last_full.set(Some(full_target.clone()));
    let data_t_val = if animate { data_t.get() } else { 1.0 };
    let morph_from = from_full.peek();

    let target_values: Vec<f64> = visible.iter().map(|&oi| full_target[oi]).collect();
    // Morphed values drive the drawn wedges + total; labels/hit-tests use the target set.
    let values: Vec<f64> = visible
        .iter()
        .map(|&oi| {
            let tv = full_target[oi];
            let fv = morph_from.get(oi).copied().unwrap_or(tv);
            if data_t_val >= 1.0 {
                tv
            } else {
                fv + (tv - fv) * data_t_val
            }
        })
        .collect();
    let draw_colors = vis_colors.clone();
    let total = values.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let label_values = target_values.clone();
    let label_slices = vis_slices.clone();
    let label_colors = vis_colors.clone();
    let hit_values = target_values.clone();

    let plot = canvas(move |c: &mut Canvas<'_>| {
        let s = c.size();
        let cx0 = s.width / 2.0;
        let cy0 = s.height / 2.0;
        let r = (s.width.min(s.height) / 2.0) - 6.0;
        let ir = r * hole;
        let mut a0 = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
        for (i, &v) in values.iter().enumerate() {
            // Entry sweep: scale each wedge's angle by anim_t so the ring unfolds 0→full.
            let sweep = v / total * std::f64::consts::TAU * anim_t;
            let a1 = a0 + sweep;
            // The active slice pops OUT along its mid-angle for emphasis.
            let mid = a0 + sweep / 2.0;
            let (cx, cy) = if active_slice_val == Some(i) {
                (cx0 + 8.0 * mid.cos(), cy0 + 8.0 * mid.sin())
            } else {
                (cx0, cy0)
            };
            let steps = ((sweep / std::f64::consts::TAU) * 96.0).ceil().max(2.0) as usize;
            let mut p = BezPath::new();
            if ir <= 0.5 {
                p.move_to((cx, cy));
                for k in 0..=steps {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    p.line_to((cx + r * a.cos(), cy + r * a.sin()));
                }
            } else {
                // annulus sector: outer arc forward, inner arc back
                for k in 0..=steps {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    let pt = (cx + r * a.cos(), cy + r * a.sin());
                    if k == 0 {
                        p.move_to(pt);
                    } else {
                        p.line_to(pt);
                    }
                }
                for k in (0..=steps).rev() {
                    let a = a0 + sweep * k as f64 / steps as f64;
                    p.line_to((cx + ir * a.cos(), cy + ir * a.sin()));
                }
            }
            p.close_path();
            c.fill_path(&p, draw_colors[i]);
            a0 = a1;
        }
    })
    .width(size)
    .height(size);

    // Hold slice labels until the sweep finishes (they're placed by full mid-angle).
    let plot = if data_labels && anim_t >= 0.999 && data_t_val >= 0.999 {
        sized_box(stack(children![
            plot.into_widget(),
            pie_data_labels(
                &label_slices,
                &label_values,
                &label_colors,
                total,
                size,
                hole,
                chart.style.label_px(),
                &chart.style.font_family,
            )
        ]))
        .width(size)
        .height(size)
        .into_widget()
    } else {
        plot.into_widget()
    };

    // Interaction: hover (continuous) / press / drag / tap. Hits a slice → pops it out +
    // shows a value tooltip; tap also fires `on_slice`. The gesture lives in its own
    // sub-component (`render_pie_plot`) so `use_bounds()` gives the plot's exact screen
    // rect — needed to turn a hover's global position into slice-local coordinates.
    let activate: Rc<dyn Fn(Offset, Offset)> = {
        let vis_slices = vis_slices.clone();
        let vis_colors = vis_colors.clone();
        Rc::new(move |local: Offset, global: Offset| {
            match hit_pie_slice(local, size, hole, &hit_values, total) {
                Some(si) => {
                    active_slice.set(Some(si));
                    if tooltip && let Some(s) = vis_slices.get(si) {
                        let value = s.value.max(0.0);
                        show_pie_tooltip(
                            global,
                            &s.label,
                            vis_colors.get(si).copied().unwrap_or(palette_color(si)),
                            compact_number(value),
                            value / total * 100.0,
                        );
                    }
                }
                None => {
                    active_slice.set(None);
                    hide_passive();
                }
            }
        })
    };
    let clear: Rc<dyn Fn()> = Rc::new(move || {
        active_slice.set(None);
        hide_passive();
    });
    let tap: Rc<dyn Fn(Offset)> = {
        let tap_values: Vec<f64> = vis_slices.iter().map(|s| s.value.max(0.0)).collect();
        let tap_slices = vis_slices.clone();
        Rc::new(move |local: Offset| {
            if let Some(si) = hit_pie_slice(local, size, hole, &tap_values, total)
                && let Some(cb) = &on_slice
                && let Some(&orig) = visible.get(si)
            {
                cb(orig, tap_slices[si].value);
            }
        })
    };
    let plot = center(component_props(
        render_pie_plot,
        PiePlotProps {
            plot,
            size,
            activate,
            tap,
            clear,
        },
    ))
    .into_widget();

    // The legend lists every named slice (dimmed if hidden); tapping a chip toggles it.
    let legend_el = if legend && slices.iter().any(|s| !s.label.is_empty()) {
        let vis_total: f64 = slices
            .iter()
            .enumerate()
            .filter(|(i, _)| !hidden_set.contains(i))
            .map(|(_, s)| s.value.max(0.0))
            .sum::<f64>()
            .max(f64::MIN_POSITIVE);
        let items: Vec<LegendItem> = slices
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let is_hidden = hidden_set.contains(&i);
                LegendItem {
                    label: s.label.clone(),
                    color: all_colors[i],
                    value: (legend_values && !is_hidden)
                        .then(|| format!("{:.0}%", s.value.max(0.0) / vis_total * 100.0)),
                    hidden: is_hidden,
                }
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

    let Some(el) = legend_el else {
        return column(children![plot])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min)
            .into_widget();
    };
    match legend_position {
        LegendPosition::Bottom => column(children![plot, gap_h(14.0), el]),
        LegendPosition::Top => column(children![el, gap_h(14.0), plot]),
        LegendPosition::Left => {
            return row(children![el, gap_w(20.0), plot])
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .main_axis_size(MainAxisSize::Min)
                .into_widget();
        }
        LegendPosition::Right => {
            return row(children![plot, gap_w(20.0), el])
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .main_axis_size(MainAxisSize::Min)
                .into_widget();
        }
    }
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_size(MainAxisSize::Min)
    .into_widget()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pie_data_labels(
    slices: &[Slice],
    values: &[f64],
    colors: &[Color],
    total: f64,
    size: f64,
    hole: f64,
    label_px: f32,
    font: &Option<String>,
) -> AnyWidget {
    let r = size / 2.0 - 6.0;
    let label_r = r * if hole > 0.0 { (1.0 + hole) / 2.0 } else { 0.6 };
    let mut a0 = -std::f64::consts::FRAC_PI_2;
    let mut items = Vec::new();
    for (i, &value) in values.iter().enumerate() {
        if !value.is_finite() || value <= 0.0 {
            continue;
        }
        let sweep = value / total * std::f64::consts::TAU;
        let mid = a0 + sweep / 2.0;
        a0 += sweep; // advance for EVERY slice, even the ones we skip below
        let pct = value / total * 100.0;
        // A label must fit inside its own slice, or it overruns the neighbours (the old
        // bug: wide "name %" labels on thin slices scattered everywhere). `label_r*sweep`
        // is the slice's tangential room at the label radius. Fit the fullest label that
        // room allows; skip slices too thin for even a percentage (the legend names them).
        let arc = label_r * sweep;
        let name = slices
            .get(i)
            .map(|s| s.label.clone())
            .filter(|label| !label.is_empty());
        let (label, w) = match name {
            Some(name) if arc >= 78.0 => (format!("{name} {:.0}%", pct), 88.0),
            _ if arc >= 26.0 => (format!("{:.0}%", pct), 34.0),
            _ => continue,
        };
        let color = colors
            .get(i)
            .copied()
            .map(contrast_on)
            .unwrap_or(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF));
        // Center the fixed-width label box ON the slice centroid so it actually sits
        // where the slice is (the child text is centered within that box).
        let cx = size / 2.0 + label_r * mid.cos();
        let cy = size / 2.0 + label_r * mid.sin();
        items.push(
            positioned(
                sized_box(
                    apply_font(
                        text(label)
                            .size((label_px - 0.5).max(6.0))
                            .semibold()
                            .color(color),
                        font,
                    )
                    .align(TextAlign::Center),
                )
                .width(w)
                .height(14.0),
            )
            .left((cx - w / 2.0).clamp(0.0, (size - w).max(0.0)))
            .top((cy - 7.0).clamp(0.0, (size - 14.0).max(0.0)))
            .into_widget(),
        );
    }
    sized_box(stack(items))
        .width(size)
        .height(size)
        .into_widget()
}
