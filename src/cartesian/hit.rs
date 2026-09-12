//! Pointer hit-testing for cartesian plots (category + series resolution).

use super::*;
use crate::cartesian::Kind;
use pebbles::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveDatum {
    pub(crate) category: usize,
    pub(crate) series: Option<usize>,
}

pub(crate) fn hit_category(
    pos: Offset,
    width: f64,
    height: f64,
    ncat: usize,
    pad: EdgeInsets,
) -> Option<usize> {
    let ncat = ncat.max(1);
    let pw = (width - pad.left - pad.right).max(1.0);
    let y1 = (height - pad.bottom).max(pad.top + 1.0);
    if pos.x < pad.left || pos.x > pad.left + pw || pos.y < pad.top || pos.y > y1 {
        return None;
    }
    let slot = pw / ncat as f64;
    Some(
        ((pos.x - pad.left) / slot)
            .floor()
            .clamp(0.0, (ncat - 1) as f64) as usize,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hit_cartesian_datum(
    kind: Kind,
    pos: Offset,
    width: f64,
    height: f64,
    ncat: usize,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
    pad: EdgeInsets,
) -> Option<ActiveDatum> {
    let category = hit_category(pos, width, height, ncat, pad)?;
    let series = match kind {
        Kind::Bar => hit_bar_series(pos, width, ncat, category, vals, pad),
        Kind::StackedBar | Kind::PercentStackedBar | Kind::HorizontalBar => {
            hit_bar_series(pos, width, ncat, category, vals, pad)
        }
        Kind::Line
        | Kind::SteppedLine
        | Kind::Area
        | Kind::StackedArea
        | Kind::PercentStackedArea
        | Kind::Combo
        | Kind::Sparkline => hit_nearest_line_series(pos, height, category, vals, axis, pad),
    };
    Some(ActiveDatum { category, series })
}

pub(crate) fn hit_bar_series(
    pos: Offset,
    width: f64,
    ncat: usize,
    category: usize,
    vals: &[Vec<f64>],
    pad: EdgeInsets,
) -> Option<usize> {
    let nser = vals.len().max(1);
    let pw = (width - pad.left - pad.right).max(1.0);
    let slot = pw / ncat.max(1) as f64;
    let group_w = slot * 0.7;
    let bw = group_w / nser as f64;
    let cx = pad.left + pw * (category as f64 + 0.5) / ncat.max(1) as f64;
    let group_x = cx - group_w / 2.0;
    let local = ((pos.x - group_x) / bw).floor() as isize;
    let preferred = (local >= 0 && (local as usize) < vals.len()).then_some(local as usize);
    preferred
        .filter(|&si| {
            vals.get(si)
                .and_then(|sv| sv.get(category))
                .is_some_and(|v| v.is_finite())
        })
        .or_else(|| {
            vals.iter()
                .enumerate()
                .filter(|(_, sv)| sv.get(category).is_some_and(|v| v.is_finite()))
                .min_by(|(ai, _), (bi, _)| {
                    let ax = group_x + (*ai as f64 + 0.5) * bw;
                    let bx = group_x + (*bi as f64 + 0.5) * bw;
                    (pos.x - ax)
                        .abs()
                        .partial_cmp(&(pos.x - bx).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(si, _)| si)
        })
}

pub(crate) fn hit_nearest_line_series(
    pos: Offset,
    height: f64,
    category: usize,
    vals: &[Vec<f64>],
    axis: &ValueAxis,
    pad: EdgeInsets,
) -> Option<usize> {
    let bottom = (height - pad.bottom).max(pad.top + 1.0);
    let scale = axis.scale(pad.top, bottom);
    vals.iter()
        .enumerate()
        .filter_map(|(si, sv)| {
            let value = *sv.get(category)?;
            value
                .is_finite()
                .then_some((si, (pos.y - scale.map(value)).abs()))
        })
        .min_by(|(_, ad), (_, bd)| ad.partial_cmp(bd).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(si, _)| si)
}
