//! A sample app for `pebbles-charts` — one card per chart family with sample data,
//! so you can see the whole set at a glance.
//!
//! Run it: `cargo run -p demo`
//! Headless screenshot: `SHOT=1180:900:/tmp/charts.rgba cargo run -p demo`

use pebbles::prelude::*;
use pebbles_charts::{
    area_chart, bar_chart, bubble_chart, bubble_point, candle, candlestick_chart, combo_chart,
    combo_series, donut_chart, funnel_chart, gauge_chart, heat_cell, heatmap_chart,
    horizontal_bar_chart, line_chart, percent_stacked_area_chart, percent_stacked_bar_chart,
    pie_chart, point, point_series, progress_ring, radar_chart, sankey_chart, sankey_link,
    scatter_chart, series, series_with_gaps, slice, sparkline, stacked_area_chart,
    stacked_bar_chart, stepped_line_chart, CategoryLabelMode, CurveInterpolation, SeriesKind,
};

mod capture;

fn months() -> Vec<String> {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun"].iter().map(|s| s.to_string()).collect()
}

fn card(title: &str, sub: &str, chart: impl IntoWidget) -> impl IntoWidget {
    let c = theme().colors;
    container()
        .width(500.0)
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
        .x_axis_title("Month")
        .y_axis_title("Visitors")
        .vertical_grid(true)
        .reference_line(pebbles_charts::reference_line(250.0).label("Goal"))
        .reference_band(pebbles_charts::reference_band(90.0, 140.0).label("Baseline"))
        .width(460.0)
        .height(240.0),
    );

    let line = card(
        "Line Chart",
        "Monthly revenue with a gap",
        line_chart(
            months(),
            vec![series_with_gaps(
                "Revenue",
                vec![Some(12.0), Some(19.0), None, Some(27.0), Some(24.0), Some(33.0)],
            )],
        )
        .data_labels(true)
        .width(460.0)
        .height(240.0),
    );

    let area = card(
        "Area Chart",
        "Active users, smooth curve",
        area_chart(months(), vec![series("Users", vec![320.0, 410.0, 505.0, 480.0, 620.0, 700.0])])
            .curve(CurveInterpolation::Smooth)
            .area_gradient(true)
            .width(460.0)
            .height(240.0),
    );

    let stacked_bar = card(
        "Stacked Bar",
        "Requests by status",
        stacked_bar_chart(
            months(),
            vec![
                series("Success", vec![142.0, 188.0, 220.0, 260.0, 240.0, 300.0]),
                series("Retry", vec![18.0, 26.0, 24.0, 34.0, 30.0, 28.0]),
                series("Failed", vec![6.0, 9.0, 12.0, 10.0, 8.0, 11.0]),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let percent_bar = card(
        "100% Stacked Bar",
        "Channel share",
        percent_stacked_bar_chart(
            months(),
            vec![
                series("Organic", vec![42.0, 38.0, 44.0, 40.0, 45.0, 47.0]),
                series("Paid", vec![30.0, 34.0, 28.0, 32.0, 30.0, 29.0]),
                series("Referral", vec![28.0, 28.0, 28.0, 28.0, 25.0, 24.0]),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let horizontal = card(
        "Horizontal Bar",
        "Top pipeline stages",
        horizontal_bar_chart(
            vec!["Lead".into(), "Qualified".into(), "Proposal".into(), "Won".into(), "Lost".into()],
            vec![series("Deals", vec![128.0, 96.0, 54.0, 32.0, 20.0])],
        )
        .top_n(4)
        .category_label_mode(CategoryLabelMode::Truncate(4))
        .width(460.0)
        .height(240.0),
    );

    let stepped = card(
        "Stepped Line",
        "Capacity plan",
        stepped_line_chart(months(), vec![series("Capacity", vec![12.0, 12.0, 18.0, 18.0, 24.0, 30.0])])
            .width(460.0)
            .height(240.0),
    );

    let stacked_area = card(
        "Stacked Area",
        "Usage by product",
        stacked_area_chart(
            months(),
            vec![
                series("Core", vec![40.0, 60.0, 70.0, 88.0, 96.0, 120.0]),
                series("Plus", vec![22.0, 28.0, 35.0, 42.0, 60.0, 72.0]),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let percent_area = card(
        "100% Stacked Area",
        "Workspace mix",
        percent_stacked_area_chart(
            months(),
            vec![
                series("Small", vec![50.0, 46.0, 40.0, 36.0, 30.0, 28.0]),
                series("Team", vec![30.0, 34.0, 38.0, 42.0, 46.0, 48.0]),
                series("Org", vec![20.0, 20.0, 22.0, 22.0, 24.0, 24.0]),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let combo = card(
        "Combo Chart",
        "Bookings and conversion",
        combo_chart(
            months(),
            vec![
                combo_series("Bookings", vec![32.0, 46.0, 40.0, 62.0, 70.0, 82.0], SeriesKind::Bar),
                combo_series("Conversion", vec![8.0, 11.0, 10.0, 14.0, 15.0, 17.0], SeriesKind::Line),
            ],
        )
        .right_y_axis(0.0, 20.0)
        .right_y_axis_title("Conversion")
        .width(460.0)
        .height(240.0),
    );

    let scatter = card(
        "Bubble Chart",
        "Load vs. latency",
        bubble_chart(vec![point_series(
            "Endpoints",
            vec![
                bubble_point(10.0, 22.0, 5.0),
                bubble_point(24.0, 34.0, 8.0),
                bubble_point(38.0, 30.0, 6.0),
                bubble_point(48.0, 54.0, 12.0),
                bubble_point(65.0, 62.0, 9.0),
            ],
        )])
        .x_log()
        .width(460.0)
        .height(240.0),
    );

    let dots = card(
        "Point Scatter",
        "Scores by effort",
        scatter_chart(vec![point_series(
            "Teams",
            vec![point(1.0, 2.0), point(2.0, 5.0), point(3.0, 4.0), point(4.0, 8.0), point(5.0, 7.0)],
        )])
        .width(460.0)
        .height(240.0),
    );

    let radar = card(
        "Radar Chart",
        "Team capability",
        radar_chart(
            vec!["Speed".into(), "Quality".into(), "Cost".into(), "Coverage".into(), "Risk".into()],
            vec![
                series("Current", vec![72.0, 84.0, 58.0, 76.0, 64.0]),
                series("Target", vec![86.0, 90.0, 70.0, 88.0, 72.0]),
            ],
        )
        .size(240.0),
    );

    let radial = card("Progress Ring", "Rollout completion", progress_ring("Complete", 78.0, 100.0).size(180.0));
    let gauge = card("Gauge", "Resource pressure", gauge_chart("Pressure", 64.0, 100.0).size(180.0));

    let candles = card(
        "Candlestick",
        "Daily price movement",
        candlestick_chart(
            vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into()],
            vec![
                candle(32.0, 38.0, 29.0, 36.0),
                candle(36.0, 40.0, 33.0, 34.0),
                candle(34.0, 41.0, 32.0, 39.0),
                candle(39.0, 43.0, 37.0, 42.0),
                candle(42.0, 44.0, 35.0, 38.0),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let heatmap = card(
        "Heatmap",
        "Hourly activity",
        heatmap_chart(
            vec!["00".into(), "06".into(), "12".into(), "18".into()],
            vec!["API".into(), "UI".into(), "Jobs".into()],
            vec![
                heat_cell(0, 0, 12.0),
                heat_cell(1, 0, 28.0),
                heat_cell(2, 0, 44.0),
                heat_cell(3, 0, 30.0),
                heat_cell(0, 1, 8.0),
                heat_cell(1, 1, 18.0),
                heat_cell(2, 1, 52.0),
                heat_cell(3, 1, 41.0),
                heat_cell(0, 2, 4.0),
                heat_cell(1, 2, 12.0),
                heat_cell(2, 2, 22.0),
                heat_cell(3, 2, 18.0),
            ],
        )
        .width(460.0)
        .height(240.0),
    );

    let funnel = card(
        "Funnel",
        "Activation journey",
        funnel_chart(vec![slice("Visits", 100.0), slice("Signup", 64.0), slice("Setup", 42.0), slice("Active", 28.0)])
            .width(360.0)
            .height(240.0),
    );

    let sankey = card(
        "Sankey",
        "Traffic flow",
        sankey_chart(vec![
            sankey_link("Landing", "Docs", 42.0),
            sankey_link("Landing", "Signup", 28.0),
            sankey_link("Docs", "Signup", 18.0),
        ])
        .width(460.0)
        .height(220.0),
    );

    let mini = card("Sparkline", "Last 24 hours", sparkline(vec![8.0, 12.0, 9.0, 16.0, 14.0, 22.0, 18.0]).width(460.0));

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
        .data_labels(true)
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
                    wrap(children![
                        bar,
                        stacked_bar,
                        percent_bar,
                        horizontal,
                        line,
                        stepped,
                        area,
                        stacked_area,
                        percent_area,
                        combo,
                        scatter,
                        dots,
                        radar,
                        radial,
                        gauge,
                        candles,
                        heatmap,
                        funnel,
                        sankey,
                        mini,
                        pie,
                        donut,
                    ])
                    .spacing(24.0)
                    .run_spacing(24.0),
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
