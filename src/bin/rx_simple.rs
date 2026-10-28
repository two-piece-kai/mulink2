//! Live receiver and decoder for JANUS-encoded hop-FSK frames, from the default audio input.
//! The settings below must match the transmitter
use clap::Parser;
use log::{info, trace, warn};
use mulink_dsp::{
    backend::cpal::CpalBackend,
    core::{
        block::{
            buffer::HistoryBuffer, downconv::Downconv, filter::Filter, refragment::Refragmenter,
            windower::BrickWallWindower,
        },
        signal::Amplitude,
    },
    logging::init_bb_wav_logger,
    prelude::{init_tracing, Signal},
    protocol::mulink::{
        fsk::demod_extra_bits,
        preamble::{preamble_detect_kern_timed, ControlDetection},
    },
    robust::codec::{janus_decode_bytes, janus_encode_bytes},
};
use std::fs::create_dir_all;

/// Command-line options.
#[derive(Clone, Debug, Parser)]
pub struct RxArgs {
    /// Expected message length in bytes (ASCII characters) BEFORE FEC encoding.
    /// Must match the length of the transmitter's --msg.
    #[arg(short = 'n', long, default_value_t = 32)]
    msg_len: usize,
    /// Optional: the message being sent.
    #[arg(short, long)]
    expected: Option<String>,
    /// Number of channels to open on the input device.
    #[arg(long, default_value_t = 2)]
    channels: usize,
    /// Which of those channels the hydrophone is on (0 = first).
    #[arg(long, default_value_t = 0)]
    channel: usize,
    /// Minimum preamble sync SNR (dB) needed to start decoding a frame.
    #[arg(long, default_value_t = 12.0)]
    min_snr: f64,
    /// Don't save the baseband recordings to mulink-logs
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_bb_log: bool,
}


const MSG_SNR_NOISE_S: f64 = 0.1;
const MSG_SNR_GUARD_S: f64 = 0.01;

/// Mean power (|x|^2) of a baseband segment.
fn mean_power(sig: &Signal<f32>) -> f64 {
    sig.iter().map(|v| v.norm_sqr() as f64).sum::<f64>() / sig.len().max(1) as f64
}

/// Peak power (max |x|^2, i.e. max amplitude squared) of a baseband segment.
fn peak_power(sig: &Signal<f32>) -> f64 {
    sig.iter().map(|v| v.norm_sqr() as f64).fold(0.0, f64::max)
}

