use anyhow::Context;
use itertools::Itertools;
use log::trace;
use num::Complex;
use rand::Rng;

use crate::{core::{r#gen::chirp::chirp_complex, signal::Amplitude}, plot::{spectrum::spectrogram, time::{plot, plot_complex}}, prelude::{init_tracing, FromVec, Signal, SignalType}};

/// len = len(samples) & level = noise level (in dB i.e. 20log10(linear amplitude))
pub fn noise_complex<T: SignalType>(len: usize, lvl: f64, sample_rate: f64) -> anyhow::Result<Signal<T>> {
    let mut rng = rand::rng();

    let mut mag = vec![T::zero(); len];
    let mut phase = vec![T::zero(); len];

    let rng_mag = rand::distr::Uniform::new_inclusive(T::zero(), T::from_f64_(f64::powf(10.0, lvl/20.0)))?;
    let rng_phase = rand::distr::Uniform::new(T::zero(), T::from_f64_(core::f64::consts::FRAC_2_PI))?;

    mag.fill_with(|| rng.sample(&rng_mag)); 
    phase.fill_with(|| rng.sample(&rng_phase));

    Ok(Signal::from_vec(sample_rate,mag.iter().zip(phase.iter()).map(|(a,b)| Complex::from_polar(*a, *b)).collect_vec()))
} 

// Todo: I think we could make a monadic representation of Linear/Log amplitudes

#[test]
fn test_noise() -> anyhow::Result<()> {
    // fwd complex
    init_tracing();
    trace!("6dB = {}", Amplitude::Log(6.0_f64).linear::<f64>());
    let mut chirp = Signal::from_vec(192000.0, chirp_complex::<f64>(4096, -1.0, 1.0));
    let noise = noise_complex(chirp.len(), -6.0, 192000.0)?;
    chirp += noise;
    spectrogram("plot/test/test_noise/noisy_chirp_fwd_spect.png", chirp.clone(), 512, 256+128+64+32+16, true, Some(-120.0));
    plot_complex("plot/test/test_noise/noisy_chirp_fwd.png", "Chirp [-0.5, 0.5]", &chirp);
    let chirp = chirp.fft_fwd()?;
    plot("plot/test/test_noise/noisy_chirp_fwd_fft.png", "Chirp [-0.5, 0.5] [fft]", &chirp.abs());
    Ok(())
}