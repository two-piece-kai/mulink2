use anyhow::Context;
use num::{cast::AsPrimitive, Complex, Float};

use crate::{
    core::signal::RealSignal,
    plot::{
        spectrum::spectrogram,
        time::{plot, plot_complex},
    },
    prelude::{FromVec, Signal, SignalType},
};

pub fn euler<T: SignalType>(x: T) -> Complex<T> {
    // let x = T::from(x).unwrap_or_default();
    Complex {
        re: T::cos(x),
        im: T::sin(x),
    }
}
/// f_start f_end normalized to nyquist
pub fn chirp_complex<T: SignalType>(
    len: usize,
    f_start: impl Into<T>,
    f_end: impl Into<T>,
) -> Vec<Complex<T>> {
    let (f_start, f_end) = (
        f_start.into() / T::from_f32_(2.0),
        f_end.into() / T::from_f32_(2.0),
    );
    let bw = f_end - f_start;
    let len_t = T::from_usize_(len);
    let pi = T::from_f64_(core::f64::consts::PI);

    let offset_samples: i64 = ((f_end.abs() - f_start.abs()) * len_t).as_();

    (0..len)
        .map(|idx| {
            euler::<T>(
                (pi * bw
                    * T::powi(
                        T::from_i64_(idx as i64 - (len as i64 / 2) + offset_samples),
                        2,
                    ))
                    / (len_t),
            )
        })
        .collect()
}

pub fn chirp_real<T: SignalType>(len: usize, f_start: f64, f_end: f64) -> Vec<T> {
    //let mut signal = vec![0.0_f64; len];
    //let f_step: f64 = (f_start - f_end) / len as f64;
    let (f_start, f_end): (T, T) = (T::from_f64_(f_start / 2.0), T::from_f64_(f_end / 2.0));
    let c: T = (f_end - f_start) / (T::from_usize_(len)) / T::from_f64_(2.0);

    // let c: T = T::from(c);
    let pi = T::from_f64_(core::f64::consts::PI * 2.0);
    // let two = T::from_f64(2.0);//.context("Failed conversion")?;

    (0..len)
        .map(|idx| {
            let idx: T = T::from_usize_(idx);
            T::sin(pi * (c * T::powi(idx, 2) + f_start * (idx)))
        })
        .collect()
}

impl<T: SignalType> Signal<T> {
    pub fn chirp_complex(sample_rate: f64, len: usize, f_start: f64, f_end: f64) -> Signal<T> {
        Signal::from_vec(
            sample_rate,
            chirp_complex(
                len,
                T::from_f64_(f_start / sample_rate * 2.0),
                T::from_f64_(f_end / sample_rate * 2.0),
            ),
        )
    }
    pub fn chirp_real(sample_rate: f64, len: usize, f_start: f64, f_end: f64) -> RealSignal<T> {
        RealSignal::from_vec(
            sample_rate,
            chirp_real(len, f_start / sample_rate * 2.0, f_end / sample_rate * 2.0),
        )
    }
}

impl<T: SignalType> RealSignal<T> {
    pub fn chirp_real(sample_rate: f64, len: usize, f_start: f64, f_end: f64) -> RealSignal<T> {
        RealSignal::from_vec(
            sample_rate,
            chirp_real(len, f_start / sample_rate * 2.0, f_end / sample_rate * 2.0),
        )
    }
}
#[test]
fn test_chirp() -> anyhow::Result<()> {
    // fwd complex
    let chirp = Signal::from_vec(192000.0, chirp_complex::<f64>(2048, -0.5, 0.5));
    spectrogram(
        "plot/test/test_chirp/chirp_fwd_spect.png",
        chirp.clone(),
        256,
        128 + 64 + 32 + 16,
        true,
        Some(-40.0),
    );
    plot_complex(
        "plot/test/test_chirp/chirp_fwd.png",
        "Chirp [-0.5, 0.5]",
        &chirp,
    );
    let chirp = chirp.fft_fwd()?;
    plot(
        "plot/test/test_chirp/chirp_fwd_fft.png",
        "Chirp [-0.5, 0.5] [fft]",
        &chirp.abs(),
    );
    // rev complex
    let chirp = Signal::from_vec(192000.0, chirp_complex::<f64>(2048, 0.5, -0.5));
    spectrogram(
        "plot/test/test_chirp/chirp_rev_spect.png",
        chirp.clone(),
        256,
        128 + 64 + 32 + 16,
        true,
        Some(-40.0),
    );
    plot_complex(
        "plot/test/test_chirp/chirp_rev.png",
        "Chirp [0.5, -0.5]",
        &chirp,
    );
    let chirp = chirp.fft_fwd()?;
    plot(
        "plot/test/test_chirp/chirp_rev_fft.png",
        "Chirp [0.5, -0.5] [fft]",
        &chirp.abs(),
    );

    // fwd small
    let chirp = Signal::from_vec(192000.0, chirp_complex::<f64>(2048, -0.01, 0.01));
    //spectrogram("plot/test/test_chirp/chirp_fwd_spect.png", chirp.clone(), 256, 128+64+32+16, true, Some(-40.0));
    plot_complex(
        "plot/test/test_chirp/chirp_fwd_small.png",
        "Chirp [-0.1, 0.1]",
        &chirp,
    );

    // fwd real
    let chirp = Signal::from_vec(192000.0, chirp_real::<f64>(2048, 0.0, 0.01));
    //spectrogram("plot/test/test_chirp/chirp_fwd_spect.png", chirp.clone(), 256, 128+64+32+16, true, Some(-40.0));
    plot_complex(
        "plot/test/test_chirp/real_chirp_fwd_small.png",
        "Chirp [-0.1, 0.1]",
        &chirp,
    );
    Ok(())
}
