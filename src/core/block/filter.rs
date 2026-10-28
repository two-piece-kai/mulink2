use anyhow::{anyhow, Context};
use log::info;
use num::{bigint::Sign, Complex, Zero};
use rkyv::api::high;

use crate::{
    core::{
        block::{
            fft::{FftInst, RustFftInst},
            refragment::Refragmenter,
        },
        r#gen::{
            chirp::chirp_complex,
            fir::{fir_bpf, fir_hpf, fir_lpf},
        },
    },
    plot::spectrum::spectrogram,
    prelude::*,
};

// Overlap-add: https://en.wikipedia.org/wiki/Overlap%E2%80%93add_method
pub struct Filter<T: SignalType, FFT: FftInst<T> = RustFftInst<T>> {
    refrag: Refragmenter<T>,
    kernel_fft: Signal<T>,
    fft: FFT,
    len: usize,
    time_delay: usize,
    submitted: usize,
    step_size: usize,
    buffer: Signal<T>,
    overlap: Vec<Complex<T>>,
    sample_rate: f64,
}
impl<T: SignalType, FFT: FftInst<T>> Filter<T, FFT> {
    pub fn new(mut kernel: Signal<T>) -> anyhow::Result<Filter<T, FFT>> {
        let kern_len = kernel.len();
        let sample_rate = kernel.sample_rate;
        let len = 8 * 2_usize.pow((kern_len as u32 - 1).ilog2() + 1);
        let mut fft = FFT::new(len);

        let step_size = len - (kern_len - 1);

        let refrag = Refragmenter::<T>::new(kernel.sample_rate, step_size);

        let mut kernel = kernel.clone();
        kernel.append(&mut vec![Complex::zero(); len - kern_len]);
        fft.fft_fwd(&mut kernel)?;

        Ok(Filter {
            refrag,
            kernel_fft: kernel,
            fft,
            len,
            submitted: 0,
            step_size,
            buffer: Signal::from_vec(sample_rate, vec![Complex::zero(); len]),
            overlap: vec![Complex::zero(); len - step_size],
            sample_rate,
            time_delay: (len - step_size) / 2,
        })
    }
    fn process_chunk(&mut self, chunk: &[Complex<T>]) -> anyhow::Result<Signal<T>> {
        //trace!("chunk");
        self.buffer[0..self.step_size].clone_from_slice(chunk);
        self.buffer[self.step_size..].fill(Complex::zero());
        self.fft.fft_fwd(&mut self.buffer)?;
        let mut out = &self.buffer * &self.kernel_fft;
        self.fft.fft_rev(&mut out)?;
        self.overlap
            .iter()
            .enumerate()
            .for_each(|(idx, v)| out[idx] += *v);
        self.overlap = out.split_off(self.step_size);
        let scale_factor = T::one() / (T::from_usize_(self.fft.len()));

        out = &out * scale_factor;

        //trace!("chunk_dn");
        Ok(out)
    }
    pub fn process(&mut self, mut data: Signal<T>) -> Option<Signal<T>> {
        self.submitted += data.len();
        self.refrag.push(&mut data);

        // First fragment determines start time of `out`
        let mut output = Signal::new(self.kernel_fft.sample_rate);
        let Some(mut frag) = (&mut self.refrag).next() else {
            return None;
        };

        output.time = frag.time - self.time_delay as i64;
        let Ok(mut frag) = self.process_chunk(&mut frag) else {
            return None;
        };
        output.append(&mut frag);

        loop {
            let Some(mut frag) = (&mut self.refrag).next() else {
                break;
            };
            let Ok(mut frag) = self.process_chunk(&mut frag) else {
                break;
            };
            output.append(&mut frag);
        }

        //trace!("out_time: {}", output.time);

        Some(output)
    }
    pub fn finish(mut self) -> Option<Signal<T>> {
        //let mut output = Signal::<T>::new(self.kernel_fft.sample_rate);
        self.refrag.push(&mut Signal::from_vec(
            self.kernel_fft.sample_rate,
            vec![Complex::zero(); self.step_size],
        ));
        let mut output = Signal::new(self.kernel_fft.sample_rate);

        // First fragment determines start time of `out`
        let Some(mut frag) = (&mut self.refrag).next() else {
            return None;
        };

        output.time = frag.time - self.time_delay as i64;
        let Ok(mut frag) = self.process_chunk(&mut frag) else {
            return None;
        };
        output.append(&mut frag);

        loop {
            let Some(mut frag) = (&mut self.refrag).next() else {
                break;
            };
            let Ok(mut frag) = self.process_chunk(&mut frag) else {
                break;
            };
            output.append(&mut frag);
        }

        let finish_time: i64 = self.submitted as i64 + self.time_delay as i64;
        let trunc_output_len: usize = (finish_time - output.time) as usize;
        trace!("finish_time : trunc_output_len <==> {finish_time} : {trunc_output_len}");

        output.truncate(trunc_output_len);

        Some(output)
    }
    pub fn process_and_finish(mut self, data: Signal<T>) -> Option<Signal<T>> {
        let Some(mut filtered) = self.process(data) else {
            return None;
        };
        //trace!("out_len (pre-finish): {}", filtered.len());
        if let Some(mut sig) = self.finish() {
            filtered.append(&mut sig);
        }
        Some(filtered)
    }
    /// cutoff is real frequency
    pub fn lowpass(cutoff: f64, order: usize, sample_rate: f64) -> anyhow::Result<Filter<T, FFT>> {
        let kern = fir_lpf::<T>(cutoff / sample_rate * 2.0, order);
        let kern = Signal::from_vec(sample_rate, kern);

        Self::new(kern)
    }
    /// cutoff is real frequency
    pub fn highpass(cutoff: f64, order: usize, sample_rate: f64) -> anyhow::Result<Filter<T, FFT>> {
        let kern = fir_hpf::<T>(cutoff / sample_rate * 2.0, order);
        let kern = Signal::from_vec(sample_rate, kern);

        Self::new(kern)
    }
    /// cutoff is real frequency
    pub fn bandpass(
        low_cutoff: f64,
        high_cutoff: f64,
        order: usize,
        sample_rate: f64,
    ) -> anyhow::Result<Filter<T, FFT>> {
        let kern = fir_bpf::<T>(
            low_cutoff / sample_rate * 2.0,
            high_cutoff / sample_rate * 2.0,
            order,
        );
        let kern = Signal::from_vec(sample_rate, kern);

        Self::new(kern)
    }
}

