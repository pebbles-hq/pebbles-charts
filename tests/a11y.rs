//! Accessibility: every chart emits a semantics node (role + spoken summary + a per-point
//! data read-out) so a screen reader announces the chart AND its data instead of hitting an
//! opaque canvas. Verified headlessly through the framework's `semantics_tree()` — no screen
//! reader or platform needed.

use pebbles::core::Ui;
use pebbles::prelude::*;
use pebbles::render::TextEnv;
use pebbles_charts::{bar_chart, donut_chart, pie_chart, series, slice};

fn white() -> Color {
    Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)
}

#[test]
fn cartesian_chart_emits_a_data_readout_node() {
    pebbles::widgets::overlay::init();
    pebbles::core::focus::init();

    let mut ui = Ui::new();
    let mut env = TextEnv::new();
    ui.mount_root(
        View::new(
            white(),
            bar_chart(
                vec!["Jan".into(), "Feb".into(), "Mar".into()],
                vec![
                    series("Desktop", vec![10.0, 20.0, 30.0]),
                    series("Mobile", vec![5.0, 15.0, 25.0]),
                ],
            )
            .width(300.0)
            .height(200.0),
        )
        .into_widget(),
    );
    ui.layout(&mut env, Size::new(400.0, 400.0));

    let tree = ui.render_tree().semantics_tree();
    let node = tree
        .iter()
        .find(|n| n.props.role == SemanticsRole::Image)
        .expect("chart contributes an Image semantics node");

    // The summary names the chart type + shape.
    assert!(
        node.props.label.contains("Bar chart"),
        "label: {}",
        node.props.label
    );
    assert!(
        node.props.label.contains("2 series"),
        "label: {}",
        node.props.label
    );

    // The value is the full data read-out — both series, every category.
    let value = node.props.value.as_deref().unwrap_or("");
    assert!(
        value.contains("Desktop: Jan 10, Feb 20, Mar 30"),
        "value: {value}"
    );
    assert!(
        value.contains("Mobile: Jan 5, Feb 15, Mar 25"),
        "value: {value}"
    );
}

#[test]
fn custom_a11y_label_is_used() {
    pebbles::widgets::overlay::init();
    pebbles::core::focus::init();

    let mut ui = Ui::new();
    let mut env = TextEnv::new();
    ui.mount_root(
        View::new(
            white(),
            bar_chart(vec!["Q1".into()], vec![series("Rev", vec![100.0])])
                .a11y_label("Quarterly revenue")
                .width(200.0)
                .height(150.0),
        )
        .into_widget(),
    );
    ui.layout(&mut env, Size::new(300.0, 300.0));

    let tree = ui.render_tree().semantics_tree();
    let node = tree
        .iter()
        .find(|n| n.props.role == SemanticsRole::Image)
        .expect("Image node");
    assert_eq!(node.props.label, "Quarterly revenue");
}

#[test]
fn pie_and_donut_read_out_slices_with_percentages() {
    pebbles::widgets::overlay::init();
    pebbles::core::focus::init();

    for (make_donut, kind_word) in [(false, "Pie"), (true, "Donut")] {
        let mut ui = Ui::new();
        let mut env = TextEnv::new();
        let chart = if make_donut {
            donut_chart(vec![slice("A", 3.0), slice("B", 1.0)]).size(200.0)
        } else {
            pie_chart(vec![slice("A", 3.0), slice("B", 1.0)]).size(200.0)
        };
        ui.mount_root(View::new(white(), chart).into_widget());
        ui.layout(&mut env, Size::new(300.0, 300.0));

        let tree = ui.render_tree().semantics_tree();
        let node = tree
            .iter()
            .find(|n| n.props.role == SemanticsRole::Image)
            .expect("Image node");
        assert!(
            node.props.label.contains(kind_word),
            "label: {}",
            node.props.label
        );
        let value = node.props.value.as_deref().unwrap_or("");
        // A = 3/4 = 75%, B = 1/4 = 25%.
        assert!(value.contains("A 3 (75%)"), "value: {value}");
        assert!(value.contains("B 1 (25%)"), "value: {value}");
    }
}
