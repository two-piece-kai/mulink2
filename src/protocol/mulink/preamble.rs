use std::ops::Deref;

use fmtastic::Subscript;
use itertools::Itertools;
use log::trace;

use crate::{
    core::{
        signal::Amplitude,
        util::{compute_snr::compute_snr_narrow, find_direct_path::find_direct_path},
    },
    prelude::{Signal, SignalType},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChirpDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug)]
pub struct PreambleDetection {
    pub direction: ChirpDirection,
    pub detection: ControlDetection,
}
impl Deref for PreambleDetection {
    type Target = ControlDetection;

    fn deref(&self) -> &Self::Target {
        &self.detection
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ControlDetection {
    pub amplitude: f64,
    pub snr: f64,
    pub time: f64,
    pub idx: i64,
    pub doppler_ratio: f64,
}

pub const SNR_SIGNAL_LEN_S: f64 = 0.000625; 
pub const DEFAULT_CHIRP_LEN_S: f64 = 0.020;

pub const GAP_TOL_S: f64 = 0.002;

pub fn preamble_detect_kern<T: SignalType>(
    impulse_response: &Signal<T>,
    min_peak_lvl: Amplitude,
    min_required_spacing: usize,
) -> anyhow::Result<ControlDetection> {
    preamble_detect_kern_impl(
        impulse_response,
        min_peak_lvl,
        min_required_spacing,
        0.120,
        DEFAULT_CHIRP_LEN_S,
        None,
    )
}

/// Same as `preamble_detect_kern`, but with no hard code thnx
pub fn preamble_detect_kern_timed<T: SignalType>(
    impulse_response: &Signal<T>,
    min_peak_lvl: Amplitude,
    min_required_spacing: usize,
    chirp_period: f64,
    chirp_len: f64,
) -> anyhow::Result<ControlDetection> {
    preamble_detect_kern_impl(
        impulse_response,
        min_peak_lvl,
        min_required_spacing,
        chirp_period,
        chirp_len,
        Some(GAP_TOL_S),
    )
}

fn preamble_detect_kern_impl<T: SignalType>(
    impulse_response: &Signal<T>,
    min_peak_lvl: Amplitude,
    min_required_spacing: usize,
    chirp_period: f64,
    chirp_len: f64,
    gap_tol: Option<f64>,
) -> anyhow::Result<ControlDetection> {
    let fs = impulse_response.sample_rate;
    let sig_len = ((SNR_SIGNAL_LEN_S * fs).round() as usize).max(1);
    let exclude = (chirp_len * fs).ceil() as usize;
    let expected_span = 2.0 * chirp_period; // three peaks -> two gaps
    let peaks = find_direct_path(impulse_response, min_peak_lvl, min_required_spacing)?;

    if peaks.len() < 3 {
        // trace!("Fail: not enough peaks");
        return Err(anyhow::anyhow!("Did not find adequate number of peaks"));
    }

    let peak_idx = peaks[0..3].iter().map(|(i, _)| *i).collect_vec();
    let silence = ((chirp_period - chirp_len).max(0.0) * fs).floor() as usize;
    let noise_end = peak_idx.iter().max().copied().unwrap_or(0) + silence;
    let mut noise_pow = impulse_response
        .iter()
        .enumerate()
        .take(noise_end)
        .filter(|(i, _)| peak_idx.iter().all(|p| i.abs_diff(*p) > exclude))
        .map(|(_, v)| v.norm_sqr().as_f64())
        .collect_vec();
    if noise_pow.len() < 16 {
        return Err(anyhow::anyhow!(
            "window too short for a noise estimate ({} samples outside the chirps)",
            noise_pow.len()
        ));
    }
    let mid = noise_pow.len() / 2;
    let (_, median, _) = noise_pow.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
    let noise_mean_pow = (*median / std::f64::consts::LN_2).max(f64::MIN_POSITIVE);

    let top_peaks = peaks[0..3]
        .iter()
        .cloned()
        .sorted_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(idx, mag)| {
            let sig = &impulse_response[idx..(idx + sig_len).min(impulse_response.len())];
            let sig_pow =
                sig.iter().map(|v| v.norm_sqr().as_f64()).sum::<f64>() / sig.len() as f64;
            (
                idx,
                idx as f64 / impulse_response.sample_rate,
                10.0 * (sig_pow / noise_mean_pow).log10(),
                mag,
            )
        })
        .collect_vec();

    // println!("Top 3 peaks: ");
    // for (idx, (peak_idx, peak_time, peak_snr, peak_amplitude)) in top_peaks.iter().enumerate() {
    //     println!("Peak #{idx}@idx={peak_idx} ({peak_time:.3}s) SNR: {peak_snr:.3} dB, amplitude: {peak_amplitude:.4}")
    // }

    let ampl = top_peaks
        .iter()
        .fold(0.0, |acc, (_, _, _, peak_amplitude_0)| {
            acc + peak_amplitude_0.as_f64()
        })
        / 3.0;
    let snr = top_peaks
        .iter()
        .fold(0.0, |acc, (_, _, peak_snr_0, _)| acc + peak_snr_0)
        / 3.0;
    // println!("DTAnalysis: ");
    // for (
    //     (idx_0, (peak_idx_0, peak_time_0, _peak_snr_0, _peak_amplitude_0)),
    //     (_idx_1, (peak_idx_1, peak_time_1, _peak_snr_1, _peak_amplitude_1)),
    // ) in top_peaks.iter().enumerate().tuple_windows::<(_, _)>()
    // {
    //     println!(
    //         "Δt{}={:.3}s | Δidx{}={}",
    //         Subscript(idx_0),
    //         peak_time_1 - peak_time_0,
    //         Subscript(idx_0),
    //         peak_idx_1 - peak_idx_0
    //     );
    // }

    let dt0 = top_peaks[1].1 - top_peaks[0].1;
    let dt1 = top_peaks[2].1 - top_peaks[1].1;
    //let dt2 = top_peaks[3].1 - top_peaks[2].1;

    // total preamble doppler +/- 5% && preamble trenches within 2% of each other && message length > 0.0ms
    let envelope_valid = match gap_tol {
        // Timed API: each gap within +/- gap_tol of the expected chirp period.
        Some(tol) => (dt0 - chirp_period).abs() <= tol && (dt1 - chirp_period).abs() <= tol,
        // Original API (rx_simple, demod_wav_hifiberry): unchanged.
        None => (1.0 - (dt0 + dt1) / expected_span).abs() < 0.05 && (dt0 - dt1).abs() < 0.02,
    };
    //&& (dt2 - dt1).abs() > 0.200;
    let doppler_ratio = (dt0 + dt1) / expected_span;

    // println!("doppler_ratio: {doppler_ratio}");

    // println!("envelope valid: {envelope_valid}");

    if !envelope_valid {
        return Err(anyhow::anyhow!("Invalid envelope"));
    }

    //println!("")

    Ok(ControlDetection {
        idx: top_peaks[0].0 as i64 + impulse_response.time,
        time: top_peaks[0].1 + (impulse_response.time as f64 / impulse_response.sample_rate),
        snr,
        amplitude: ampl,
        doppler_ratio,
    })
}
