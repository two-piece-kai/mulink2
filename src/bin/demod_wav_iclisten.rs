//! Offline decoder for icListen recordings of JANUS-encoded hop-FSK frames.
//!
//! Reads a WAV file, converts it to complex baseband, finds each frame's 3 up-chirp
//! preamble with a matched filter, and then demodulates a fixed number of data symbols
//! after it (set by `--msg-len`). Down-chirps are not used.
//!
//! The transmitter side of this is tx_gen_wav_janus.rs.
use core::f32;
use std::{fs::create_dir_all, path::Path};

use clap::Parser;
use log::{info, trace};
use mulink_dsp::{
    core::{
        block::{
            buffer::HistoryBuffer,
            downconv::Downconv,
            filter::Filter,
            refragment::Refragmenter,
            windower::BrickWallWindower,
        },
        r#gen::{
            fir::hamming,
        },
        signal::{Amplitude, RealSignal},
    },
    plot::{
        spectrum::spectrogram,
        time::{plot},
    },
    prelude::{init_tracing, Signal},
    protocol::mulink::{
        chip_pattern::MuLinkHopPattern,
        fsk::demod_extra_bits,
        preamble::{preamble_detect_kern_timed, ControlDetection},
    },
    robust::codec::{janus_decode_bytes, janus_encode_bytes},
};
use num::{
    Complex, Zero,
};

/// Command-line options.
#[derive(clap::Parser)]
struct Args {
    /// WAV file to decode
    path: String,
    /// Expected message length in bytes (ASCII characters) BEFORE FEC encoding.
    /// The frame carries 16 * (msg_len + 1) symbols
    #[arg(short = 'n', long, default_value_t = 32)]
    msg_len: usize,
    /// Optional: the message that was actually sent
    #[arg(short, long)]
    expected: Option<String>,
}

const MSG_SNR_NOISE_S: f64 = 0.1;
const MSG_SNR_GUARD_S: f64 = 0.01;

/// Mean power a baseband segment.
fn mean_power(sig: &Signal<f32>) -> f64 {
    sig.iter().map(|v| v.norm_sqr() as f64).sum::<f64>() / sig.len().max(1) as f64
}

/// Peak power of a baseband segment.
fn peak_power(sig: &Signal<f32>) -> f64 {
    sig.iter().map(|v| v.norm_sqr() as f64).fold(0.0, f64::max)
}

