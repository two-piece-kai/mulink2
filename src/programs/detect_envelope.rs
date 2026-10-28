use std::{cmp::Ordering, fmt::format, fs::create_dir_all, path::{Path, PathBuf}};

use anyhow::{anyhow, Context};
use clap::Parser;
use fmtastic::Subscript;
use itertools::Itertools;

use crate::{core::{block::fft::{fft_mag, FftInst, RustFftInst}, signal::{Amplitude, RealSignal}, util::{compute_snr::{compute_snr, compute_snr_narrow, compute_snr_real}, find_direct_path::find_direct_path}}, plot::time::{plot, plot_complex}, prelude::{FromVec, Signal, SignalType}};

/// Impulse response extraction program

fn max<T: SignalType>(a: &&T,b: &&T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Less)
}

fn max_at(a: &(usize, f32),b: &(usize, f32)) -> Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Less)
}


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

pub fn detect_envelope(args: IrExtractArgs) -> anyhow::Result<()> {
    create_dir_all(&args.output_dir)?;

    // Sanitize output directory string
    let output_dir = PathBuf::from(args.output_dir);
    let output_dir = output_dir.canonicalize()?;
    let output_dir = output_dir.to_str().context("Could not convert path to string")?;

    create_dir_all(&format!("{output_dir}/chips"))?;

    let mut signal = Signal::<f32>::read_wav(&args.src_signal)?;
    plot_complex(&format!("{output_dir}/signal_in_complex.png"), &format!("Input Signal"), &signal);
    plot(&format!("{output_dir}/signal_in_abs.png"), &format!("Input Signal"), &signal.abs());
    // signal.sample_rate *= 125.0/120.0;

    // 20ms chirp
    let chirp_len = (signal.sample_rate*0.02) as usize;
    // 100ms silence
    let silence_len = (signal.sample_rate*0.100) as usize;
    // 10ms back porch (echo zone)
    let back_porch_len = (signal.sample_rate*0.01) as usize;
    // zone to exclude in front of chirp candidates
    let exclusion_zone_len = (signal.sample_rate*0.04) as usize;
    let sample_rate = signal.sample_rate;

    let mut filt = Signal::<f32>::chirp_complex(signal.sample_rate, chirp_len, -5000.0, 5000.0).conj();
    filt.reverse();
    let ir = signal.fftfilt(&filt, crate::core::block::filter::ConvShape::VALID)?;
    let ir_abs = ir.abs();
    plot(&format!("{output_dir}/matched_filt.png"), "Chirp matched filter", &ir.abs());

    // Find top 4 direct paths
    let peaks = find_direct_path(&ir, Amplitude::Log(-7.0), exclusion_zone_len)?;

    println!("peaks_len: {}", peaks.len());

    if peaks.len() < 4 {
        return Err(anyhow!("Did not find adequate number of peaks"));
    }

    let top_peaks = peaks[0..4].iter().cloned().sorted_by(max_idx).map(|(idx, mag)|
        (idx, idx as f64 / signal.sample_rate, compute_snr_narrow(&ir, idx-200..idx-100, idx..idx+10).log::<f64>(), mag)
    ).collect_vec();

    // for (idx, _, _) in top_peaks.iter() {
    //     let idx = *idx;
        
    // }
    println!("Top 4 peaks: ");
    for (idx, (peak_idx, peak_time, peak_snr, peak_amplitude)) in top_peaks.iter().enumerate() {
        println!("Peak #{idx}@idx={peak_idx} ({peak_time:.3}s) SNR: {peak_snr:.3} dB, amplitude: {peak_amplitude:.4}")
    }
    println!("DTAnalysis: ");
    for (
        (idx_0, (peak_idx_0, peak_time_0, peak_snr_0, peak_amplitude_0)),
        (idx_1, (peak_idx_1, peak_time_1, peak_snr_1, peak_amplitude_1))
    ) in top_peaks.iter().enumerate().tuple_windows::<(_,_)>() {
        println!("Δt{}={:.3}s | Δidx{}={}", Subscript(idx_0), peak_time_1-peak_time_0,  Subscript(idx_0), peak_idx_1-peak_idx_0);
    }

    let dt0 = top_peaks[1].1-top_peaks[0].1;
    let dt1 = top_peaks[2].1-top_peaks[1].1;
    let dt2 = top_peaks[3].1-top_peaks[2].1;

    // total preamble doppler +/- 5% && preamble trenches within 2% of each other && message length > 0.0ms
    let envelope_valid = (1.0-(dt0+dt1)/0.240).abs() < 0.05 && (dt0-dt1).abs() < 0.02 && (dt2-dt1).abs() > 0.200;
    let doppler_ratio = (dt0+dt1)/0.240;

    println!("doppler_ratio: {doppler_ratio}");

    println!("envelope valid: {envelope_valid}");

    let symbol_len_s = 0.01*doppler_ratio;
    let symbol_len = (sample_rate * symbol_len_s).round() as usize;

    let msg_range = top_peaks[2].0+(chirp_len+silence_len)..top_peaks[3].0-(silence_len);
    let msg_start_s = top_peaks[2].1+0.12;
    let msg_end_s = top_peaks[3].1-0.1;
    println!("message in: t=[{:.3}, {:.3}] (idx=[{},{}])", msg_start_s, msg_end_s, msg_range.start, msg_range.end);
    let msg_len_s = dt2-0.1-0.12;
    println!("message len: {:.3}s / {:.3} bits / {:.3} bytes", msg_len_s, msg_len_s/symbol_len_s, msg_len_s/(symbol_len_s*8.0));

    let n_bits = (msg_len_s/0.01).round() as usize;

    println!("n_bits: {n_bits}");

    //let mut chips: Vec<RealSignal<f32>> = Vec::new();

    let mut bits = Vec::<u8>::new();

    for idx in 0..n_bits {

        let bin = 12-((idx * 3) % 13);

        let chip_start = ((msg_start_s+symbol_len_s*idx as f64)*sample_rate).round() as usize;
        let chip_end = ((msg_start_s+symbol_len_s*(idx+1) as f64)*sample_rate).round() as usize;
        

        let chip_fft = signal.subslice(chip_start..chip_end).fft_fwd()?;
        // if idx == 0 {
        //      plot_complex(&format!("{output_dir}/first_chip.png"), &format!("chip @idx={chip_start}"), &signal[chip_start-100..chip_end+100]);
        // }
        let mut mag = chip_fft.abs();

        let width = mag.len() as f32 * (5000.0/(sample_rate as f32 / 2.0));
        
        let band_start = mag.len() as f32 / 2.0 - width / 2.0;
        let bin_width = width / 13.0;

        println!("{} :: {}", bin_width, mag.len());
    
        // give margin of 1 bin on both sides
        let bin_start = (band_start + bin_width * (bin as i32 - 1) as f32).floor() as usize;
        let bin_end =  (band_start + bin_width * (bin as i32 + 2) as f32).ceil() as usize;

        let rotate_dist = mag.len()/2;
        mag.rotate_right(rotate_dist);
        plot(&format!("{output_dir}/chips/{idx}_fft.png"), &format!("chip @idx={chip_start}"), &mag);
        plot_complex(&format!("{output_dir}/chips/{idx}.png"), &format!("chip @idx={chip_start}"), &signal[chip_start-100..chip_end+100]);
        //println!("{}", mag.len());
        // chips.push(mag);
        let dominant_idx = bin_start + mag[bin_start..=bin_end].iter().position_max_by(max).unwrap_or_default();
        // let dominant_idx =  mag.iter().position_max_by(max).unwrap_or_default();
        let dominant =  (dominant_idx as i64-(mag.len()/2) as i64) as f32 * (sample_rate/(mag.len() as f64)) as f32;
        let dominant_sym =  dominant/400.0+5000.0/400.0;
        
        let bit: u8 = dominant_sym.round() as u8 % 2;

        //let snr = compute_snr_real(&mag, dominant_idx..dominant_idx+1)?;
        println!("chip {idx} size: {} freq: {dominant} sym: {dominant_sym} bit: {bit} range [{bin_start} {bin_end}]", chip_end-chip_start);
        bits.push(bit);
    }

    let mut is_ascii = true;
    let mut bytes = Vec::new();
    for byte in bits.chunks(8) {
        let byte = byte.iter().enumerate().fold(0_u8, |acc, (idx, bit)| acc | (*bit << idx));
        if byte < ' ' as u8 || byte > '~' as u8 {
            is_ascii = false;
        }
        bytes.push(byte);
        
    }
    if is_ascii {
        println!("{}",String::from_utf8(bytes)?);
    }else {
        println!("{bytes:?}");
    }
    // let freqs = chips.iter().map(| chip| chip.iter().position_max_by(max).unwrap_or_default() as f32 * (sample_rate/(2.0*chip.len() as f64)) as f32).collect_vec();

    // for (idx,freq) in freqs.iter().enumerate() {
    //     println!("chip {idx}")
    // }

    //println!("Top 4: {:#?}", &peaks[0..4]);

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