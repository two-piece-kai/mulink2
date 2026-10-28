use std::cmp::Ordering;
use std::ops::Range;

use itertools::Itertools;
use num::Complex;
use crate::core::block::filter;
use crate::core::signal::Amplitude;
use crate::plot::time::{plot, plot_complex};
use crate::{io::wav::read_wav_complex, logging::init_logging};

use crate::prelude::*;
use plotters::data::fitting_range;

fn max<T: SignalType>(a: &&T,b: &&T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Less)
}

fn max_idx<T:SignalType>(a: &(usize, T),b: &(usize, T)) -> Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Less)
}
fn min_idx<T:SignalType>(a: &(usize, T),b: &(usize, T)) -> Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Less).reverse()
}


pub fn rms<T: SignalType>(signal: &[Complex<T>]) -> T {
    T::sqrt(signal.iter().map(|x| x.norm_sqr()).sum::<T>()/(T::from_usize_(signal.len())))
}
pub fn rms_real<T: SignalType>(signal: &[T]) -> T {
    T::sqrt(signal.iter().map(|x| x.powi(2)).sum::<T>()/(T::from_usize_(signal.len())))
}