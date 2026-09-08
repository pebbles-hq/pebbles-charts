# pebbles-charts

Composable, themeable **chart widgets** for the
[Pebbles](https://github.com/pebbles-hq/pebbles) GUI framework — **bar, line, area, pie,
and donut** — drawn on the GPU canvas with a config-driven palette and an auto legend.

![demo](docs/demo.png)

```rust
use pebbles::prelude::*;
use pebbles_charts::{bar_chart, series};

fn view() -> impl IntoWidget {
    bar_chart(
        vec!["Jan".into(), "Feb".into(), "Mar".into()],
        vec![
            series("Desktop", vec![186.0, 305.0, 237.0]),
            series("Mobile", vec![80.0, 200.0, 120.0]),
        ],
    )
    .width(520.0)
    .height(260.0)
}
```

## Charts

| Constructor | Chart |
|---|---|
| `bar_chart(categories, series)` | grouped bars |
| `line_chart(categories, series)` | line(s) with points |
| `area_chart(categories, series)` | filled line(s) |
| `pie_chart(slices)` | pie |
| `donut_chart(slices)` | pie with a hole (`.hole(frac)` to tune) |

- **Cartesian** charts take `Vec<String>` categories + `Vec<Series>`
  (`series("Label", vec![..])`), and support `.width` `.height` `.legend(bool)`
  `.grid(bool)`.
- **Pie / donut** take `Vec<Slice>` (`slice("Label", value)`), and support `.size`
  `.hole` `.legend`.

## Theming

- **Chrome** (grid lines, axis labels) follows the app's `theme()`, so charts match
  light/dark automatically.
- **Series colors** come from a built-in [`palette`] (6 distinct hues, cycled), or set
  one explicitly: `series("Desktop", vals).color(Color::from_rgba8(0x63,0x66,0xF1,0xFF))`
  / `slice("Chrome", 62.0).color(..)`.
- **Legend** is generated from series/slice labels; hide with `.legend(false)`.

## Run the sample

```sh
cargo run -p demo
# headless screenshot (no display needed):
SHOT=1180:900:/tmp/charts.rgba cargo run -p demo
```

## Roadmap

- Stacked bars, horizontal bars, and negative values.
- Y-axis tick labels and value tooltips on hover.
- Radar and radial (progress) charts.
- Entry animations (grow-in bars, draw-on lines).

## License

MIT OR Apache-2.0.