pub enum ConvShape {
    FULL,
    SAME,
    VALID,
}
/// Convenience API
pub fn fftfilt<T: SignalType>(
    signal: &Signal<T>,
    filter: &Signal<T>,
    shape: ConvShape,
) -> anyhow::Result<Signal<T>> {
    let in_len = signal.len();
    let sample_rate = signal.sample_rate;
    let filt = Filter::<T>::new(filter.clone())?;
    let out = filt
        .process_and_finish(signal.clone())
        .context("filter did not return values")?;
    match shape {
        ConvShape::FULL => Ok(out),
        ConvShape::SAME => Ok(Signal::from_vec(
            sample_rate,
            out[out.time.abs() as usize..out.time.abs() as usize + in_len].to_vec(),
        )),
        ConvShape::VALID => Ok(Signal::from_vec(
            sample_rate,
            out[out.time.abs() as usize * 2..in_len].to_vec(),
        )),
    }
}

impl<T: SignalType> Signal<T> {
    pub fn fftfilt(&self, filter: &Signal<T>, shape: ConvShape) -> anyhow::Result<Signal<T>> {
        fftfilt(self, filter, shape)
    }
}

#[test]
fn test_filter() -> anyhow::Result<()> {
    init_tracing();
    info!("Unit test: test_filter");
    let mut lpf = Filter::<f64>::lowpass(48000.0, 300, 192000.0)?;
    let chirp = Signal::from_vec(192000.0, chirp_complex::<f64>(50000, 1.0, -1.0));
    trace!("chirp_len: {}", chirp.len());
    let filtered = lpf.process_and_finish(chirp.clone()).unwrap();
    trace!("out_time: {}", filtered.time);
    trace!("out_len: {}", filtered.len());

    spectrogram(
        "plot/test/test_filter/lpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );

    let mut hpf = Filter::<f64>::highpass(48000.0, 300, 192000.0)?;
    let filtered = hpf.process_and_finish(chirp.clone()).unwrap();
    spectrogram(
        "plot/test/test_filter/hpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );

    let mut bpf = Filter::<f64>::bandpass(24000.0, 72000.0, 300, 192000.0)?;
    let filtered = bpf.process_and_finish(chirp.clone()).unwrap();
    spectrogram(
        "plot/test/test_filter/bpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );

    let mut bpf = Signal::from_vec(192000.0, fir_bpf::<f64>(0.25, 0.75, 300));
    let filtered = fftfilt(&chirp, &bpf, ConvShape::FULL)?;
    trace!("len_full: {}", filtered.len());

    spectrogram(
        "plot/test/test_filter/fftfilt_full_bpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );
    let filtered = fftfilt(&chirp, &bpf, ConvShape::SAME)?;
    trace!("len_same: {}", filtered.len());

    spectrogram(
        "plot/test/test_filter/fftfilt_same_bpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );
    let filtered = fftfilt(&chirp, &bpf, ConvShape::VALID)?;
    trace!("len_valid: {}", filtered.len());

    spectrogram(
        "plot/test/test_filter/fftfilt_valid_bpf_spect.png",
        filtered,
        512,
        512 - 128,
        true,
        Some(-120.0),
    );
    Ok(())
}
