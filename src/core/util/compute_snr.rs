use std::cmp::Ordering;
use std::ops::Range;

use itertools::Itertools;
use num::Complex;
use crate::core::block::filter;
use crate::core::signal::{Amplitude, Power};
use crate::core::util::rms::{rms, rms_real};
use crate::plot::time::{plot, plot_complex};
use crate::{io::wav::read_wav_complex, logging::init_logging};

use crate::prelude::*;
use plotters::data::fitting_range;

fn max<T: SignalType>(a: &T,b: &T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Less)
}

fn max_idx<T:SignalType>(a: &(usize, T),b: &(usize, T)) -> Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Less)
}
fn min_idx<T:SignalType>(a: &(usize, T),b: &(usize, T)) -> Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Less).reverse()
}

/// Compute SNR (power)
pub fn compute_snr_narrow<T: SignalType>(signal: &[Complex<T>], noise_period: Range<usize>, signal_period: Range<usize>) -> Power {
    //let ir = impulse_response.abs();

    //let k = 0..12;
    
    // println!("min_snr:{:?} {:?} {}", T::one()/min_snr.linear(), min_snr, min_snr.linear::<f64>());
    // let min_snr = T::one()/min_snr.linear();
    // let max = ir.iter().max_by(max).unwrap();
    // let candidates= ir.iter().enumerate().flat_map( | (idx,x)| if *x > *max*min_snr && idx > min_required_spacing {
    //     Some(idx)
    // } else {None}).collect_vec();

    // let mut filtered_candidates = Vec::from([candidates[0]]);
    // candidates.windows(2).for_each(|x|if x[1]-x[0] > min_required_spacing {filtered_candidates.push(x[1]);});

    // let sorted_peaks = filtered_candidates.iter().map(|x| (*x, ir[*x])).sorted_by(min_idx).collect_vec();
    // Ok(sorted_peaks)
    Power::Linear(T::powi(rms(&signal[signal_period])/rms(&signal[noise_period]),2).as_())
}

pub fn compute_noise_floor<T: SignalType>(signal: &[Complex<T>], ignore_bottom_percent: f64, ignore_top_percent: f64) -> Power {
    let mut signal = signal.iter().map(|x| x.norm()).collect_vec();
    signal.sort_by(max);

    Power::Linear(T::powi(rms_real(&signal[((ignore_bottom_percent)*signal.len() as f64) as usize..((1.0-ignore_top_percent)*signal.len() as f64) as usize]),2).as_())
}

pub fn compute_snr<T: SignalType>(signal: &[Complex<T>], signal_period: Range<usize>) -> Power {
    let mut out_of_signal = Vec::new();
    for (idx, v) in signal.iter().enumerate() {
        if !signal_period.contains(&idx) {
            out_of_signal.push(*v);
        }
    }
    Power::Linear(T::powi(rms(&signal[signal_period])/rms(&out_of_signal),2).as_())
}


pub fn compute_snr_real<T: SignalType>(signal: &[T], signal_period: Range<usize>) -> Power {
    let mut out_of_signal = Vec::new();
    for (idx, v) in signal.iter().enumerate() {
        if !signal_period.contains(&idx) {
            out_of_signal.push(*v);
        }
    }
    Power::Linear(T::powi(rms_real(&signal[signal_period])/rms_real(&out_of_signal),2).as_())
}