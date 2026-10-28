use std::ops::{
    Add, AddAssign, Deref, DerefMut, DivAssign, MulAssign, Range, RemAssign, SubAssign,
};

use anyhow::Result;
use hound::SampleFormat;
use itertools::Itertools;
use log::trace;
use num::{cast::AsPrimitive, Complex, Float, FromPrimitive, Signed, ToPrimitive};
use rand::distr::uniform::{SampleBorrow, SampleUniform};
use serde::{Deserialize, Serialize};

use crate::{
    io::wav::{read_wav_complex, read_wav_real, write_wav_complex, write_wav_real},
    prelude::init_tracing,
};

#[derive(Clone, Copy, Debug)]
pub enum Amplitude {
    Linear(f64),
    Log(f64),
}
impl Amplitude {
    pub fn linear<T: SignalType>(&self) -> T {
        T::from_f64_(match self {
            Amplitude::Linear(a) => *a,
            Amplitude::Log(a) => 10.0_f64.powf(*a / 20.0),
        })
    }
    pub fn log<T: SignalType>(&self) -> T {
        T::from_f64_(match self {
            Amplitude::Linear(a) => {
                let a = 20.0 * a.log10();
                if a.is_finite() {
                    a
                } else {
                    -300.0
                }
            }
            Amplitude::Log(a) => *a,
        })
    }
    pub fn to_power(&self) -> Power {
        Power::Linear(self.linear())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Power {
    Linear(f64),
    Log(f64),
}
impl Power {
    pub fn linear<T: SignalType>(&self) -> T {
        T::from_f64_(match self {
            Power::Linear(a) => *a,
            Power::Log(a) => 10.0_f64.powf(*a / 10.0),
        })
    }
    pub fn log<T: SignalType>(&self) -> T {
        T::from_f64_(match self {
            Power::Linear(a) => {
                let a = 10.0 * a.log10();
                if a.is_finite() {
                    a
                } else {
                    -300.0
                }
            }
            Power::Log(a) => *a,
        })
    }
    pub fn to_amplitude(&self) -> Amplitude {
        Amplitude::Linear(self.linear())
    }
}

#[test]
fn test_power_amplitude() -> anyhow::Result<()> {
    init_tracing();
    let ampl = Amplitude::Log(6.0);
    let pwr = Power::Log(6.0);
    trace!("6 dB ampl = {}", ampl.linear::<f64>());
    trace!("6 dB ampl = {} dB pwr", ampl.to_power().log::<f64>());
    trace!("6 dB pwr = {} dB ampl", pwr.to_amplitude().log::<f64>());
    Ok(())
}

pub trait SignalType:
    for<'a> Deserialize<'a>
    + From<f32>
    + std::iter::Sum
    + FromPrimitive
    + Serialize
    + num::Float
    + std::fmt::Debug
    + std::fmt::Display
    + Signed
    + Send
    + Sync
    + AsPrimitive<i32>
    + AsPrimitive<i64>
    + AsPrimitive<f64>
    + AsPrimitive<f32>
    + SampleBorrow<Self>
    + SampleUniform
    + Default
    + AddAssign
    + SubAssign
    + DivAssign
    + MulAssign
    + RemAssign
    + 'static
{
    fn from_f32_(v: f32) -> Self;
    fn from_f64_(v: f64) -> Self;
    fn from_usize_(v: usize) -> Self;
    fn from_i64_(v: i64) -> Self;
    fn from_i32_(v: i32) -> Self;
    fn from_u64_(v: u64) -> Self;
    fn from_u32_(v: u32) -> Self;
    fn as_f32(&self) -> f32 {
        self.as_()
    }
    fn as_f64(&self) -> f64 {
        self.as_()
    }
}
impl SignalType for f32 {
    fn from_f32_(v: f32) -> Self {
        v as f32
    }
    fn from_f64_(v: f64) -> Self {
        v as f32
    }
    fn from_usize_(v: usize) -> Self {
        v as f32
    }
    fn from_i64_(v: i64) -> Self {
        v as f32
    }
    fn from_i32_(v: i32) -> Self {
        v as f32
    }
    fn from_u64_(v: u64) -> Self {
        v as f32
    }
    fn from_u32_(v: u32) -> Self {
        v as f32
    }
}
impl SignalType for f64 {
    fn from_f32_(v: f32) -> Self {
        v as f64
    }
    fn from_f64_(v: f64) -> Self {
        v
    }
    fn from_usize_(v: usize) -> Self {
        v as f64
    }
    fn from_i64_(v: i64) -> Self {
        v as f64
    }
    fn from_i32_(v: i32) -> Self {
        v as f64
    }
    fn from_u64_(v: u64) -> Self {
        v as _
    }
    fn from_u32_(v: u32) -> Self {
        v as _
    }
}

impl<T: SignalType> Into<Signal<T>> for RealSignal<T> {
    fn into(self) -> Signal<T> {
        self.complex()
    }
}

pub trait FromVec<T, U> {
    fn from_vec(sample_rate: f64, samples: Vec<T>) -> Signal<U>;
}

impl<T: SignalType> FromVec<Complex<T>, T> for Signal<T> {
    fn from_vec(sample_rate: f64, samples: Vec<Complex<T>>) -> Signal<T> {
        Signal {
            time: 0,
            sample_rate,
            samples,
        }
    }
}
impl<T: SignalType> FromVec<T, T> for Signal<T> {
    fn from_vec(sample_rate: f64, samples: Vec<T>) -> Signal<T> {
        Signal {
            time: 0,
            sample_rate,
            samples: samples
                .iter()
                .map(|x| Complex::new(*x, T::zero()))
                .collect_vec(),
        }
    }
}

pub trait FromFunction<T, U> {
    fn from_function(sample_rate: f64, len: usize, func: impl Fn(f64) -> T) -> Signal<U>;
}
impl<T: SignalType> FromFunction<Complex<T>, T> for Signal<T> {
    fn from_function(sample_rate: f64, len: usize, func: impl Fn(f64) -> Complex<T>) -> Signal<T> {
        Signal {
            time: 0,
            sample_rate,
            samples: (0..len)
                .into_iter()
                .map(|x| func((x as f64) / sample_rate))
                .collect_vec(),
        }
    }
}
impl<T: SignalType> FromFunction<T, T> for Signal<T> {
    fn from_function(sample_rate: f64, len: usize, func: impl Fn(f64) -> T) -> Signal<T> {
        Self::from_function(sample_rate, len, |x| Complex {
            re: func(x),
            im: T::zero(),
        })
    }
}

#[derive(Clone)]
pub struct RealSignal<T: SignalType> {
    pub sample_rate: f64,
    pub time: i64,
    samples: Vec<T>,
}
impl<T: SignalType> RealSignal<T> {
    pub fn new(sample_rate: f64) -> RealSignal<T> {
        RealSignal {
            time: 0,
            sample_rate,
            samples: Vec::new(),
        }
    }
    pub fn write_wav(
        &self,
        path: &str,
        sample_format: SampleFormat,
        normalize: bool,
    ) -> Result<()> {
        // write_wav_real(path, self, sample_format, normalize)
        write_wav_real(path, self, sample_format, normalize)
    }
    pub fn read_wav(path: &str) -> Result<RealSignal<T>> {
        // read_wav_real(path)
        read_wav_real(path)
    }
    pub fn from_function(sample_rate: f64, len: usize, func: impl Fn(f64) -> T) -> RealSignal<T> {
        RealSignal {
            time: 0,
            sample_rate,
            samples: (0..len)
                .into_iter()
                .map(|x| func((x as f64) / sample_rate))
                .collect_vec(),
        }
    }
    pub fn from_vec(sample_rate: f64, samples: Vec<T>) -> RealSignal<T> {
        RealSignal {
            time: 0,
            sample_rate,
            samples,
        }
    }
    pub fn complex(&self) -> Signal<T> {
        let mut signal = Signal::from_vec(self.sample_rate, self.samples.clone());
        signal.time = self.time;
        signal
    }
    pub fn subslice(&self, range: Range<usize>) -> RealSignal<T> {
        let start = range.start;
        let mut sig = RealSignal::from_vec(self.sample_rate, self[range].to_vec());
        sig.time += start as i64;
        sig
    }
}

impl<T: SignalType> Deref for RealSignal<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.samples
    }
}
impl<T: SignalType> DerefMut for RealSignal<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.samples
    }
}

