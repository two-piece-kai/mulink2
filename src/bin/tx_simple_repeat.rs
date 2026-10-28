//! Transmits the same JANUS-encoded frame over and over through the default audio output.
//!
//! Frames are built exactly like tx_simple.rs (3 up-chirps, JANUS-encoded hop-FSK data,
//! 3 down-chirps) and re-sent every `--interval-s` seconds until the program is stopped.
//! Useful for leaving a transmitter running while testing the receiver.
use core::f32;
use std::{thread::sleep, time::Duration};

use clap::Parser;
use log::trace;
use mulink_dsp::{
    backend::cpal::CpalBackend,
    core::{
        r#gen::fir::hamming,
        signal::RealSignal,
    },
    prelude::init_tracing,
    protocol::mulink::chip_pattern::MuLinkHopPattern,
    robust::codec::janus_encode_bytes,
};

/// Command-line options.
#[derive(Clone, Debug, Parser)]
pub struct TxArgs {
    /// Text to send.
    #[arg(short, long, default_value_t = String::from("hello"))]
    msg: String,
    /// Length of one FSK symbol, in seconds.
    #[arg(short, long, default_value_t = 0.02)]
    symbol_len: f64,
    /// Seconds between the start of one transmission and the next.
    #[arg(short, long, default_value_t = 15.0)]
    interval_s: f64,
    /// Send each symbol as a short chirp across its tone's guard band instead of a pure tone.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    minichirp: bool,
}

fn main() -> anyhow::Result<()> {
    init_tracing()?;
    // Taper each tone's edges to reduce spectral splatter between symbols.
    let window_chips = true;
    let args = TxArgs::parse();
    // Encoded output is one bit per byte: 16 * (bytes + 1) bits (rate-1/2 code plus flush bits).
    let msg = janus_encode_bytes(args.msg.as_bytes());
    let fs = 192000.0;

    trace!("Transmitting with args: {args:?}");

    // Preamble timing: 20 ms chirps separated by 100 ms of silence.
    let symbol_len = args.symbol_len;
    let chirp_len: f64 = 0.02;
    let silence_len: f64 = 0.1;

    let samples_per_symbol = (fs * symbol_len).round() as usize;

    // 13 hops, each a pair of tones (bit 0 / bit 1) spread across 65-75 kHz.
    let hop_tab = MuLinkHopPattern::new(13, 70000.0, 10000.0, fs);

    let mut signal = RealSignal::<f32>::new(fs);

    // Pre-build the two symbols for every hop so the data loop just copies them in.
    let mut chip_bank = Vec::<(RealSignal<f32>, RealSignal<f32>)>::new();

    for hop in hop_tab.tab.iter() {
        // Hop frequencies are stored normalised to Nyquist; convert to rad/s.
        let w0 = hop.f0 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let w1 = hop.f1 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let s0 = if args.minichirp {
            RealSignal::<f32>::chirp_real(
                fs,
                samples_per_symbol,
                hop.guard_f0.0 * fs / 2.0,
                hop.guard_f0.1 * fs / 2.0,
            )
        } else {
            RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w0))
        };
        let s1 = if args.minichirp {
            RealSignal::<f32>::chirp_real(
                fs,
                samples_per_symbol,
                hop.guard_f1.0 * fs / 2.0,
                hop.guard_f1.1 * fs / 2.0,
            )
        } else {
            RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w1))
        };

        // Flat-top window: half a Hamming ramp on each end (1/8 symbol each), 1.0 between.
        let window_hamming = hamming::<f32>(samples_per_symbol / 4);
        let (window_front, window_back) = window_hamming.split_at(samples_per_symbol / 8);

        let mut window = vec![0.0; samples_per_symbol];
        window[0..window_front.len()].clone_from_slice(window_front);
        window[window_front.len()..window_front.len() + samples_per_symbol - window_hamming.len()]
            .fill(1.0);
        window[window_front.len() + samples_per_symbol - window_hamming.len()..]
            .clone_from_slice(window_back);

        let window = RealSignal::from_vec(fs, window);

        let (s0, s1) = if window_chips {
            (&s0 * &window, &s1 * &window)
        } else {
            (s0, s1)
        };

        chip_bank.push((s0, s1));
    }

    // Sync chirps sweep the same 65-75 kHz band as the data.
    let chirp_up =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 65000.0, 75000.0);

    let chirp_down =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 75000.0, 65000.0);

    let silence = RealSignal::<f32>::from_vec(fs, vec![0.0; (fs * silence_len).round() as usize]);

    // Lead-in silence so the receiver has some noise to look at first.
    for _ in 0..5 {
        signal.extend_from_slice(&silence);
    }
    // Preamble.
    for _ in 0..3 {
        signal.extend_from_slice(&chirp_up);
        signal.extend_from_slice(&silence);
    }
    // Data: one symbol per encoded bit. Symbol n uses hop pattern entry n.
    for (idx, c) in msg.iter().enumerate() {
        let chip_id = idx;
        let bit = (c & 1) == 1;
        if bit {
            signal.extend_from_slice(&chip_bank[hop_tab[chip_id].idx].1);
        } else {
            signal.extend_from_slice(&chip_bank[hop_tab[chip_id].idx].0);
        }
    }
    // Postamble.
    for _ in 0..3 {
        signal.extend_from_slice(&silence);
        signal.extend_from_slice(&chirp_down);
    }
    // Trailing silence.
    for _ in 0..5 {
        signal.extend_from_slice(&silence);
    }

    let sender = CpalBackend::playback(2, 192000.0);
    //let len = signal.len() as f64 / fs;
    // Queue a copy of the frame, then wait. The interval should be longer than the frame
    // itself, otherwise frames pile up in the playback queue. The loop never ends, so there is
    // no Ok(()) after it; stop the program with Ctrl+C.
    loop {
        sender.send(signal.clone()).unwrap();
        sleep(Duration::from_millis((args.interval_s * 1000.0) as u64));
    }
}
