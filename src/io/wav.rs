use std::{fs::File, i32, io::BufReader};

use anyhow::{anyhow, Result};
use hound::SampleFormat;
use log::{error, warn};
use num::{cast::AsPrimitive, Complex, FromPrimitive};

use crate::{core::{r#gen::chirp::{chirp_complex, chirp_real}, signal::RealSignal}, plot::spectrum::spectrogram, prelude::*};



pub fn read_wav_complex<T: SignalType>(path: &str) -> Result<Signal<T>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let len = reader.len() as usize;

    let max = T::from_i32_(i32::MAX);
    let signal: Vec<Complex<T>> =
        match spec.sample_format {
            hound::SampleFormat::Float => match spec.channels {
                1 => {
                    let samples: anyhow::Result<Vec<Complex<T>>> = reader
                        .samples::<f32>()
                        .try_fold(Vec::<Complex<T>>::with_capacity(len), |mut acc, sample| {
                            acc.push(Complex {
                                re: T::from_f32_(sample?),
                                im: T::from_f32_(0.0),
                            });
                            Ok(acc)
                        });
                    samples?
                }
                2 => {
                    let samples: anyhow::Result<Vec<f32>> = reader.samples::<f32>().try_fold(
                        Vec::<f32>::with_capacity(len),
                        |mut acc, sample| {
                            acc.push(sample?);
                            Ok(acc)
                        },
                    );
                    let samples: Vec<Complex<T>> = samples?.chunks_exact(2).fold(
                        Vec::<Complex<T>>::with_capacity(len),
                        |mut acc, sample| {
                            acc.push(Complex {
                                re: T::from_f32_(sample[0]),
                                im: T::from_f32_(sample[1]),
                            });
                            acc
                        },
                    );
                    samples
                }
                _ => {
                    return Err(anyhow::format_err!(
                        "Unsupported channels count: {}",
                        spec.channels
                    ))
                }
            },
            hound::SampleFormat::Int => match spec.channels {
                1 => {
                    let samples: anyhow::Result<Vec<Complex<T>>> = reader
                        .samples::<i32>()
                        .try_fold(Vec::<Complex<T>>::with_capacity(len), |mut acc, sample| {
                            acc.push(Complex {
                                re: T::from_i32_(sample?)/max,
                                im: T::from_i32_(0),
                            });
                            Ok(acc)
                        });
                    samples?
                }
                2 => {
                    let samples: anyhow::Result<Vec<i32>> = reader.samples::<i32>().try_fold(
                        Vec::<i32>::with_capacity(len),
                        |mut acc, sample| {
                            acc.push(sample?);
                            Ok(acc)
                        },
                    );
                    let samples: Vec<Complex<T>> = samples?.chunks_exact(2).fold(
                        Vec::<Complex<T>>::with_capacity(len),
                        |mut acc, sample| {
                            acc.push(Complex {
                                re: T::from_i32_(sample[0])/max,
                                im: T::from_i32_(sample[1])/max,
                            });
                            acc
                        },
                    );
                    samples
                }
                _ => {
                    return Err(anyhow::format_err!(
                        "Unsupported channels count: {}",
                        spec.channels
                    ))
                }
            },
        };
    Ok(Signal::from_vec(spec.sample_rate as f64, signal))
}

pub fn write_wav_complex<T: SignalType>(
    path: &str,
    signal: &Signal<T>,
    sample_format: SampleFormat,
    normalize: bool,
) -> Result<()> {
    // add some error checks here for Int signals of unusually low precision. (and warnings) as well as out of bounds float signals
    let mut signal = signal.clone();
    if normalize {
        signal.normalize();
    }

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: signal.sample_rate as u32,
        bits_per_sample: 32,
        sample_format: sample_format.into(),
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    let max = T::from_i32_(i32::MAX);
    let result: Result<()> = match sample_format {
        SampleFormat::Float => signal.iter().try_for_each(|x| {
            writer.write_sample::<f32>(x.re.as_())?;
            writer.write_sample::<f32>(x.im.as_())?;
            Ok(())
        }),
        SampleFormat::Int => signal.iter().try_for_each(|x| {
            writer.write_sample::<i32>(T::clamp(x.re*max, -max, max).as_())?;
            writer.write_sample::<i32>(T::clamp(x.im*max, -max, max).as_())?;
            Ok(())
        }),
    };
    result?;
    Ok(())
}

pub fn read_wav_real<T: SignalType>(path: &str) -> Result<RealSignal<T>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let len = reader.len() as usize;

    if spec.channels != 1 {
        error!("Channels count {} != 1", spec.channels);
        return Err(anyhow!("Channels count {} != 1", spec.channels))
    }

    let signal: Vec<T> =
        match spec.sample_format {
            hound::SampleFormat::Float => match spec.channels {
                1 => {
                    let samples: anyhow::Result<Vec<T>> = reader
                        .samples::<f32>()
                        .try_fold(Vec::<T>::with_capacity(len), |mut acc, sample| {
                            acc.push(
                                 T::from_f32_(sample?)
                            );
                            Ok(acc)
                        });
                    samples?
                }
                _ => {
                    return Err(anyhow::format_err!(
                        "Unsupported channels count: {}",
                        spec.channels
                    ))
                }
            },
            hound::SampleFormat::Int => match spec.channels {
                1 => {
                    let max = T::from_i32_(i32::MAX);
                    let samples: anyhow::Result<Vec<T>> = reader
                        .samples::<i32>()
                        .try_fold(Vec::<T>::with_capacity(len), |mut acc, sample| {
                                acc.push(
                                 T::from_i32_(sample?)/max
                            );
                            Ok(acc)

                    });
                    samples?
                }
                _ => {
                    return Err(anyhow::format_err!(
                        "Unsupported channels count: {}",
                        spec.channels
                    ))
                }
            },
        };
    Ok(RealSignal::from_vec(spec.sample_rate as f64, signal))
}

