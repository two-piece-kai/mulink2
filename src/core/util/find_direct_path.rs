use std::cmp::Ordering;

use crate::core::block::filter;
use crate::core::signal::Amplitude;
use crate::plot::time::{plot, plot_complex};
use crate::{io::wav::read_wav_complex, logging::init_logging};
use itertools::Itertools;

use crate::prelude::*;
use plotters::data::fitting_range;

fn max<T: SignalType>(a: &&T, b: &&T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Less)
}
fn min<T: SignalType>(a: &&T, b: &&T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Greater).reverse()
}

fn max_idx<T: SignalType>(a: &(usize, T), b: &(usize, T)) -> Ordering {
    a.0.partial_cmp(&b.0).unwrap_or(Ordering::Less)
}
fn min_idx<T: SignalType>(a: &(usize, T), b: &(usize, T)) -> Ordering {
    a.0.partial_cmp(&b.0).unwrap_or(Ordering::Less).reverse()
}

/// Given an impulse response (or other similar matched filter result) tries to find the direct paths.
///
/// # Arguments:
/// * `impulse_response`: The complex or real impulse response signal
/// * `min_peak_lvl`: Minimum required value as compared to the maximum value within the signal. If max(impulse_response) = 1.0, min_peak_lvl = -6 dB, then only peaks above 0.5 will be considered viable candidates
/// * `min_required_spacing`: The guard interval between subsequent detections. Used to reject cases where the direct path is not the strongest path. Length should be, at minimum, the filter length
///
/// # Returns:
/// A vector of all candidate peaks in the form of (index: usize, amplitude: T), sorted from greatest amplitude to least
pub fn find_direct_path<T: SignalType>(
    impulse_response: &Signal<T>,
    min_peak_lvl: Amplitude,
    min_required_spacing: usize,
) -> anyhow::Result<Vec<(usize, T)>> {
    let ir = impulse_response.abs();

    //println!("min_snr:{:?} {:?} {}", min_peak_lvl.linear::<f32>(), min_peak_lvl, min_peak_lvl.linear::<f64>());

    let search_end_idx = impulse_response.len() - min_required_spacing;
    let min_snr = min_peak_lvl.linear();
    let max_amp = ir.iter().max_by(max).unwrap();
    let candidates = ir
        .iter()
        .enumerate()
        .flat_map(|(idx, x)| {
            if *x > *max_amp * min_snr && idx > min_required_spacing && idx < search_end_idx {
                Some(idx)
            } else {
                None
            }
        })
        .collect_vec();

    if candidates.is_empty() {
        return Err(anyhow::anyhow!("not enough peaks"));
    }

    let mut filtered_candidates = Vec::from([candidates[0]]);
    candidates.windows(2).for_each(|x| {
        if x[1] - x[0] > min_required_spacing {
            filtered_candidates.push(x[1]);
        }
    });

    let sorted_peaks = filtered_candidates
        .iter()
        .map(|x| {
            (
                *x,
                *ir[*x..*x + min_required_spacing]
                    .iter()
                    .max_by(max)
                    .unwrap_or(&ir[*x]),
            )
        })
        .sorted_by(|(_, a), (_, b)| min(&a, &b))
        .collect_vec();
    Ok(sorted_peaks)
}
