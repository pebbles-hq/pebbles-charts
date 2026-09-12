//! Smoke tests for the styling / state / annotation / error-bar feature surface: each
//! chart lays out headlessly without panicking, exercising the ChartStyle + aspect-ratio +
//! loading/error + error-bar + annotation code paths (regression guard for the threading).

use pebbles::core::Ui;
use pebbles::prelude::*;
use pebbles::render::TextEnv;
use pebbles_charts::{
    annotation, bar_chart, line_chart, series, series_with_errors, ChartStyle,
};

fn white() -> Color {
    Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)
}

fn lay_out(w: impl IntoWidget) {
    pebbles::widgets::overlay::init();
    pebbles::core::focus::init();
    let mut ui = Ui::new();
    let mut env = TextEnv::new();
    ui.mount_root(View::new(white(), w).into_widget());
    ui.layout(&mut env, Size::new(600.0, 400.0));
    // A second frame settles any mount-time animation writes.
    ui.rebuild_if_dirty();
    ui.layout(&mut env, Size::new(600.0, 400.0));
}

#[test]
fn chart_style_and_aspect_lay_out() {
    lay_out(
        bar_chart(vec!["A".into(), "B".into(), "C".into()], vec![series("v", vec![3.0, 5.0, 4.0])])
            .style(
                ChartStyle::new()
                    .grid_color(white())
                    .line_width(4.0)
                    .point_radius(5.0)
                    .area_alpha(0.3)
                    .bar_radius(2.0)
                    .label_size(14.0)
                    .font_family("Inter"),
            )
            .aspect_ratio(2.0)
            .width(400.0),
    );
}

#[test]
fn loading_and_error_states_lay_out() {
    lay_out(bar_chart(vec!["A".into()], vec![series("v", vec![1.0])]).loading(true).width(300.0).height(200.0));
    lay_out(bar_chart(vec!["A".into()], vec![series("v", vec![1.0])]).error("boom").width(300.0).height(200.0));
    // Empty data → the "No data" panel path.
    lay_out(bar_chart(vec![], vec![]).width(300.0).height(200.0));
}

#[test]
fn error_bars_and_annotations_lay_out() {
    lay_out(
        line_chart(
            vec!["Jan".into(), "Feb".into(), "Mar".into()],
            vec![series_with_errors("Rev", vec![10.0, 20.0, 15.0], vec![2.0, 3.0, 1.5])],
        )
        .annotation(annotation(1, 20.0, "peak"))
        .annotation(annotation(2, 15.0, "dip").color(white()))
        .width(400.0)
        .height(240.0),
    );
}
