//! Offline decoder for baseband recordings of JANUS-encoded hop-FSK frames (HiFiBerry
//! receiver).
//!
//! Expects the 24 kHz complex baseband WAVs that rx_simple logs, so there is no
//! down-conversion step. Frames are found by pairing a 3 up-chirp preamble with the
//! 3 down-chirp postamble after it; the message length comes from the gap between them.
//! Uses 10 ms symbols and the original fixed-timing preamble detector.
//!
//! Output:
//! - `asa_logs/<file>.csv`, one row per frame of the expected length (time, sync SNR,
//!   symbol certainty, BER against the known test message)
//! - preamble diagnostic plots and a spectrogram of the last 60 s, in `plot/`
use core::f32;
use std::{collections::VecDeque, fs::create_dir_all, path::Path};

use clap::Parser;
use itertools::Itertools;
use log::{info, trace};
use mulink_dsp::{
    core::{
        block::{
            buffer::HistoryBuffer,
            downconv::Downconv,
            fft::{FftInst, RustFftInst},
            filter::{ConvShape, Filter},
            refragment::{self, Refragmenter},
            windower::BrickWallWindower,
        },
        r#gen::{
            fir::{fir_lpf, hamming},
            noise::noise_complex,
        },
        signal::{Amplitude, RealSignal},
    },
    plot::{
        spectrum::spectrogram,
        time::{plot, plot_complex},
    },
    prelude::{init_tracing, FromVec, Signal, SignalType},
    protocol::mulink::{
        chip_pattern::MuLinkHopPattern,
        fsk::{demod, demod_extra},
        preamble::{preamble_detect_kern, ChirpDirection, ControlDetection, PreambleDetection},
    },
    robust::codec::{janus_decode_bytes, janus_encode_bytes},
};
use num::{
    traits::{sign, ConstZero},
    Complex, Zero,
};
use rand::Fill;

/// Command-line options.
#[derive(clap::Parser)]
struct Args {
    /// WAV file to decode (24 kHz complex baseband, as written by rx_simple's logger).
    path: String,
}

