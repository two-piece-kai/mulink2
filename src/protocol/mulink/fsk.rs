use crate::{
    core::block::fft::{FftInst, RustFftInst},
    plot::time::plot,
    prelude::{FromVec, Signal, SignalType},
    protocol::mulink::chip_pattern::MuLinkHopPattern,
};
use itertools::Itertools;
use log::{info, trace};
use num::{
    traits::{sign, ConstZero},
    Complex, Float, Zero,
};
use rand::Fill;

pub fn demod<T: SignalType>(
    signal: &Signal<T>,
    symbol_len: f64,
    debug: bool,
    debug_plots: bool,
) -> anyhow::Result<Vec<u8>> {
    let fs = signal.sample_rate;
    let len = signal.len() as f64 / fs;

    let num_bytes = (len / symbol_len / 8.0).round() as u64;
    let num_bits = (num_bytes * 8) as f64;
    let pairs = MuLinkHopPattern::new(13, 0.0, 10000.0, fs);

    let doppler_stretch = len / (num_bits * symbol_len);
    let symbol_len_stretch = symbol_len * doppler_stretch;

    let mut msg = Vec::<u8>::new();

    let inst = RustFftInst::<T>::new((symbol_len_stretch * fs * 4.0) as usize);
    let mut buffer = vec![Complex::<T>::zero(); (symbol_len_stretch * fs * 4.0) as usize];

    let mut certainty_avg = 0.00;

    for byte_id in 0..num_bytes {
        // let mut byte: u8 = 0;
        if debug {
            trace!("[byte {byte_id}]");
        }
        for bit_id in 0..8 {
            buffer.fill(Complex::zero());
            let bit_id_global = byte_id * 8 + bit_id;
            let sym_begin =
                ((bit_id_global as f64 * symbol_len_stretch - symbol_len_stretch * 0.05) * fs)
                    .floor()
                    .clamp(0.0, signal.len() as f64 - 1.0) as usize;
            let sym_end = ((bit_id_global as f64 * symbol_len_stretch + symbol_len_stretch * 1.05)
                * fs)
                .ceil()
                .clamp(0.0, signal.len() as f64 - 1.0) as usize;
            let sym = signal.subslice(sym_begin..sym_end);
            buffer[0..sym.len()].clone_from_slice(&sym);
            inst.fft_fwd(&mut buffer)?;
            let sym = Signal::from_vec(0.0, buffer.clone()).abs();
            let sym_slices = sym.split_at(sym.len() / 2);
            let mut sym = sym_slices.1.to_vec();
            sym.append(&mut sym_slices.0.to_vec());

            let pair = pairs[bit_id_global as usize];
            let range = (pair.guard_f0, pair.guard_f1);
            let sym_len = sym.len() as f64;
            //let sym_len = sym_len - sym_len / 2.0;
            let range = (
                (
                    ((range.0 .0 + 1.0) * sym_len / 2.0).floor() as usize,
                    ((range.0 .1 + 1.0) * sym_len / 2.0).ceil() as usize,
                ),
                (
                    ((range.1 .0 + 1.0) * sym_len / 2.0).floor() as usize,
                    ((range.1 .1 + 1.0) * sym_len / 2.0).ceil() as usize,
                ),
            );
            //println!("{range:?}, {sym_len}");

            let mut vec = vec![0.0; sym.len()];
            vec[range.0 .0..range.1 .1].fill(1.0);

            let sum0 = sym[range.0 .0..range.0 .1]
                .iter()
                .cloned()
                .sum::<T>()
                .as_f64();
            let sum1 = sym[range.1 .0..range.1 .1]
                .iter()
                .cloned()
                .sum::<T>()
                .as_f64();

            // byte >>= 1;
            // if sum1 > sum0 {
            // byte |= 128;
            // }

            let c1 = sum1 * sum1;
            let c0 = sum0 * sum0;

            let c_tot = c1 + c0;

            let probability = ((c1 / c_tot - c0 / c_tot + 1.0) / 2.0 * 256.0).floor() as u8;
            let certainty = (c1 / c_tot - c0 / c_tot).abs() * 100.0;

            certainty_avg += certainty;

            msg.push(probability);

            if debug {
                trace!(
                "[bit {bit_id_global:>3}] {sum0:>9.02} / {sum1:>9.02} = {} ({:>5.02}) in ({:>5.1} kHz, {:>5.1} kHz) [{certainty:.1} % certainty]",
                if sum1 > sum0 { 1 } else { 0 },
                // if sum1 > sum0 {
                //     sum1 / sum0
                // } else {
                //     sum0 / sum1
                // },
                (c1 / c_tot - c0 / c_tot + 1.0) / 2.0 ,
                pair.f0 * 12.0,
                pair.f1 * 12.0,
                );
            }

            // sum0 += sum0;

            if debug_plots {
                plot(&format!("plot/bits/{}.png", bit_id_global), "bit", &sym);
                plot(&format!("plot/bits/{}_bin.png", bit_id_global), "bit", &vec);
            }
        }
        // if debug {
        //     if (byte as char).is_alphanumeric() {
        //         println!(" -> {} ({byte})", byte as char);
        //     } else {
        //         println!(" -> [non-alphanumeric] ({byte})");
        //     }
        // }
        // msg.push(byte);
    }

    certainty_avg /= msg.len() as f64;
    trace!("MEAN CERTAINTY: {certainty_avg}");
    //00101100
    //01101000
    // if let Ok(msg) = String::from_utf8(msg.clone()) {
    //     println!("{msg}");
    // } else {
    //     println!("{msg:?}");
    // }

    Ok(msg)
}

