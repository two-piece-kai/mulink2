use std::{cmp::Ordering, fs::create_dir_all, path::{Path, PathBuf}};

use anyhow::Context;
use clap::Parser;
use itertools::Itertools;

use crate::{core::{signal::Amplitude, util::find_direct_path::find_direct_path}, plot::time::{plot, plot_complex}, prelude::{FromVec, Signal}};

/// Impulse response extraction program


fn max_idx(a: &(usize, f32),b: &(usize, f32)) -> Ordering {
    a.0.partial_cmp(&b.0).unwrap_or(Ordering::Less)
}
fn min_idx(a: &(usize, f32),b: &(usize, f32)) -> Ordering {
    a.0.partial_cmp(&b.0).unwrap_or(Ordering::Less).reverse()
}

#[derive(Debug,Clone,Parser)]
pub struct IrExtractArgs {
    src_signal: String,
    output_dir: String,
}

pub fn ir_extract(args: IrExtractArgs) -> anyhow::Result<()> {
    create_dir_all(&args.output_dir)?;

    // Sanitize output directory string
    let output_dir = PathBuf::from(args.output_dir);
    let output_dir = output_dir.canonicalize()?;
    let output_dir = output_dir.to_str().context("Could not convert path to string")?;

    let signal = Signal::<f32>::read_wav(&args.src_signal)?;

    // 20ms chirp
    let chirp_len = (signal.sample_rate*0.02) as usize;
    // 10ms back porch (echo zone)
    let back_porch_len = (signal.sample_rate*0.01) as usize;
    let sample_rate = signal.sample_rate;

    let filt = Signal::<f32>::chirp_complex(signal.sample_rate, chirp_len, -5000.0, 5000.0);
    filt.write_wav(&format!("{output_dir}/generated_filter.wav"), hound::SampleFormat::Float, false)?;
    plot_complex(&format!("{output_dir}/matched_filt_chirp_gen.png"), "Chirp used for matched filter", &filt);
    let filt = Signal::<f32>::read_wav(&format!("{output_dir}/generated_filter.wav"))?;
    let mut filt = filt.conj();
    filt.reverse();
    let ir = signal.fftfilt(&filt, crate::core::block::filter::ConvShape::VALID)?;
    let ir_abs = ir.abs();
    plot(&format!("{output_dir}/matched_filt.png"), "Chirp matched filter", &ir.abs());

    // Find top 4 direct paths
    let peaks = find_direct_path(&ir, Amplitude::Log(-6.0), 480)?;
    let top_peaks = peaks[0..4].iter().cloned().sorted_by(max_idx).collect_vec();
    println!("Top 4: {:#?}", top_peaks);

    // process the top 4 peaks.
    for i in  0..4 {
            let idx = top_peaks[i].0;

            plot_complex(&format!("{output_dir}/{i}_chirp.png"), &format!("chirp @idx={idx}"), &signal[idx-200..idx+200+chirp_len]);
            plot_complex(&format!("{output_dir}/{i}_ir.png"), &format!("impulse response @idx={idx}"), &ir[idx-200..idx+200+chirp_len]);
            plot(&format!("{output_dir}/{i}_ir-abs.png"), &format!("impulse response (abs) @idx={idx}"), &ir_abs[idx-200..idx+200+chirp_len]);

            let mut filt = Signal::<f32>::from_vec(sample_rate, signal[idx..idx+chirp_len+back_porch_len].to_vec()).conj();
            filt.reverse();

            // Just look at the chirp local area
            let autocorrelated = signal.fftfilt(&filt, crate::core::block::filter::ConvShape::VALID)?;
            plot_complex(&format!("{output_dir}/{i}_autocorrelated_zoomed.png"), &format!("autocorrelation of chirp {i} w/ self @idx={idx}"), &autocorrelated[idx-200..idx+200+chirp_len]);
            plot(&format!("{output_dir}/{i}_autocorrelated_zoomed-abs.png"), &format!("autocorrelation (abs) of chirp {i} w/ self @idx={idx}"), &autocorrelated.abs()[idx-200..idx+200+chirp_len]);
            
            // Look at the whole signal
            plot_complex(&format!("{output_dir}/{i}_autocorrelated.png"), &format!("autocorrelation w/ chirp {i} @idx={idx}"), &autocorrelated);
            plot(&format!("{output_dir}/{i}_autocorrelated-abs.png"), &format!("autocorrelation w/ chirp {i} (abs) @idx={idx}"), &autocorrelated.abs());
    }
    Ok(())
}