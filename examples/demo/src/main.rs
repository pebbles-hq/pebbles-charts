//! A sample app for `pebbles-charts` — one card per chart type (bar, line, area, pie,
//! donut) with sample data, so you can see the whole set at a glance.
//!
//! Run it: `cargo run -p demo`
//! Headless screenshot: `SHOT=1180:900:/tmp/charts.rgba cargo run -p demo`

use pebbles::prelude::*;
use pebbles_charts::{area_chart, bar_chart, donut_chart, line_chart, pie_chart, series, slice};

mod capture;

fn months() -> Vec<String> {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun"].iter().map(|s| s.to_string()).collect()
}

fn card(title: &str, sub: &str, chart: impl IntoWidget) -> impl IntoWidget {
    let c = theme().colors;
    container()
        .decoration(
            BoxDecoration::new()
                .color(c.card)
                .radius(BorderRadius::all(14.0))
                .border(Border::new(c.border, 1.0)),
        )
        .padding(EdgeInsets::all(20.0))
        .child(
            column(children![
                text(title.to_string()).size(16.0).bold().color(c.foreground),
                gap_h(2.0),
                text(sub.to_string()).size(12.5).color(c.muted_foreground),
                gap_h(18.0),
                chart.into_widget(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min),
        )
}

fn app() -> AnyWidget {
    let c = theme().colors;

    let bar = card(
        "Bar Chart",
        "Desktop vs. mobile visitors",
        bar_chart(
            months(),
            vec![
                series("Desktop", vec![186.0, 305.0, 237.0, 273.0, 209.0, 264.0]),
                series("Mobile", vec![80.0, 200.0, 120.0, 190.0, 130.0, 140.0]),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let line = card(
        "Line Chart",
        "Monthly revenue (k)",
        line_chart(months(), vec![series("Revenue", vec![12.0, 19.0, 15.0, 27.0, 24.0, 33.0])])
            .width(460.0)
            .height(240.0),
    );

    let area = card(
        "Area Chart",
        "Active users",
        area_chart(months(), vec![series("Users", vec![320.0, 410.0, 505.0, 480.0, 620.0, 700.0])])
            .width(460.0)
            .height(240.0),
    );

    let pie = card(
        "Pie Chart",
        "Traffic by browser",
        pie_chart(vec![
            slice("Chrome", 62.0),
            slice("Safari", 18.0),
            slice("Firefox", 9.0),
            slice("Edge", 7.0),
            slice("Other", 4.0),
        ])
        .size(240.0),
    );

    let donut = card(
        "Donut Chart",
        "Storage used",
        donut_chart(vec![slice("Docs", 42.0), slice("Media", 28.0), slice("Apps", 18.0), slice("Free", 12.0)])
            .size(240.0),
    );

    container()
        .color(c.background)
        .child(scroll_view(
            container().padding(EdgeInsets::all(28.0)).child(
                column(children![
                    text("Pebbles Charts").size(24.0).bold().color(c.foreground),
                    gap_h(4.0),
                    text("Composable, themeable chart widgets — drawn on the GPU canvas.")
                        .size(13.5)
                        .color(c.muted_foreground),
                    gap_h(24.0),
                    wrap(children![bar, line, area, pie, donut]).spacing(24.0).run_spacing(24.0),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .main_axis_size(MainAxisSize::Min),
            ),
        ))
        .into_widget()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Theme::light().make_current();
    if let Ok(spec) = std::env::var("SHOT") {
        return capture::shot(&spec, app, theme().colors.background);
    }
    App::new(component(app)).title("Pebbles Charts").size(1180, 900).background(theme().colors.background).run()
}