fn main() -> anyhow::Result<()> {
    init_tracing()?;
    // Reference tones are tapered the same way the transmitter tapers them.
    let window_chips = true;
    let s_path = &Args::parse().path;

    let mut signal = Signal::<f32>::read_wav(s_path)?;
    let fs = signal.sample_rate;

    // Protocol parameters. Note the 10 ms symbols, unlike the 20 ms default elsewhere.
    let symbol_len: f64 = 0.01;
    let chirp_len: f64 = 0.02;
    let silence_len: f64 = 0.1;

    // The file is already at baseband, so there is no down-conversion or decimation.
    let down_ratio = 1.0;
    let fs_bb = fs / down_ratio;

    let samples_per_symbol = (fs * symbol_len).round() as usize;

    // 13 hops, each a pair of tones (bit 0 / bit 1) spread across 65-75 kHz.
    let hop_tab = MuLinkHopPattern::new(13, 70000.0, 10000.0, fs);

    // Passband reference tones for every hop, built the same way as the transmitter's.
    let mut chip_bank = Vec::<(RealSignal<f32>, RealSignal<f32>)>::new();

    for hop in hop_tab.tab.iter() {
        // Hop frequencies are stored normalised to Nyquist; convert to rad/s.
        let w0 = hop.f0 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let w1 = hop.f1 as f32 * (fs as f32) / 2.0 * 2.0 * f32::consts::PI;
        let s0 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w0));
        let s1 = RealSignal::from_function(fs, samples_per_symbol, |t| f32::sin(t as f32 * w1));

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
    let chirp_up =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 65000.0, 75000.0);

    let chirp_down =
        RealSignal::<f32>::chirp_real(fs, (fs * chirp_len).round() as usize, 75000.0, 65000.0);

    let silence = RealSignal::<f32>::from_vec(fs, vec![0.0; (fs * silence_len).round() as usize]);

    // Matched filters for the baseband up- and down-chirps (time-reversed conjugates).
    let mut chirp_up =
        Signal::<f32>::chirp_complex(fs_bb, (chirp_len * fs_bb).round() as usize, -5000.0, 5000.0)
            .conj();
    chirp_up.reverse();
    let mut chirp_down =
        Signal::<f32>::chirp_complex(fs_bb, (chirp_len * fs_bb).round() as usize, 5000.0, -5000.0)
            .conj();
    chirp_down.reverse();
    let mut filt_chirp_up = Filter::<f32>::new(chirp_up.clone())?;
    let mut filt_chirp_down = Filter::<f32>::new(chirp_down)?;

    // Matched-filter output is cut into overlapping windows one chirp group (3 chirps +
    // 3 silences) long, so every group falls completely inside at least one window.
    let mut windower_up = BrickWallWindower::new(
        fs_bb,
        (chirp_len * fs_bb * 3.0 + silence_len * fs_bb * 3.0) as usize,
    );

    let mut windower_down = BrickWallWindower::new(
        fs_bb,
        (chirp_len * fs_bb * 3.0 + silence_len * fs_bb * 3.0) as usize,
    );

    // The file is fed through in 512-sample blocks to mimic a live stream.
    let mut refrag = Refragmenter::<f32>::new(fs, 512);

    // Last 60 s of baseband, so a frame's data can be pulled out once its postamble arrives.
    let mut history = HistoryBuffer::<f32>::new(fs_bb, 60.0);

    refrag.push(&mut signal);

    // Recent up- and down-chirp group detections, oldest first.
    let mut detections = Vec::<PreambleDetection>::new();

    // Sample index of the first preamble seen, used for the diagnostic plots at the end.
    let mut soi_start = 0;

    // The message the test transmitter sends. Used for BER and the correct-decode count.
    let expected_msg = "The quick brown fox jumped over the lazy old dog";

    let true_bytes =
        janus_encode_bytes(&"The quick brown fox jumped over the lazy old dog".as_bytes());

    trace!("tru_len: {}", true_bytes.len());

    let mut frames_count = 0;
    let mut frames_successful = 0;
    let mut frames_ber = Vec::new();

    create_dir_all("asa_logs")?;
    let file_name = Path::new(&s_path).file_name().unwrap().to_str().unwrap();
    let mut logger = csv::Writer::from_path(format!("asa_logs/{file_name}.csv"))?;
    logger.write_record(["Time (s)", "SNR", "Certainty", "Certainty STDDEV", "BER%"])?;

    for frag in &mut refrag {
        let signal = frag;

        let Ok(_) = history.write(&signal) else {
            panic!("FAILED TO WRITE TO HIST BUF");
        };

        // Look for up-chirp groups (preambles).
        if let Some(upchirp_ir) = filt_chirp_up.process(signal.clone()) {
            windower_up.push(&mut upchirp_ir.clone());

            for window in &mut windower_up {
                let Ok(detection) = preamble_detect_kern(
                    &window,
                    Amplitude::Log(-9.0),
                    (chirp_len * fs_bb * 0.75) as usize,
                ) else {
                    continue;
                };
                if soi_start == 0 {
                    soi_start = detection.idx;
                }
                trace!("UP: {detection:?}");
                // Merge into the detection list: anything within 0.2 s is the same group, so
                // keep whichever is stronger. Detections older than 45 s are dropped.
                let mut new_detections = Vec::<PreambleDetection>::new();
                let mut inserted = false;
                for det in detections.iter() {
                    if (detection.time - 0.2..detection.time + 0.2).contains(&det.time) {
                        if detection.amplitude >= det.amplitude {
                            if !inserted {
                                inserted = true;
                                new_detections.push(PreambleDetection {
                                    direction: ChirpDirection::Up,
                                    detection,
                                });
                            }
                        } else if !inserted {
                            inserted = true;
                            new_detections.push(*det);
                        }
                    } else if det.time > (detection.time - 45.0) {
                        new_detections.push(*det);
                    }
                }
                if !inserted {
                    new_detections.push(PreambleDetection {
                        direction: ChirpDirection::Up,
                        detection,
                    });
                }
                detections = new_detections;
            }
        };
        // Look for down-chirp groups (postambles), merged into the same list the same way.
        if let Some(downchirp_ir) = filt_chirp_down.process(signal.clone()) {
            windower_down.push(&mut downchirp_ir.clone());

            for window in &mut windower_down {
                let Ok(detection) = preamble_detect_kern(
                    &window,
                    Amplitude::Log(-9.0),
                    (chirp_len * fs_bb * 0.75) as usize,
                ) else {
                    continue;
                };
                let mut new_detections = Vec::<PreambleDetection>::new();
                let mut inserted = false;
                for det in detections.iter() {
                    if (detection.time - 0.2..detection.time + 0.2).contains(&det.time) {
                        if detection.amplitude >= det.amplitude {
                            if !inserted {
                                inserted = true;
                                new_detections.push(PreambleDetection {
                                    direction: ChirpDirection::Down,
                                    detection,
                                });
                            }
                        } else if !inserted {
                            inserted = true;
                            new_detections.push(*det);
                        }
                    } else if det.time > (detection.time - 45.0) {
                        new_detections.push(*det);
                    }
                }
                if !inserted {
                    new_detections.push(PreambleDetection {
                        direction: ChirpDirection::Down,
                        detection,
                    });
                }
                detections = new_detections;

                trace!("DOWN: {detection:?}");
            }
        };

        // A frame is ready once the newest detection is a strong postamble that is at least
        // 1 s old (so nothing stronger nearby can still replace it).
        if let Some(detection) = {
            let mut detection = None;

            for det in detections.iter().rev() {
                if det.direction != ChirpDirection::Down {
                    detection = None;
                    break;
                }
                if det.idx < (signal.time - (fs_bb * 1.0) as i64) && det.snr >= 12.0 {
                    detection = Some(det);
                    break;
                }
            }
            detection
        } {
            // Pair it with the latest strong preamble that isn't the postamble itself and
            // whose amplitude is within a plausible range of it (0.05x to 20x).
            let mut closest: Option<usize> = None;
            for (idx, det) in detections.iter().enumerate() {
                if !(det.direction == ChirpDirection::Up) {
                    continue;
                }
                if (det.time - detection.time).abs() <= 0.2
                    || !(detection.amplitude * 0.05..detection.amplitude * 20.0)
                        .contains(&det.amplitude)
                    || det.snr < 12.0
                {
                    continue;
                }
                closest = Some(idx);
            }
            if let Some(closest) = closest {
                let closest = detections[closest];
                // The data sits between the preamble and postamble; its length (and so the
                // message length) comes from the gap between them, not from a fixed setting.
                let frame_begin = closest.time - 0.120;
                let frame_end = detection.time + 0.340;
                let fsk_begin = frame_begin + 0.460;
                let fsk_end = frame_end - 0.460;
                // Number of data symbols in the gap.
                let bytes = (fsk_end - fsk_begin) / symbol_len;
                info!(
                "FRAME DETECT: {:.03}s..{:.03}s @ snr = {:0.3} ({bytes:.03} symbols/{:.03} bytes)",
                frame_begin,
                frame_end,
                closest.snr.min(detection.snr),
                bytes / 8.0
            );
                let (msg, certainty_avg, certainty_std) = demod_extra(
                    &history
                        .get(((fsk_begin * fs_bb) as i64)..(fsk_end * fs_bb) as i64)
                        .unwrap(),
                    symbol_len,
                    false,
                    false,
                )
                .unwrap();
                trace!("{}", msg.len());
                let len = msg.len();
                // BER before FEC, only possible when the frame is the expected length.
                if msg.len() == true_bytes.len() {
                    let mut wrong = 0;
                    for (_idx, (obs, tru)) in msg.iter().zip(true_bytes.iter()).enumerate() {
                        wrong += ((*obs > 127) as u8 != *tru) as u8;

                        if (*obs > 127) as u8 != *tru {
                            trace!("[{_idx}] {:.03} : {}", *obs as f32 / 256.0, *tru);
                        }
                    }
                    frames_ber.push(wrong as f32 / len as f32 * 100.0);
                    trace!("BER: {} %", wrong as f32 / len as f32 * 100.0);

                    // First column is the frame start as m:ss, for lining up with the recording.
                    logger.write_record([
                        format!(
                            "{}:{:02}",
                            frame_begin.round() as u64 / 60,
                            frame_begin.round() as u64 % 60
                        ),
                        closest.snr.min(detection.snr).to_string(),
                        certainty_avg.to_string(),
                        certainty_std.to_string(),
                        (wrong as f32 / len as f32 * 100.0).to_string(),
                    ])?;
                } else {
                    trace!("MSG LEN MISMATCH");
                }
                // Deinterleave + Viterbi decode back to the original bytes.
                let Ok(msg) = janus_decode_bytes(&msg) else {
                    detections.clear();
                    continue;
                };

                if let Ok(msg) = String::from_utf8(msg) {
                    trace!("Got message: {msg}");
                    if msg == expected_msg {
                        frames_successful += 1;
                    }
                }

                frames_count += 1;

                // Start fresh for the next frame.
                detections.clear();
            }
        }
    }

    println!("\n\n{:#?}", detections);

    info!("frames received: {frames_count} ({frames_successful} correct decode)");

    info!("Final BER: {frames_ber:?}");

    // Diagnostic plots of the first preamble: raw magnitude, matched-filter output, and the
    // spectrum of the matched-filter output. Assumes 24 kHz baseband and that the preamble
    // is still inside the 60 s history buffer.
    let soi_start = soi_start as usize;
    plot(
        "plot/test_preamble.png",
        "h",
        &history.full().abs()[soi_start..(soi_start + (0.380 * 24000.0) as usize)],
    );
    let mut sig = Signal::from_vec(
        24000.0,
        history.full()[soi_start..(soi_start + (0.380 * 24000.0) as usize)].to_vec(),
    );
    let mut sig = sig.fftfilt(&chirp_up, mulink_dsp::core::block::filter::ConvShape::FULL)?;
    plot("plot/test_preamble_ir.png", "h", &sig.abs());
    // Zero-pad to 1 s of samples so the FFT bins are 1 Hz apart.
    let append_len = 24000 - sig.len();
    sig.append(&mut vec![Complex::zero(); append_len]);
    let mut s = sig.fft_fwd()?;
    // Put 0 Hz in the middle of the plot, then convert to dB.
    let midpt = s.len() / 2;
    s.rotate_left(midpt);
    let mut s = s.abs();
    s.iter_mut()
        .for_each(|v| *v = 20.0 * f32::log10(v.clamp(0.01, f32::INFINITY)));

    plot("plot/test_preamble_ir_spect.png", "h", &s);
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
