use std::fs::OpenOptions;

use std::io::Write;

use anyhow::anyhow;
use itertools::Itertools;
use num::Complex;

use crate::core::r#gen::chirp::chirp_complex;
use crate::{core::gen::chirp::chirp_real, prelude::*};

pub fn read_txt_real<T: SignalType>(sample_rate: f64, path: &str) -> anyhow::Result<Vec<T>> {
    let mut reader = csv::ReaderBuilder::new().has_headers(false).from_path(path)?;
    let sig = reader.deserialize().map(|sample: Result<T, csv::Error>| sample.unwrap_or_default()).collect_vec();
    Ok(sig)
}

pub fn read_txt_complex<T: SignalType>(sample_rate: f64, path: &str) -> anyhow::Result<Signal<T>> {
    let sig = read_txt_real::<T>(sample_rate, path)?.chunks_exact(2).map(|chunk| Complex {re:chunk[0], im:chunk[1]}).collect_vec();
    Ok(Signal::from_vec(sample_rate, sig))
}

pub fn write_txt_real<T: SignalType>(path: &str, sig: &[T]) -> anyhow::Result<()> {
    let mut output = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    let written = sig.iter().map_while(|sample| writeln!(output, "{sample}").ok()).count();
    if written != sig.len() {
        return Err(anyhow!("Expected to write {} samples in {path}, found that only {written} were written.", sig.len()))
    }
    Ok(())
}
pub fn write_txt_complex<T: SignalType>(path: &str, sig: &[Complex<T>]) -> anyhow::Result<()> {
    let mut output = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    let written = sig.iter().map_while(|sample| writeln!(output, "{}\n{}", sample.re, sample.im).ok()).count();
    if written != sig.len() {
        return Err(anyhow!("Expected to write {} samples in {path}, found that only {written} were written.", sig.len()))
    }
    Ok(())
}

#[test]
fn test_txt_io() -> anyhow::Result<()> {
    init_tracing();

    static REAL_CHIRP_PATH: &str = "data/test/real_chirp.txt";
    static COMPLEX_CHIRP_PATH: &str = "data/test/complex_chirp.txt";

    static CHIRP_LEN: usize = 128;

    trace!("testing real txt I/O");
    let sig_write = chirp_real::<f64>(CHIRP_LEN, 0.1, 0.9);
    write_txt_real(REAL_CHIRP_PATH, &sig_write)?;

    let sig_read = read_txt_real::<f64>(192000.0, REAL_CHIRP_PATH)?;

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r-w).abs()).sum::<f64>()/CHIRP_LEN as f64;

    trace!("R/W loss for real: {err}");

    trace!("testing complex txt I/O");
    let sig_write = chirp_complex::<f64>(CHIRP_LEN, 0.1, 0.9);
    write_txt_complex(COMPLEX_CHIRP_PATH, &sig_write)?;

    let sig_read = read_txt_complex::<f64>(192000.0, COMPLEX_CHIRP_PATH)?;

    let err: f64 = sig_read.iter().zip(sig_write.iter()).map(|(r,w)| (r.im-r.im).abs()+(r.re-w.re).abs()).sum::<f64>()/(CHIRP_LEN as f64 * 2.0);

    trace!("R/W loss for complex: {err}");


    Ok(())
}