#![allow(clippy::needless_return)]
/*
function WINDOW = ba_cosine_sum_window(width, a0)
    timebase = linspace(0, 2.0*pi, width);
    WINDOW = a0-(1-a0).*cos(timebase);
end
function WINDOW = ba_hamming(width)
    WINDOW = ba_cosine_sum_window(width, 0.54);
    %WINDOW = ba_cosine_sum_window(width, 0.53836);
end
*/

use crate::{plot::time::{plot, plot_complex}, prelude::*};
use std::f64::consts::PI;

use log::warn;
use num::Integer;

fn order_to_len_round_up(order: usize) -> usize {
    if order % 2 == 1 {
        warn!("filter order rounded up to {}", order+1);
        order +2
    } else {
        order +1
    }
}

pub fn cosine_sum<T: SignalType>(width: usize, a0: f64) -> Vec<T> {
    let mut window = vec![T::zero(); width];
    let a1 = T::from_f64_(1.0 - a0);
    let a0 = T::from_f64_(a0);
    let width_t = T::from_usize_(width);
    let freq = T::from_f64_ (PI*2.0);
    for idx in 0..width {
        window[idx] = a0 - a1 * T::cos(freq * (T::from_usize_(idx) / width_t));
    }
    window
}

pub fn hamming<T: SignalType>(width: usize) -> Vec<T> {
    cosine_sum(width, 0.54)
}

/*
function SINC = ba_normalized_sinc(width, npoints)
    timebase = linspace(-width/2.0, width/2.0, npoints);
    sinc_result = sin(pi.*timebase)./(pi.*timebase);
    sinc_result(ceil(npoints/2)) = 1.0;
    SINC = sinc_result;
end
*/
/*
pub fn sinc(position: f64) {
    if position == 0.0 {
        return 1.0;
    } else {
        return
    }
}*/

pub fn sinc<T: SignalType>(bandwidth: f64, points: usize) -> Vec<T> {
    let mut window = vec![T::zero(); points];
    let bandwidth = points as f64 * bandwidth;
    let negative_half_bandwidth = T::from_f64_(-bandwidth/2.0);
    let bandwidth = T::from_f64_(bandwidth);
    let scale_factor = bandwidth/T::from_usize_(points);
    let pi = T::from_f64_(PI);
    for n in 0..points {
        window[n] = (negative_half_bandwidth + (T::from_usize_(n) * scale_factor)) as T;
        //println!("{:#?}", window[n]);
        window[n] = T::sin(pi * window[n]) / (pi * window[n]);
        if !window[n].is_finite() {
            window[n] = T::one();
        }
    }
    //println!("{:#?}", window);

    /*
    if points.is_even() {
        for idx in 0..points {
            let pos = PI * (bandwidth / (points as f64) * idx as f64 - bandwidth / 2.0);
            window[idx] = f64::sin(pos) / (pos);
        }
    } else {
        for idx in 0..points {
            let pos = PI * (bandwidth / (points as f64) * idx as f64 - bandwidth / 2.0);
            window[idx] = f64::sin(pos) / (pos);
        }
        /*window[points / 2 + 1] = 1.0;
        for idx in (points / 2 + 2)..points {
            let pos = PI * (bandwidth / (points as f64) * idx as f64 - bandwidth / 2.0);
            window[idx] = f64::sin(pos) / (pos);
        }*/
    }*/
    window
}

/*
function FILTER = ba_fir_lpf(order, cutoff)
    width = order * cutoff;
    win = ba_hamming(order+1);
    sinc_result = cutoff.*ba_normalized_sinc(width, order+1);
    FILTER = sinc_result .* win;
end
function FILTER = ba_fir_bpf(order, low, high)
    width_low = order * low;
    width_high = order * high;
    win = ba_hamming(order+1);
    sinc_result = high.*ba_normalized_sinc(width_high, order+1)-low.*ba_normalized_sinc(width_low, order+1);
    FILTER = sinc_result .* win;
end */

pub fn fir_lpf<T: SignalType>(cutoff: f64, order: usize) -> Vec<T> {
    let filter_len = order_to_len_round_up(order);
    let window = hamming(filter_len);
    let mut filter = sinc(cutoff, filter_len);
    for idx in 0..filter_len {
        filter[idx] = filter[idx] * window[idx];
    }
    filter
}
pub fn fir_bpf<T: SignalType>(low_cutoff: f64, high_cutoff: f64, order: usize) -> Vec<T> {
    let filter_len = order_to_len_round_up(order);
    let window = hamming(filter_len);
    let filter_low = sinc(low_cutoff, filter_len);
    let filter_high = sinc(high_cutoff, filter_len);
    let mut filter = vec![T::zero(); filter_len];
    let (high_cutoff, low_cutoff) = (T::from_f64_(high_cutoff), T::from_f64_(low_cutoff));
    for idx in 0..filter_len {
        filter[idx] = (high_cutoff * filter_high[idx]
            - low_cutoff * filter_low[idx])
            * window[idx];
    }
    filter
}
/// Cutoff normalized to Nyquist
pub fn fir_hpf<T: SignalType>(cutoff: f64, order: usize) -> Vec<T> {
    return fir_bpf(cutoff, 1.0, order);
}

#[test]
fn test_fir() -> anyhow::Result<()> {
    let lpf = Signal::<f32>::from_vec(192000.0, fir_lpf::<f32>(0.5, 512));
    plot_complex("plot/test/test_fir/lpf.png", "lpf=0.5", &lpf);
    let lpf_fft = lpf.fft_fwd()?;
    plot("plot/test/test_fir/lpf_fft.png", "lpf=0.5 [fft]", &lpf_fft.abs());

    let hpf = Signal::<f32>::from_vec(192000.0, fir_hpf::<f32>(0.5, 512));
    plot_complex("plot/test/test_fir/hpf.png", "hpf=0.5", &hpf);
    let hpf_fft = hpf.fft_fwd()?;
    plot("plot/test/test_fir/hpf_fft.png", "hpf=0.5 [fft]", &hpf_fft.abs());

    let bpf = Signal::<f32>::from_vec(192000.0, fir_bpf::<f32>(0.25, 0.75, 512));
    plot_complex("plot/test/test_fir/bpf.png", "bpf=[0.25,0.75]", &bpf);
    let bpf_fft = bpf.fft_fwd()?;
    plot("plot/test/test_fir/bpf_fft.png", "bpf=[0.25,0.75] [fft]", &bpf_fft.abs());
    

    Ok(())
}