fn main() -> anyhow::Result<()> {
    init_tracing()?;
    let window_chips = true;
    let args = Args::parse();
    let s_path = &args.path;

    let mut signal = Signal::<f32>::read_wav(s_path)?;
    let fs = signal.sample_rate;

    // Protocol parameters. These must match the transmitter (tx_gen_wav_janus.rs).
    let symbol_len: f64 = 0.02;
    let chirp_len: f64 = 0.02;
    let silence_len: f64 = 0.1;
    let n_chirps: f64 = 3.0; // chirps in the preamble and in the postamble
    let n_hops: usize = 13;
    let fc: f64 = 70000.0; // carrier / centre of the FSK hop band (Hz)
    let bw: f64 = 10000.0; // FSK hop band and chirp sweep width (Hz)

    // Baseband low-pass passes the signal band plus 20% margin.
    let downconv_bw = bw * 1.2;
    // Decimate to a complex baseband rate of about 2x the filter passband
    let down_ratio = (fs / (2.0 * downconv_bw)).floor().max(1.0);
    let fs_bb = fs / down_ratio;

    let chirp_period = chirp_len + silence_len;
    let preamble_len = n_chirps * chirp_period;
    info!(
        "fs = {fs} Hz, down_ratio = {down_ratio}, fs_bb = {fs_bb:.1} Hz, symbol_len = {symbol_len} s"
    );

    let samples_per_symbol = (fs * symbol_len).round() as usize;

    let hop_tab = MuLinkHopPattern::new(n_hops, fc, bw, fs);

    let mut chip_bank = Vec::<(RealSignal<f32>, RealSignal<f32>)>::new();

    for hop in hop_tab.tab.iter() {
        let w0 = hop.f0 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let w1 = hop.f1 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let s0 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w0));
        let s1 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w1));

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

    // Mix the 70 kHz band down to 0 Hz, low-pass, and decimate to fs_bb.
    let mut downconv = Downconv::<f32>::new(fs, fc, downconv_bw, 128, down_ratio as usize)?;

    // Up-chirp matched filter only
    let mut chirp_up =
        Signal::<f32>::chirp_complex(fs_bb, (chirp_len * fs_bb).round() as usize, -bw / 2.0, bw / 2.0)
            .conj();
    chirp_up.reverse();
    let mut filt_chirp_up = Filter::<f32>::new(chirp_up.clone())?;

    // Matched-filter output is cut into overlapping windows one preamble long, so every
    // preamble falls completely inside at least one window.
    let mut windower_up = BrickWallWindower::new(fs_bb, (preamble_len * fs_bb) as usize);

    let mut refrag = Refragmenter::<f32>::new(fs, 512);

    // Last 60 s of baseband, so the data after a preamble can be pulled out once it arrives.
    let mut history = HistoryBuffer::<f32>::new(fs_bb, 60.0);

    refrag.push(&mut signal);

    // Expected frame size, from the fixed message length.
    let n_bits = 16 * (args.msg_len + 1); // 8 bits/byte + 8 flush bits, rate-1/2 code
    let data_len = n_bits as f64 * symbol_len;
 
    let postamble_len = preamble_len;
    let min_snr = 12.0; // dB, minimum sync SNR for a preamble to start a frame
    let dedupe_s = 0.2; // detections closer than this are the same preamble

    let settle_s = 2.0 * symbol_len + 0.2 + chirp_period;
    info!(
        "expecting {} byte message -> {n_bits} symbols, {data_len:.2} s of data per frame",
        args.msg_len
    );

    let expected_bytes = args.expected.as_ref().map(|m| m.as_bytes().to_vec());
    if let Some(e) = &expected_bytes {
        if e.len() != args.msg_len {
            log::warn!(
                "--expected is {} bytes but --msg-len is {}; correct-decode count will be 0",
                e.len(),
                args.msg_len
            );
        }
    }
    let expected_bits = expected_bytes.as_ref().map(|e| janus_encode_bytes(e));

    // Preambles found but whose data hasn't fully arrived yet.
    let mut pending = Vec::<ControlDetection>::new();
    let mut holdoff_until = f64::NEG_INFINITY;

    let mut soi_start = 0;
    let mut last_preamble: Option<Signal<f32>> = None;

    let mut frames_count = 0;
    let mut frames_valid = 0;
    let mut frames_successful = 0;
    let mut frames_ber = Vec::new();

    create_dir_all("asa_logs")?;
    let file_name = Path::new(&s_path).file_name().unwrap().to_str().unwrap();
    let mut logger = csv::Writer::from_path(format!("asa_logs/{file_name}.csv"))?;
    logger.write_record([
        "Time (s)",
        "Start (s)",
        "SNR",
        "Max Msg SNR (dB)",
        "Avg Msg SNR (dB)",
        "Certainty",
        "Certainty STDDEV",
        "BER%",
        "BER reference",
        "Valid ASCII",
        "Message",
    ])?;

    for frag in &mut refrag {
        let Some(signal) = downconv.process(frag) else {
            continue;
        };

        let Ok(_) = history.write(&signal) else {
            panic!("FAILED TO WRITE TO HIST BUF");
        };

        // Look for preambles
        if let Some(upchirp_ir) = filt_chirp_up.process(signal.clone()) {
            windower_up.push(&mut upchirp_ir.clone());

            for window in &mut windower_up {
                let Ok(detection) = preamble_detect_kern_timed(
                    &window,
                    Amplitude::Log(-9.0),
                    (chirp_len * fs_bb * 0.75) as usize,
                    chirp_period,
                    chirp_len,
                ) else {
                    continue;
                };
                trace!("UP: {detection:?}");
                if detection.snr < min_snr || detection.time < holdoff_until {
                    continue;
                }
                // The same preamble can be found in more than one window: keep the strongest.
                if let Some(p) = pending
                    .iter_mut()
                    .find(|p| (p.time - detection.time).abs() < dedupe_s)
                {
                    if detection.amplitude > p.amplitude {
                        *p = detection;
                    }
                } else {
                    pending.push(detection);
                }
            }
        };

        // Demodulate every pending frame whose data has fully arrived.
        let now = (signal.time + signal.len() as i64) as f64 / fs_bb;
        pending.sort_by(|a, b| a.time.total_cmp(&b.time));
        while let Some(det) = pending.first().copied() {
            // det.time = end of the first up-chirp (matched-filter peak)
            let fsk_begin = det.time - chirp_len + preamble_len;
            let fsk_end = fsk_begin + data_len;
            if now < fsk_end + settle_s {
                break;
            }
            pending.remove(0);
            // Drop any detections that fall inside this frame's data or postamble.
            holdoff_until = fsk_end + postamble_len + dedupe_s;
            pending.retain(|p| p.time >= holdoff_until);

            let frame_begin = det.time - chirp_len - silence_len;
            soi_start = det.idx;
            if let Some(p) =
                history.get(soi_start..soi_start + ((preamble_len + chirp_len) * fs_bb).round() as i64)
            {
                last_preamble = Some(p);
            }
            info!(
                "FRAME DETECT: {:.03}s..{:.03}s @ snr = {:0.3} ({n_bits} symbols/{} bytes)",
                frame_begin,
                fsk_end,
                det.snr,
                n_bits / 8
            );
            frames_count += 1;

            let mut best: Option<(f64, (Vec<u8>, f64, f64))> = None;
            for k in [0.0, -1.0, 1.0] {
                let start_s = fsk_begin + k * chirp_period;
                let seg_start = (start_s * fs_bb).floor() as i64;
                let seg_end = seg_start + ((data_len + symbol_len) * fs_bb).ceil() as i64;
                let Some(seg) = history.get(seg_start..seg_end) else {
                    continue;
                };
                match demod_extra_bits(&seg, symbol_len, n_bits, false, false) {
                    Ok(r) => {
                        if best.as_ref().is_none_or(|(_, b)| r.1 > b.1) {
                            best = Some((k, r));
                        }
                    }
                    Err(e) => log::warn!("demod failed at offset {k:+} chirp: {e}"),
                }
            }
            let Some((k_best, (soft, certainty_avg, certainty_std))) = best else {
                log::warn!("frame data {fsk_begin:.3}s..{fsk_end:.3}s not in history buffer, skipped");
                continue;
            };
            if k_best != 0.0 {
                info!(
                    "preamble lock corrected by {k_best:+} chirp period ({:+.3} s): data starts at {:.3}s",
                    k_best * chirp_period,
                    fsk_begin + k_best * chirp_period
                );
            }
            let frame_begin = frame_begin + k_best * chirp_period;

            // Message SNR
            let first_chirp_start = det.time - chirp_len + k_best * chirp_period;
            let noise_end = ((first_chirp_start - MSG_SNR_GUARD_S) * fs_bb).floor() as i64;
            let noise_start = noise_end - (MSG_SNR_NOISE_S * fs_bb).round() as i64;
            let data_start = ((fsk_begin + k_best * chirp_period) * fs_bb).floor() as i64;
            let data_end = data_start + (data_len * fs_bb).round() as i64;
            let (max_msg_snr, avg_msg_snr) = match (history.get(noise_start..noise_end), history.get(data_start..data_end)) {
                (Some(noise), Some(data)) => {
                    let p_noise = mean_power(&noise).max(f64::MIN_POSITIVE);
                    let p_peak = peak_power(&data);
                    let p_data = mean_power(&data);
                    // Below the noise floor gives -inf.
                    (
                        10.0 * ((p_peak - p_noise).max(0.0) / p_noise).log10(),
                        10.0 * ((p_data - p_noise).max(0.0) / p_noise).log10(),
                    )
                }
                _ => {
                    log::warn!("message SNR: noise or data window not in history buffer");
                    (f64::NAN, f64::NAN)
                }
            };
            info!(
                "SNR: sync {:.1} dB | max msg {max_msg_snr:.1} dB | avg msg {avg_msg_snr:.1} dB",
                det.snr
            );
            let hard: Vec<u8> = soft.iter().map(|v| (*v > 127) as u8).collect();

            let decoded = janus_decode_bytes(&soft)?;
            let valid_ascii = decoded.len() == args.msg_len
                && decoded.iter().all(|b| (0x20..=0x7e).contains(b));
            let text = String::from_utf8_lossy(&decoded).to_string();

            // BER before FEC
            let (reference, ref_name) = match &expected_bits {
                Some(e) => (e.clone(), "expected"),
                None => (janus_encode_bytes(&decoded), "re-encoded decode"),
            };
            let wrong = hard.iter().zip(reference.iter()).filter(|(a, b)| a != b).count()
                + hard.len().abs_diff(reference.len());
            let ber = wrong as f32 / n_bits as f32 * 100.0;
            frames_ber.push(ber);
            trace!("BER: {ber} % ({wrong}/{n_bits} bits vs {ref_name})");

            if valid_ascii {
                frames_valid += 1;
                trace!("Got message: {text}");
            } else {
                log::warn!("Decode is not {} printable ASCII chars: {text:?}", args.msg_len);
            }
            if let Some(e) = &expected_bytes {
                if &decoded == e {
                    frames_successful += 1;
                }
            }

            logger.write_record([
                format!(
                    "{}:{:02}",
                    frame_begin.round() as u64 / 60,
                    frame_begin.round() as u64 % 60
                ),
                format!("{frame_begin:.3}"),
                det.snr.to_string(),
                max_msg_snr.to_string(),
                avg_msg_snr.to_string(),
                certainty_avg.to_string(),
                certainty_std.to_string(),
                ber.to_string(),
                ref_name.to_string(),
                valid_ascii.to_string(),
                text,
            ])?;
        }
    }

    for det in &pending {
        log::warn!(
            "preamble at {:.3}s: recording ends before its data does ({:.2} s needed), not decoded",
            det.time,
            data_len
        );
    }

    if expected_bytes.is_some() {
        info!("frames received: {frames_count} ({frames_valid} valid ASCII, {frames_successful} correct decode)");
    } else {
        info!("frames received: {frames_count} ({frames_valid} valid ASCII)");
    }

    info!("Final BER: {frames_ber:?}");

    let plot_len = ((preamble_len + chirp_len) * fs_bb).round() as i64;
    match last_preamble.or_else(|| history.get(soi_start..soi_start + plot_len)) {
        None => info!(
            "Skipping preamble plots: no frame preamble was captured"
        ),
        Some(sig) => {
            plot("plot/test_preamble.png", "h", &sig.abs());
            let mut sig = sig.fftfilt(&chirp_up, mulink_dsp::core::block::filter::ConvShape::FULL)?;
            plot("plot/test_preamble_ir.png", "h", &sig.abs());
            let fft_len = (fs_bb.round() as usize).max(sig.len());
            let append_len = fft_len - sig.len();
            sig.append(&mut vec![Complex::zero(); append_len]);
            let mut s = sig.fft_fwd()?;
            let midpt = s.len() / 2;
            s.rotate_left(midpt);
            let mut s = s.abs();
            s.iter_mut()
                .for_each(|v| *v = 20.0 * f32::log10(v.clamp(0.01, f32::INFINITY)));
            plot("plot/test_preamble_ir_spect.png", "h", &s);
        }
    }
    // Spectrogram of the last 60 s of baseband.
    spectrogram(
        "plot/record.hist.wav.png",
        history.full(),
        256,
        128 + 64 + 32,
        true,
        Some(-100.0),
    );
    Ok(())
}