pub fn demod_extra<T: SignalType>(
    signal: &Signal<T>,
    symbol_len: f64,
    debug: bool,
    debug_plots: bool,
) -> anyhow::Result<(Vec<u8>, f64, f64)> {
    demod_extra_impl(signal, symbol_len, None, debug, debug_plots)
}


pub fn demod_extra_bits<T: SignalType>(
    signal: &Signal<T>,
    symbol_len: f64,
    num_bits: usize,
    debug: bool,
    debug_plots: bool,
) -> anyhow::Result<(Vec<u8>, f64, f64)> {
    if num_bits == 0 || num_bits % 8 != 0 {
        return Err(anyhow::anyhow!("num_bits ({num_bits}) must be a non-zero multiple of 8"));
    }
    let needed = (num_bits as f64 * symbol_len * signal.sample_rate).floor() as usize;
    if signal.len() < needed {
        return Err(anyhow::anyhow!(
            "signal too short: {} samples, need {needed} for {num_bits} symbols",
            signal.len()
        ));
    }
    demod_extra_impl(signal, symbol_len, Some(num_bits), debug, debug_plots)
}

fn demod_extra_impl<T: SignalType>(
    signal: &Signal<T>,
    symbol_len: f64,
    fixed_bits: Option<usize>,
    debug: bool,
    debug_plots: bool,
) -> anyhow::Result<(Vec<u8>, f64, f64)> {
    let fs = signal.sample_rate;
    let len = signal.len() as f64 / fs;

    let (num_bytes, num_bits, doppler_stretch) = match fixed_bits {
        // Known length: no stretch estimate, symbols sit on the nominal symbol_len grid.
        Some(bits) => ((bits / 8) as u64, bits as f64, 1.0),
        // Unknown length: infer it from the segment and spread any leftover time
        // (Doppler / clock drift) evenly over the symbols.
        None => {
            let num_bytes = (len / symbol_len / 8.0).round() as u64;
            let num_bits = (num_bytes * 8) as f64;
            (num_bytes, num_bits, len / (num_bits * symbol_len))
        }
    };
    let pairs = MuLinkHopPattern::new(13, 0.0, 10000.0, fs);
    let _ = num_bits;

    let symbol_len_stretch = symbol_len * doppler_stretch;

    let mut msg = Vec::<u8>::new();

    let inst = RustFftInst::<T>::new((symbol_len_stretch * fs * 4.0) as usize);
    let mut buffer = vec![Complex::<T>::zero(); (symbol_len_stretch * fs * 4.0) as usize];

    let mut certainties = Vec::<f64>::new();

    for byte_id in 0..num_bytes {
        // let mut byte: u8 = 0;
        if debug {
            trace!("[byte {byte_id}]");
        }
        for bit_id in 0..8 {
            buffer.fill(Complex::zero());
            let bit_id_global = byte_id * 8 + bit_id;
            let sym_begin =
                ((bit_id_global as f64 * symbol_len_stretch - symbol_len_stretch * 0.05) * fs)
                    .floor()
                    .clamp(0.0, signal.len() as f64 - 1.0) as usize;
            let sym_end = ((bit_id_global as f64 * symbol_len_stretch + symbol_len_stretch * 1.05)
                * fs)
                .ceil()
                .clamp(0.0, signal.len() as f64 - 1.0) as usize;
            let sym = signal.subslice(sym_begin..sym_end);
            buffer[0..sym.len()].clone_from_slice(&sym);
            inst.fft_fwd(&mut buffer)?;
            let sym = Signal::from_vec(0.0, buffer.clone()).abs();
            let sym_slices = sym.split_at(sym.len() / 2);
            let mut sym = sym_slices.1.to_vec();
            sym.append(&mut sym_slices.0.to_vec());

            let pair = pairs[bit_id_global as usize];
            let range = (pair.guard_f0, pair.guard_f1);
            let sym_len = sym.len() as f64;
            //let sym_len = sym_len - sym_len / 2.0;
            let range = (
                (
                    ((range.0 .0 + 1.0) * sym_len / 2.0).floor() as usize,
                    ((range.0 .1 + 1.0) * sym_len / 2.0).ceil() as usize,
                ),
                (
                    ((range.1 .0 + 1.0) * sym_len / 2.0).floor() as usize,
                    ((range.1 .1 + 1.0) * sym_len / 2.0).ceil() as usize,
                ),
            );
            //println!("{range:?}, {sym_len}");

            let mut vec = vec![0.0; sym.len()];
            vec[range.0 .0..range.1 .1].fill(1.0);

            let sum0 = sym[range.0 .0..range.0 .1]
                .iter()
                .cloned()
                .sum::<T>()
                .as_f64();
            let sum1 = sym[range.1 .0..range.1 .1]
                .iter()
                .cloned()
                .sum::<T>()
                .as_f64();

            // byte >>= 1;
            // if sum1 > sum0 {
            // byte |= 128;
            // }

            let c1 = sum1 * sum1;
            let c0 = sum0 * sum0;

            let c_tot = c1 + c0;

            let probability = ((c1 / c_tot - c0 / c_tot + 1.0) / 2.0 * 256.0).floor() as u8;
            let certainty = (c1 / c_tot - c0 / c_tot).abs() * 100.0;

            // certainty_avg += certainty;
            certainties.push(certainty);

            msg.push(probability);

            if debug {
                trace!(
                "[bit {bit_id_global:>3}] {sum0:>9.02} / {sum1:>9.02} = {} ({:>5.02}) in ({:>5.1} kHz, {:>5.1} kHz) [{certainty:.1} % certainty]",
                if sum1 > sum0 { 1 } else { 0 },
                // if sum1 > sum0 {
                //     sum1 / sum0
                // } else {
                //     sum0 / sum1
                // },
                (c1 / c_tot - c0 / c_tot + 1.0) / 2.0 ,
                pair.f0 * 12.0,
                pair.f1 * 12.0,
                );
            }

            // sum0 += sum0;

            if debug_plots {
                plot(&format!("plot/bits/{}.png", bit_id_global), "bit", &sym);
                plot(&format!("plot/bits/{}_bin.png", bit_id_global), "bit", &vec);
            }
        }
        // if debug {
        //     if (byte as char).is_alphanumeric() {
        //         println!(" -> {} ({byte})", byte as char);
        //     } else {
        //         println!(" -> [non-alphanumeric] ({byte})");
        //     }
        // }
        // msg.push(byte);
    }

    let certainty_avg = certainties.iter().sum::<f64>() / msg.len() as f64;
    let certainty_std = (certainties
        .iter()
        .map(|x| f64::powi(*x - certainty_avg, 2))
        .sum::<f64>()
        / (msg.len() as f64))
        .sqrt();
    trace!("MEAN CERTAINTY: {certainty_avg}, std: {certainty_std}");
    //00101100
    //01101000
    // if let Ok(msg) = String::from_utf8(msg.clone()) {
    //     println!("{msg}");
    // } else {
    //     println!("{msg:?}");
    // }

    Ok((msg, certainty_avg, certainty_std))
}
