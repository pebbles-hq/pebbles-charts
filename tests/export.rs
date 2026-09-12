//! Exporting a chart to PNG via the framework's offscreen capture API. A chart (a widget)
//! can't rasterize itself, but `pebbles::shell::capture` renders any widget headlessly —
//! so this is the export path for charts/reports. `#[ignore]` (needs a GPU): run with
//! `cargo test --test export -- --ignored`.

#![cfg(not(target_family = "wasm"))]

use pebbles::prelude::*;
use pebbles::shell::capture;
use pebbles_charts::{bar_chart, series};

#[test]
#[ignore = "requires a GPU"]
fn chart_exports_to_png() {
    let chart = bar_chart(
        vec!["Jan".into(), "Feb".into(), "Mar".into()],
        vec![series("Rev", vec![10.0, 20.0, 15.0])],
    )
    .width(400.0)
    .height(240.0)
    .into_widget();

    let png = capture::capture_png(chart, 400, 260, Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF))
        .expect("capture chart to png");
    // Valid PNG signature + non-trivial payload.
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG header");
    assert!(
        png.len() > 1000,
        "encoded PNG has real content ({} bytes)",
        png.len()
    );
}
