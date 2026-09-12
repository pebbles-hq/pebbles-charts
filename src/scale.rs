//! Plot geometry constants and the numeric scales: `LinearScale`, `ValueAxis`, `XValueAxis`, tick + domain math, and number formatting.

use crate::config::*;

pub(crate) const PLOT_TOP: f64 = 12.0;
pub(crate) const PLOT_RIGHT: f64 = 10.0;
pub(crate) const PLOT_BOTTOM: f64 = 8.0;
pub(crate) const PLOT_LEFT: f64 = 10.0;
pub(crate) const Y_AXIS_WIDTH: f64 = 42.0;
pub(crate) const Y_AXIS_GAP: f64 = 8.0;
pub(crate) const TOOLTIP_OFFSET: f64 = 14.0;

#[derive(Clone, Copy)]
pub(crate) struct LinearScale {
    d0: f64,
    d1: f64,
    p0: f64,
    p1: f64,
}

impl LinearScale {
    pub(crate) fn new(d0: f64, d1: f64, p0: f64, p1: f64) -> Self {
        LinearScale { d0, d1, p0, p1 }
    }

    pub(crate) fn map(&self, value: f64) -> f64 {
        let span = self.d1 - self.d0;
        if span.abs() <= f64::EPSILON {
            return (self.p0 + self.p1) / 2.0;
        }
        let t = (value - self.d0) / span;
        self.p0 + (self.p1 - self.p0) * t
    }
}

#[derive(Clone)]
pub(crate) struct ValueAxis {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) ticks: Vec<f64>,
}

impl ValueAxis {
    pub(crate) fn from_values(
        vals: &[Vec<f64>],
        explicit: Option<(f64, f64)>,
        tick_count: usize,
    ) -> Self {
        let tick_count = tick_count.max(2);
        let is_explicit = explicit.is_some();
        let (mut lo, mut hi) = explicit.unwrap_or_else(|| data_domain(vals));
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        if !lo.is_finite() || !hi.is_finite() {
            lo = 0.0;
            hi = 1.0;
        }
        if !is_explicit {
            lo = lo.min(0.0);
            hi = hi.max(0.0);
        }
        if (hi - lo).abs() <= f64::EPSILON {
            let pad = if hi.abs() < 1.0 { 1.0 } else { hi.abs() * 0.1 };
            lo -= pad;
            hi += pad;
        }
        if is_explicit {
            return ValueAxis {
                min: lo,
                max: hi,
                ticks: linear_ticks(lo, hi, tick_count),
            };
        }
        let step = nice_step((hi - lo) / (tick_count - 1) as f64);
        let nice_lo = (lo / step).floor() * step;
        let nice_hi = (hi / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut v = nice_lo;
        let guard = tick_count.saturating_mul(4).max(16);
        for _ in 0..guard {
            if v > nice_hi + step * 0.5 {
                break;
            }
            ticks.push(clean_zero(v));
            v += step;
        }
        if ticks.len() < 2 {
            ticks = vec![nice_lo, nice_hi];
        }
        ValueAxis {
            min: nice_lo,
            max: nice_hi,
            ticks,
        }
    }

    pub(crate) fn scale(&self, top: f64, bottom: f64) -> LinearScale {
        LinearScale::new(self.min, self.max, bottom, top)
    }
}

#[derive(Clone)]
pub(crate) struct XValueAxis {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) ticks: Vec<f64>,
    pub(crate) scale: AxisScale,
}

impl XValueAxis {
    pub(crate) fn from_values(
        vals: &[f64],
        explicit: Option<(f64, f64)>,
        tick_count: usize,
        scale: AxisScale,
    ) -> Self {
        if scale == AxisScale::Log10 {
            let tick_count = tick_count.max(2);
            let (mut lo, mut hi) = explicit.unwrap_or_else(|| {
                let filtered = vals
                    .iter()
                    .copied()
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .collect::<Vec<_>>();
                data_domain(&[filtered])
            });
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            if !lo.is_finite() || !hi.is_finite() || hi <= 0.0 {
                lo = 1.0;
                hi = 10.0;
            }
            lo = lo.max(f64::MIN_POSITIVE);
            hi = hi.max(lo * 10.0);
            return XValueAxis {
                min: lo,
                max: hi,
                ticks: log_ticks(lo, hi, tick_count),
                scale,
            };
        }
        let filtered: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
        let source = vec![filtered];
        let axis = ValueAxis::from_values(&source, explicit, tick_count);
        XValueAxis {
            min: axis.min,
            max: axis.max,
            ticks: axis.ticks,
            scale,
        }
    }

    pub(crate) fn map(&self, value: f64, left: f64, right: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self.scale {
            AxisScale::Linear | AxisScale::Time => {
                Some(LinearScale::new(self.min, self.max, left, right).map(value))
            }
            AxisScale::Log10 => {
                if value <= 0.0 || self.min <= 0.0 || self.max <= 0.0 {
                    None
                } else {
                    Some(
                        LinearScale::new(self.min.log10(), self.max.log10(), left, right)
                            .map(value.log10()),
                    )
                }
            }
        }
    }
}

pub(crate) fn linear_ticks(lo: f64, hi: f64, tick_count: usize) -> Vec<f64> {
    let tick_count = tick_count.max(2);
    (0..tick_count)
        .map(|i| clean_zero(lo + (hi - lo) * i as f64 / (tick_count - 1) as f64))
        .collect()
}

pub(crate) fn log_ticks(lo: f64, hi: f64, tick_count: usize) -> Vec<f64> {
    let tick_count = tick_count.max(2);
    let start = lo.max(f64::MIN_POSITIVE).log10().floor() as i32;
    let end = hi.max(lo).log10().ceil() as i32;
    let mut ticks: Vec<f64> = (start..=end)
        .map(|exp| 10_f64.powi(exp))
        .filter(|v| *v >= lo && *v <= hi)
        .collect();
    if ticks.len() < 2 {
        ticks = linear_ticks(lo, hi, tick_count);
    }
    ticks
}

pub(crate) fn data_domain(vals: &[Vec<f64>]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in vals.iter().flat_map(|series| series.iter()) {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if lo.is_finite() && hi.is_finite() {
        (lo, hi)
    } else {
        (0.0, 1.0)
    }
}

pub(crate) fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10_f64.powf(exp);
    let frac = raw / base;
    let nice = if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

pub(crate) fn clean_zero(v: f64) -> f64 {
    if v.abs() <= f64::EPSILON { 0.0 } else { v }
}

pub(crate) fn compact_number(value: f64) -> String {
    let abs = value.abs();
    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (value / 1_000.0, "k")
    } else {
        (value, "")
    };
    let decimals = if scaled.abs() >= 100.0 || scaled.fract().abs() <= 0.001 {
        0
    } else {
        1
    };
    format!("{scaled:.decimals$}{suffix}")
}
