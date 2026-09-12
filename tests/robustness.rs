//! Production robustness: a chart must never panic on malformed input — mismatched
//! series/category lengths, `NaN`/inf, negative or zero values, out-of-range windows, empty
//! data. Each case is laid out AND painted (the painter closures run during `paint`, so this
//! exercises the actual draw/index code, not just the builders). No GPU needed — painting
//! records the op-list on the CPU.

use pebbles::core::Ui;
use pebbles::prelude::*;
use pebbles::render::{Scene, TextEnv};
use pebbles_charts::*;

fn white() -> Color {
    Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)
}

/// Mount, lay out, and paint `w` — panics here fail the test.
fn paint(w: impl IntoWidget) {
    pebbles::widgets::overlay::init();
    pebbles::core::focus::init();
    let mut ui = Ui::new();
    let mut env = TextEnv::new();
    ui.mount_root(View::new(white(), w).into_widget());
    let size = Size::new(600.0, 400.0);
    for _ in 0..3 {
        ui.rebuild_if_dirty();
        ui.layout(&mut env, size);
        let mut scene = Scene::new();
        if !ui.paint(&mut env, &mut scene) {
            break;
        }
    }
}

fn cats(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("C{i}")).collect()
}

#[test]
fn series_shorter_than_categories_does_not_panic() {
    paint(
        bar_chart(cats(5), vec![series("x", vec![1.0, 2.0])])
            .width(400.0)
            .height(240.0),
    );
    paint(
        line_chart(cats(5), vec![series("x", vec![1.0])])
            .width(400.0)
            .height(240.0),
    );
    paint(
        area_chart(cats(5), vec![series("x", Vec::<f64>::new())])
            .width(400.0)
            .height(240.0),
    );
    paint(
        stacked_bar_chart(
            cats(4),
            vec![series("a", vec![1.0]), series("b", vec![2.0, 3.0])],
        )
        .width(400.0)
        .height(240.0),
    );
}

#[test]
fn series_longer_than_categories_does_not_panic() {
    paint(
        bar_chart(cats(2), vec![series("x", vec![1.0, 2.0, 3.0, 4.0, 5.0])])
            .width(400.0)
            .height(240.0),
    );
    paint(
        line_chart(cats(1), vec![series("x", vec![1.0, 2.0, 3.0])])
            .width(400.0)
            .height(240.0),
    );
    paint(
        horizontal_bar_chart(cats(2), vec![series("x", vec![1.0, 2.0, 3.0])])
            .width(400.0)
            .height(240.0),
    );
}

#[test]
fn non_finite_and_negative_values_do_not_panic() {
    let vals = vec![
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -50.0,
        0.0,
        100.0,
    ];
    paint(
        bar_chart(cats(6), vec![series("x", vals.clone())])
            .width(400.0)
            .height(240.0),
    );
    paint(
        line_chart(cats(6), vec![series("x", vals.clone())])
            .curve(CurveInterpolation::Smooth)
            .width(400.0)
            .height(240.0),
    );
    paint(
        area_chart(
            cats(6),
            vec![series("x", vec![f64::NAN, f64::NAN, f64::NAN])],
        )
        .width(400.0)
        .height(240.0),
    );
    // all-NaN series → empty-state path
    paint(
        bar_chart(
            cats(3),
            vec![series("x", vec![f64::NAN, f64::NAN, f64::NAN])],
        )
        .width(400.0)
        .height(240.0),
    );
}

#[test]
fn error_bars_and_annotations_out_of_range_do_not_panic() {
    paint(
        line_chart(
            cats(3),
            vec![series_with_errors("x", vec![1.0, 2.0, 3.0], vec![1000.0])],
        )
        .annotation(annotation(99, 5.0, "way out of range"))
        .annotation(annotation(0, f64::NAN, "nan value"))
        .width(400.0)
        .height(240.0),
    );
}

#[test]
fn empty_and_degenerate_inputs_do_not_panic() {
    paint(bar_chart(vec![], vec![]).width(400.0).height(240.0));
    paint(bar_chart(cats(3), vec![]).width(400.0).height(240.0));
    paint(
        line_chart(cats(0), vec![series("x", Vec::<f64>::new())])
            .width(400.0)
            .height(240.0),
    );
    paint(
        bar_chart(cats(3), vec![series("x", vec![1.0, 2.0, 3.0])])
            .width(0.0)
            .height(0.0),
    );
    paint(pie_chart(vec![]).size(200.0));
    paint(sparkline(vec![]).width(200.0));
}

#[test]
fn pie_with_negative_zero_nan_does_not_panic() {
    paint(
        pie_chart(vec![
            slice("a", -5.0),
            slice("b", 0.0),
            slice("c", f64::NAN),
            slice("d", 10.0),
        ])
        .size(200.0),
    );
    paint(donut_chart(vec![slice("only", 0.0)]).size(200.0));
    paint(
        pie_chart(vec![slice("a", f64::INFINITY), slice("b", 1.0)])
            .data_labels(true)
            .size(200.0),
    );
}

#[test]
fn out_of_range_windows_and_top_n_do_not_panic() {
    paint(
        bar_chart(cats(3), vec![series("x", vec![1.0, 2.0, 3.0])])
            .category_window(1, 99)
            .width(400.0)
            .height(240.0),
    );
    paint(
        bar_chart(cats(3), vec![series("x", vec![1.0, 2.0, 3.0])])
            .category_window(5, 2)
            .width(400.0)
            .height(240.0),
    );
    paint(
        bar_chart(cats(3), vec![series("x", vec![1.0, 2.0, 3.0])])
            .top_n(99)
            .width(400.0)
            .height(240.0),
    );
    paint(
        bar_chart(cats(3), vec![series("x", vec![1.0, 2.0, 3.0])])
            .top_n(0)
            .width(400.0)
            .height(240.0),
    );
}

#[test]
fn specialized_charts_with_degenerate_inputs_do_not_panic() {
    paint(radar_chart(cats(3), vec![series("x", vec![1.0])]).size(200.0));
    paint(
        scatter_chart(vec![point_series(
            "p",
            vec![point(f64::NAN, 1.0), point(2.0, f64::INFINITY)],
        )])
        .width(300.0)
        .height(200.0),
    );
    paint(
        candlestick_chart(cats(2), vec![candle(1.0, 5.0, 0.0, 3.0)])
            .width(300.0)
            .height(200.0),
    );
    paint(
        heatmap_chart(
            cats(2),
            cats(2),
            vec![heat_cell(9, 9, 1.0), heat_cell(0, 0, f64::NAN)],
        )
        .width(300.0)
        .height(200.0),
    );
    paint(funnel_chart(vec![]).width(300.0).height(200.0));
    paint(sankey_chart(vec![]).width(300.0).height(200.0));
    paint(progress_ring("x", f64::NAN, 0.0).size(120.0));
    paint(gauge_chart("x", 200.0, 100.0).size(120.0));
}