#[derive(Clone)]
pub struct Signal<T> {
    pub sample_rate: f64,
    pub time: i64,
    samples: Vec<Complex<T>>,
}

impl<T: SignalType> Signal<T> {
    pub fn new(sample_rate: f64) -> Signal<T> {
        Signal {
            time: 0,
            sample_rate,
            samples: Vec::new(),
        }
    }
    pub fn write_wav(
        &self,
        path: &str,
        sample_format: SampleFormat,
        normalize: bool,
    ) -> Result<()> {
        // write_wav_real(path, self, sample_format, normalize)
        write_wav_complex(path, self, sample_format, normalize)
    }
    pub fn read_wav(path: &str) -> Result<Signal<T>> {
        // read_wav_real(path)
        read_wav_complex(path)
    }
    pub fn real(&self) -> RealSignal<T> {
        RealSignal {
            sample_rate: self.sample_rate,
            time: self.time,
            samples: self.re(),
        }
    }
    pub fn subslice(&self, range: Range<usize>) -> Signal<T> {
        let start = range.start;
        let mut sig = Signal::from_vec(self.sample_rate, self[range].to_vec());
        sig.time += start as i64;
        sig
    }
}
impl<T> Deref for Signal<T> {
    type Target = Vec<Complex<T>>;

    fn deref(&self) -> &Self::Target {
        &self.samples
    }
}
impl<T> DerefMut for Signal<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.samples
    }
}