pub fn write_wav_real<T: SignalType>(
    path: &str,
    signal: &RealSignal<T>,
    sample_format: SampleFormat,
    normalize: bool,
) -> Result<()> {
    // add some error checks here for Int signals of unusually low precision. (and warnings) as well as out of bounds float signals
    let mut signal = signal.clone();
    if normalize {
        signal.normalize();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: signal.sample_rate as u32,
        bits_per_sample: 32,
        sample_format: sample_format.into(),
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    let result: Result<()> = match sample_format {
        SampleFormat::Float => signal.iter().try_for_each(|x| {
            writer.write_sample::<f32>(x.as_())?;
            Ok(())
        }),
        SampleFormat::Int => signal.iter().try_for_each(|x| {
            let max = T::from_i32_(i32::MAX);
            writer.write_sample::<i32>(T::clamp(*x*max, -max, max).as_())?;
            Ok(())
        }),
    };
    result?;
    Ok(())
}

// Need to develop tests. i32 I/O is defo busted :(

#[test]
fn test_wav_io() -> anyhow::Result<()> {
    init_tracing();

    static REAL_CHIRP_PATH_INT: &str = "data/test/real_chirp_i32.wav";
    static COMPLEX_CHIRP_PATH_INT: &str = "data/test/complex_chirp_i32.wav";
    static REAL_CHIRP_PATH_FLOAT: &str = "data/test/real_chirp_f32.wav";
    static COMPLEX_CHIRP_PATH_FLOAT: &str = "data/test/complex_chirp_f32.wav";

    static CHIRP_LEN: usize = 4096;

    static FS: f64 = 192000.0;
    static F1: f64 = 000.0;
    static F2: f64 = 96000.0;

    trace!("testing real wav i32 I/O");
    let sig_write = RealSignal::<f64>::chirp_real(FS, CHIRP_LEN, F1, F2);
    write_wav_real(REAL_CHIRP_PATH_INT, &sig_write, SampleFormat::Int, false)?;

    let sig_read = read_wav_real::<f64>(REAL_CHIRP_PATH_INT)?;

    spectrogram("plot/test/test_io/wav_int_real.png", sig_read.complex(), 512, 512-4, true, Some(-90.0));

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r-w).abs()).sum::<f64>()/CHIRP_LEN as f64;

    trace!("R/W loss for real i32: {err}");

    trace!("testing complex wav i32 I/O");
    let sig_write = Signal::<f64>::chirp_complex(FS, CHIRP_LEN, -F2, F2);
    write_wav_complex(COMPLEX_CHIRP_PATH_INT, &sig_write, SampleFormat::Int, false)?;

    let sig_read = read_wav_complex::<f64>( COMPLEX_CHIRP_PATH_INT)?;

    trace!("{}", sig_read.len());

    spectrogram("plot/test/test_io/wav_int_complex.png", sig_read.clone(), 512, 512-4, true, Some(-90.0));

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r.im-r.im).abs()+(r.re-w.re).abs()).sum::<f64>()/(CHIRP_LEN as f64 * 2.0);

    trace!("R/W loss for complex i32: {err}");

    trace!("testing real wav f32 I/O");
    let sig_write = RealSignal::<f64>::chirp_real(FS, CHIRP_LEN, F1, F2);
    write_wav_real(REAL_CHIRP_PATH_FLOAT, &sig_write, SampleFormat::Float, false)?;

    let sig_read = read_wav_real::<f64>(REAL_CHIRP_PATH_FLOAT)?;

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r-w).abs()).sum::<f64>()/CHIRP_LEN as f64;

    trace!("R/W loss for real f32: {err}");

    trace!("testing complex wav f32 I/O");
    let sig_write = Signal::<f64>::chirp_complex(FS, CHIRP_LEN, -F2, F2);
    write_wav_complex(COMPLEX_CHIRP_PATH_FLOAT, &sig_write, SampleFormat::Float, false)?;

    let sig_read = read_wav_complex::<f64>( COMPLEX_CHIRP_PATH_FLOAT)?;

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r.im-r.im).abs()+(r.re-w.re).abs()).sum::<f64>()/(CHIRP_LEN as f64 * 2.0);

    trace!("R/W loss for complex f32: {err}");


    Ok(())
}