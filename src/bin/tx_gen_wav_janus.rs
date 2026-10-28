//! Writes a JANUS-encoded test transmission to a WAV file, for testing the receiver offline.
//!
//! Frames are built the same way as tx_simple.rs: 3 up-chirps, then the message
//! convolutionally encoded and interleaved (JANUS) and sent as hop-FSK, then 3 down-chirps.
//! Frames can be repeated with a gap between them, and the down-chirp postamble can be left
//! out. This is the generator that matches demod_wav_iclisten.
use core::f32;

use clap::Parser;
use mulink_dsp::{
    core::{gen::fir::hamming, signal::RealSignal},
    prelude::init_tracing,
    protocol::mulink::chip_pattern::MuLinkHopPattern,
    robust::codec::janus_encode_bytes,
};

/// Command-line options.
#[derive(Clone, Debug, Parser)]
pub struct TxArgs {
    /// Text to send. Its byte length is what demod_wav_iclisten's --msg-len must match.
    #[arg(short, long, default_value_t = String::from("Hello from the MuLink test 32 ch"))]
    msg: String,
    /// Output WAV path.
    #[arg(short, long, default_value_t = String::from("plot/test_tx_janus.wav"))]
    wav: String,
    /// Length of one FSK symbol, in seconds.
    #[arg(short, long, default_value_t = 0.02)]
    symbol_len: f64,
    /// Sample rate of the output file, in Hz.
    #[arg(long, default_value_t = 192000.0)]
    fs: f64,
    /// Number of frames to write
    #[arg(long, default_value_t = 1)]
    repeat: usize,
    /// Seconds of silence between frames
    #[arg(long, default_value_t = 5.0)]
    gap: f64,
    /// Leave out the 3 down-chirps after the data
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_postamble: bool,
}
/// Running
/// 
/// cargo run --release --bin tx_gen_wav_janus -- -m "Hello from the MuLink test 32 ch" -w plot/test_20.wav -s 0.01 --repeat 20 --gap 3 --no-postamble
fn main() -> anyhow::Result<()> {
    init_tracing()?;
    let args = TxArgs::parse();
    // Encoded output is one bit per byte: 16 * (bytes + 1) bits (rate-1/2 code plus flush bits).
    let msg = janus_encode_bytes(args.msg.as_bytes());
    let fs = args.fs;
    let symbol_len = args.symbol_len;
    // Preamble timing: 20 ms chirps separated by 100 ms of silence.
    let chirp_len: f64 = 0.02;
    let silence_len: f64 = 0.1;
    let samples_per_symbol = (fs * symbol_len).round() as usize;
    // 13 hops, each a pair of tones (bit 0 / bit 1) spread across 65-75 kHz.
    let hop_tab = MuLinkHopPattern::new(13, 70000.0, 10000.0, fs);

    // Pre-build the two tones for every hop so the data loop just copies them in.
    let mut chip_bank = Vec::<(RealSignal<f32>, RealSignal<f32>)>::new();
    for hop in hop_tab.tab.iter() {
        // Hop frequencies are stored normalised to Nyquist; convert to rad/s.
        let w0 = hop.f0 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let w1 = hop.f1 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let s0 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w0));
        let s1 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w1));
        // Flat-top window: half a Hamming ramp on each end (1/8 symbol each), 1.0 between.
        // Tapering the edges reduces spectral splatter between symbols.
        let window_hamming = hamming::<f32>(samples_per_symbol / 4);
        let (window_front, window_back) = window_hamming.split_at(samples_per_symbol / 8);
        let mut window = vec![0.0; samples_per_symbol];
        window[0..window_front.len()].clone_from_slice(window_front);
        window[window_front.len()..window_front.len() + samples_per_symbol - window_hamming.len()]
            .fill(1.0);
        window[window_front.len() + samples_per_symbol - window_hamming.len()..]
            .clone_from_slice(window_back);
        let window = RealSignal::from_vec(fs, window);
        chip_bank.push((&s0 * &window, &s1 * &window));
    }
    // Sync chirps sweep the same 65-75 kHz band as the data.
    let chirp_up =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 65000.0, 75000.0);
    let chirp_down =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 75000.0, 65000.0);
    let silence = RealSignal::<f32>::from_vec(fs, vec![0.0; (fs * silence_len).round() as usize]);
    let gap = RealSignal::<f32>::from_vec(fs, vec![0.0; (fs * args.gap).round() as usize]);

    let mut signal = RealSignal::<f32>::new(fs);
    for f in 0..args.repeat {
        if f > 0 {
            signal.extend_from_slice(&gap);
        }
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
            let pair = &chip_bank[hop_tab[idx].idx];
            signal.extend_from_slice(if (c & 1) == 1 { &pair.1 } else { &pair.0 });
        }
        // Postamble (optional; the current receiver doesn't need it).
        if !args.no_postamble {
            for _ in 0..3 {
                signal.extend_from_slice(&silence);
                signal.extend_from_slice(&chirp_down);
            }
        }
        // Trailing silence.
        for _ in 0..5 {
            signal.extend_from_slice(&silence);
        }
    }
    signal.write_wav(&args.wav, hound::SampleFormat::Float, false)?;
    println!(
        "wrote {} ({} frames, {} bytes -> {} symbols, {:.1} s)",
        args.wav,
        args.repeat,
        args.msg.len(),
        msg.len(),
        signal.len() as f64 / fs
    );
    Ok(())
}
