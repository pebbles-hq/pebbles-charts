//! Financial charts: candlestick and OHLC.

use crate::data::*;
use crate::render::*;
use crate::scale::*;
use pebbles::prelude::*;

pub struct CandlestickChart {
    categories: Vec<String>,
    candles: Vec<Candle>,
    width: f64,
    height: f64,
    ohlc: bool,
}

pub type OhlcChart = CandlestickChart;

pub fn candlestick_chart(categories: Vec<String>, candles: Vec<Candle>) -> CandlestickChart {
    CandlestickChart {
        categories,
        candles,
        width: 520.0,
        height: 260.0,
        ohlc: false,
    }
}

pub fn ohlc_chart(categories: Vec<String>, candles: Vec<Candle>) -> OhlcChart {
    CandlestickChart {
        categories,
        candles,
        width: 520.0,
        height: 260.0,
        ohlc: true,
    }
}

impl CandlestickChart {
    pub fn width(mut self, w: f64) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }
}

impl IntoWidget for CandlestickChart {
    fn into_widget(self) -> AnyWidget {
        let CandlestickChart {
            categories,
            candles,
            width,
            height,
            ohlc,
        } = self;
        let vals = vec![
            candles
                .iter()
                .flat_map(|c| [c.open, c.high, c.low, c.close])
                .collect::<Vec<_>>(),
        ];
        let axis = ValueAxis::from_values(&vals, None, 5);
        let plot = canvas(move |c: &mut Canvas<'_>| {
            let s = c.size();
            let x0 = PLOT_LEFT;
            let x1 = s.width - PLOT_RIGHT;
            let y0 = PLOT_TOP;
            let y1 = s.height - PLOT_BOTTOM;
            let pw = (x1 - x0).max(1.0);
            let n = candles.len().max(1);
            let slot = pw / n as f64;
            let y_scale = axis.scale(y0, y1);
            let up = Color::from_rgba8(0x22, 0xC5, 0x5E, 0xFF);
            let down = Color::from_rgba8(0xF4, 0x3F, 0x5E, 0xFF);
            for (i, candle) in candles.iter().enumerate() {
                let x = x0 + slot * (i as f64 + 0.5);
                let color = if candle.close >= candle.open {
                    up
                } else {
                    down
                };
                c.stroke_line(
                    Offset::new(x, y_scale.map(candle.low)),
                    Offset::new(x, y_scale.map(candle.high)),
                    1.4,
                    color,
                );
                let open = y_scale.map(candle.open);
                let close = y_scale.map(candle.close);
                if ohlc {
                    c.stroke_line(
                        Offset::new(x - slot * 0.25, open),
                        Offset::new(x, open),
                        2.0,
                        color,
                    );
                    c.stroke_line(
                        Offset::new(x, close),
                        Offset::new(x + slot * 0.25, close),
                        2.0,
                        color,
                    );
                } else {
                    c.fill_rrect(
                        Rect::new(
                            x - slot * 0.24,
                            open.min(close),
                            x + slot * 0.24,
                            open.max(close).max(open.min(close) + 1.0),
                        ),
                        2.0,
                        color,
                    );
                }
            }
        })
        .width(width)
        .height(height);
        chart_with_categories(plot, categories, theme().colors.muted_foreground)
    }
}