fn main() -> anyhow::Result<()> {
    init_tracing()?;
    let args = RxArgs::parse();
    info!("RX with args: {args:?}");

    // Capture rate. The baseband logger assumes the 24 kHz this gives after decimation.
    let fs = 192000.0;

    // Protocol parameters. These must match the transmitter.
    let symbol_len: f64 = 0.02;
    let chirp_len: f64 = 0.02;
    let silence_len: f64 = 0.1;
    let n_chirps: f64 = 3.0; // chirps in the preamble
    let fc: f64 = 70000.0; // centre of the FSK hop band (Hz)
    let bw: f64 = 10000.0; // FSK hop band and chirp sweep width (Hz)

    // Baseband low-pass passes the signal band plus 20% margin, then decimate to about
    // 2x that passband: 192 kHz / 8 = 24 kHz.
    let downconv_bw = bw * 1.2;
    let down_ratio = (fs / (2.0 * downconv_bw)).floor().max(1.0);
    let fs_bb = fs / down_ratio;

    // A matched-filter peak marks the END of a chirp; detector reports the earliest of
    // the three peaks
    let chirp_period = chirp_len + silence_len;
    let preamble_len = n_chirps * chirp_period;
    info!(
        "fs = {fs} Hz, down_ratio = {down_ratio}, fs_bb = {fs_bb:.1} Hz, symbol_len = {symbol_len} s"
    );

    // Mix the 70 kHz band down to 0 Hz, low-pass, and decimate to fs_bb.
    let mut downconv = Downconv::<f32>::new(fs, fc, downconv_bw, 128, down_ratio as usize)?;

    // Up-chirp matched filter
    let mut chirp_up =
        Signal::<f32>::chirp_complex(fs_bb, (chirp_len * fs_bb).round() as usize, -bw / 2.0, bw / 2.0)
            .conj();
    chirp_up.reverse();
    let mut filt_chirp_up = Filter::<f32>::new(chirp_up)?;


    let mut windower_up = BrickWallWindower::new(fs_bb, (preamble_len * fs_bb) as usize);

    let mut refrag = Refragmenter::<f32>::new(fs, 8192);

    // Last 60 s of baseband, so the data after a preamble can be pulled out once it arrives.
    let mut history = HistoryBuffer::<f32>::new(fs_bb, 60.0);

    // Expected frame size, from the fixed message length.
    let n_bits = 16 * (args.msg_len + 1); // 8 bits/byte + 8 flush bits, rate-1/2 code
    let data_len = n_bits as f64 * symbol_len;

    let postamble_len = preamble_len;
    let dedupe_s = 0.2; // detections closer than this are the same preamble
    // Wait this long past the end of the data before demodulating, so the last symbol is
    // in the history buffer
    let settle_s = 2.0 * symbol_len + 0.2 + chirp_period;
    info!(
        "expecting {} byte message -> {n_bits} symbols, {data_len:.2} s of data per frame",
        args.msg_len
    );

    let expected_bytes = args.expected.as_ref().map(|m| m.as_bytes().to_vec());
    if let Some(e) = &expected_bytes {
        if e.len() != args.msg_len {
            warn!(
                "--expected is {} bytes but --msg-len is {}; correct-decode count will be 0",
                e.len(),
                args.msg_len
            );
        }
    }
    let expected_bits = expected_bytes.as_ref().map(|e| janus_encode_bytes(e));

    // Preambles found but whose data hasn't fully arrived yet.
    let mut pending = Vec::<ControlDetection>::new();
    // Ignore new detections until this time (end of the current frame and its postamble).
    let mut holdoff_until = f64::NEG_INFINITY;

    let mut frames_count = 0;
    let mut frames_valid = 0;
    let mut frames_successful = 0;

    // Background writer that saves baseband to a new WAV every 60 s.
    let bb_logger = if args.no_bb_log {
        None
    } else {
        Some(init_bb_wav_logger::<f32>(60000.0)?)
    };

    // One CSV per run, flushed after every frame so nothing is lost when the program is stopped.
    let t0 = chrono::Local::now();
    create_dir_all("asa_logs")?;
    let csv_path = format!("asa_logs/live_{}.csv", t0.format("%Y%m%d_%H%M%S"));
    let mut logger = csv::Writer::from_path(&csv_path)?;
    logger.write_record([
        "Clock time",
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
    logger.flush()?;
    info!("logging frames to {csv_path}");

    let rcx = CpalBackend::capture(args.channels, args.channel, fs);
    info!("listening... (Ctrl+C to stop)");

    for mut block in rcx {
        refrag.push(&mut block);

        for frag in &mut refrag {
            let Some(signal) = downconv.process(frag) else {
                continue;
            };

            if let Some(bb) = &bb_logger {
                if bb.send(signal.clone()).is_err() {
                    warn!("baseband logger stopped; recordings are no longer being saved");
                }
            }

            let Ok(_) = history.write(&signal) else {
                panic!("FAILED TO WRITE TO HIST BUF");
            };

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
                    if detection.snr < args.min_snr || detection.time < holdoff_until {
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
                        Err(e) => warn!("demod failed at offset {k:+} chirp: {e}"),
                    }
                }
                let Some((k_best, (soft, certainty_avg, certainty_std))) = best else {
                    warn!("frame data {fsk_begin:.3}s..{fsk_end:.3}s not in history buffer, skipped");
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

                // Message SNR, using the alignment that won above.
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
                        warn!("message SNR: noise or data window not in history buffer");
                        (f64::NAN, f64::NAN)
                    }
                };
                info!(
                    "SNR: sync {:.1} dB | max msg {max_msg_snr:.1} dB | avg msg {avg_msg_snr:.1} dB",
                    det.snr
                );
                let hard: Vec<u8> = soft.iter().map(|v| (*v > 127) as u8).collect();

                let decoded = match janus_decode_bytes(&soft) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("JANUS decode failed: {e}");
                        continue;
                    }
                };
                let valid_ascii = decoded.len() == args.msg_len
                    && decoded.iter().all(|b| (0x20..=0x7e).contains(b));
                let text = String::from_utf8_lossy(&decoded).to_string();

                // BER before FEC. With --expected: against what was sent. Without: against the
                // re-encoded decode
                let (reference, ref_name) = match &expected_bits {
                    Some(e) => (e.clone(), "expected"),
                    None => (janus_encode_bytes(&decoded), "re-encoded decode"),
                };
                let wrong = hard.iter().zip(reference.iter()).filter(|(a, b)| a != b).count()
                    + hard.len().abs_diff(reference.len());
                let ber = wrong as f32 / n_bits as f32 * 100.0;

                if valid_ascii {
                    frames_valid += 1;
                    info!("Got message: {text}");
                } else {
                    warn!("Decode is not {} printable ASCII chars: {text:?}", args.msg_len);
                }
                if let Some(e) = &expected_bytes {
                    if &decoded == e {
                        frames_successful += 1;
                    }
                }

                let clock = t0 + chrono::TimeDelta::milliseconds((frame_begin * 1000.0) as i64);
                logger.write_record([
                    clock.format("%H:%M:%S%.3f").to_string(),
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
                logger.flush()?;

                // Running totals
                if expected_bytes.is_some() {
                    info!("BER {ber:.2}% | frames so far: {frames_count} ({frames_valid} valid ASCII, {frames_successful} correct)");
                } else {
                    info!("BER {ber:.2}% | frames so far: {frames_count} ({frames_valid} valid ASCII)");
                }
            }
        }
    }

    warn!("audio capture ended");
    Ok(())
}
