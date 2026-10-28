use std::sync::{Arc, Mutex};

use itertools::Itertools;
use log::info;
use num::{complex::ComplexFloat, traits::ConstZero, Complex, Zero};
use rustfft::{Fft, FftPlanner};

use crate::{
    core::{
        signal::{FromFunction, RealSignal},
        stream::AudioStream,
    },
    plot::{self, time},
    prelude::*,
};

pub trait FftInst<T: SignalType> {
    fn new(len: usize) -> Self;
    fn len(&self) -> usize;
    fn fft_fwd(&self, signal: &mut [Complex<T>]) -> anyhow::Result<()>;
    fn fft_rev(&self, signal: &mut [Complex<T>]) -> anyhow::Result<()>;
}

pub struct RustFftInst<T: SignalType> {
    len: usize,
    scale_factor: T,
    scratch: Mutex<Vec<Complex<T>>>,
    fwd: Arc<dyn Fft<T>>,
    rev: Arc<dyn Fft<T>>,
}
impl<T: SignalType> RustFftInst<T> {}
impl<T: SignalType> FftInst<T> for RustFftInst<T> {
    fn new(len: usize) -> RustFftInst<T> {
        let mut planner = FftPlanner::<T>::new();
        let fwd = planner.plan_fft_forward(len);
        let rev = planner.plan_fft_inverse(len);
        let scale_factor = T::one() / (T::sqrt(T::from_usize_(len)));
        let scratch = Mutex::new(vec![Complex::zero(); fwd.get_inplace_scratch_len()]);

        RustFftInst {
            len,
            scale_factor,
            scratch,
            fwd,
            rev,
        }
    }
    fn len(&self) -> usize {
        self.len
    }
    /// Forward FFT. `signal.len` should be a multiple of `fft.len`
    fn fft_fwd(&self, signal: &mut [Complex<T>]) -> anyhow::Result<()> {
        let mut scratch = self.scratch.lock().unwrap();
        self.fwd.process_with_scratch(signal, scratch.as_mut());
        // signal.iter_mut().for_each(|x| *x=(*x)*self.scale_factor);
        Ok(())
    }
    /// Reverse FFT. `signal.len` should be a multiple of `fft.len`
    fn fft_rev(&self, signal: &mut [Complex<T>]) -> anyhow::Result<()> {
        let mut scratch = self.scratch.lock().unwrap();
        self.rev.process_with_scratch(signal, scratch.as_mut());
        // signal.iter_mut().for_each(|x| *x=(*x)*self.scale_factor);
        Ok(())
    }
}

impl<T: SignalType> Signal<T> {
    pub fn fft_fwd(mut self) -> anyhow::Result<Signal<T>> {
        RustFftInst::new(self.len()).fft_fwd(&mut self)?;
        Ok(self)
    }
    pub fn fft_rev(mut self) -> anyhow::Result<Signal<T>> {
        RustFftInst::new(self.len()).fft_rev(&mut self)?;
        Ok(self)
    }
}

pub fn fft_mag<T: SignalType>(signal: &Signal<T>) -> RealSignal<T> {
    let mut out = RealSignal::from_vec(signal.sample_rate, vec![T::zero(); signal.len() / 2]);
    for idx in 0..out.len() {
        out[idx] += signal[idx].norm();
        out[idx] += signal[signal.len() - 1 - idx].norm();
    }
    out
}

#[test]
fn test_fft() -> anyhow::Result<()> {
    init_tracing();
    info!("Unit test: test_fft");
    let sig = Signal::from_function(192000.0, 256, |x| {
        f32::sin(192000.0 / 4.0 * core::f32::consts::PI * 2.0 * x as f32)
    });

    time::plot_complex("plot/test/test_fft/sig.png", "Reference Signal", &sig);
    let sig_fft = sig.clone().fft_fwd()?;
    time::plot_complex("plot/test/test_fft/fft_fwd.png", "Forward FFT", &sig_fft);

    let sig_restored = sig_fft.fft_rev()?;

    let error = sig
        .iter()
        .zip(sig_restored.iter())
        .map(|(a, b)| (a - b).abs())
        .collect_vec();

    time::plot_complex("plot/test/test_fft/fft_rev.png", "Fwd->Rev", &sig_restored);

    trace!(
        "avg error: {}, max: {}",
        (error.iter().cloned().sum::<f32>()) / (error.len() as f32),
        error
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less))
            .unwrap_or(&0.0)
    );

    Ok(())
}
