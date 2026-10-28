use core::f64;
use std::sync::OnceLock;

use log::trace;
use num::{cast::AsPrimitive, Complex};
use plotters::prelude::*;
use plotters::{
    chart::ChartBuilder,
    prelude::{
        BitMapBackend, DerivedColorMap, DiscreteRanged, IntoDrawingArea, IntoLinspace, PathElement,
    },
    series::LineSeries,
    style::{RGBColor, RED, WHITE},
};
use rkyv::rancor::Trace;

use crate::{
    core::{
        block::fft::{FftInst, RustFftInst},
        r#gen::fir::hamming,
    },
    prelude::{init_tracing, Signal, SignalType},
};

static GLOBAL_REFERENCE_LVL_DB: OnceLock<f64> = OnceLock::<f64>::new();

// use crate::{
//     fft::{self, fft_complex},
//     fir::hamming,
//     signal::LegacySignal,
// };

pub fn init_global_reference(reference: f64) {
    GLOBAL_REFERENCE_LVL_DB.set(reference).unwrap();
}
pub fn get_global_reference() -> f64 {
    *GLOBAL_REFERENCE_LVL_DB.get_or_init(|| 0.0)
}

fn color_from_intensity<T: AsPrimitive<f64>>(intensity: T, reference: f64) -> RGBColor {
    let intensity: f64 = intensity.as_() * 10.0_f64.powf(-reference / 20.0);
    // RGBColor(
    //     (intensity / 10000.0) as u8,
    //     (intensity / 1000.0) as u8,
    //     (intensity / 100.0) as u8,
    // )
    let cm = DerivedColorMap::new(&[BLACK, BLUE, GREEN, YELLOW, RED]);
    cm.get_color(intensity / 150.0)
}
fn color_from_intensity_db<T: AsPrimitive<f64>>(intensity: T, reference: f64) -> RGBColor {
    let intensity: f64 = intensity.as_();
    let intensity = 20.0 * f64::log10(intensity) - reference;
    // let intensity = intensity.clamp(0.0, 150.0);
    let intensity = if intensity.is_finite() {
        intensity
    } else {
        0.0
    };
    // println!("{intensity}");
    let cm = DerivedColorMap::new(&[BLACK, BLUE, GREEN, YELLOW, RED]);
    cm.get_color(intensity / 150.0)
}

pub fn spectrogram<T: SignalType>(
    filename: &str,
    signal: Signal<T>,
    window: usize,
    noverlap: usize,
    log: bool,
    reference: Option<f64>,
) {
    let reference = reference.unwrap_or(get_global_reference());

    //let signal = sig.samples.complex();

    //let signal: Vec<_> = signal.iter().map(|x| x / 4096.0).collect();

    let spect_window: Vec<T> = hamming(window).iter().map(|x| T::from_f32_(*x)).collect();

    let chunk_size = window - noverlap;

    let chunks = signal.chunks_exact(chunk_size);

    let mut collected_chunks: Vec<Vec<Complex<T>>> = chunks.map(|f| f.to_vec()).collect();
    collected_chunks.push(vec![
        Complex {
            re: T::default(),
            im: T::default()
        };
        chunk_size
    ]);
    let nwindows = window / (chunk_size + 1) + 1;
    let chunk_windows = collected_chunks.windows(nwindows);
    let nchunks = chunk_windows.len();

    let margin: u32 = 150;
    let margin_lr: u32 = 150;

    let chart_offset_x = margin + margin_lr;
    let chart_offset_y = margin;

    let root = BitMapBackend::new(
        filename,
        (nchunks as u32 + margin + margin_lr, window as u32 + margin),
    )
    .into_drawing_area();
    root.fill(&WHITE).unwrap();

    let fft = RustFftInst::<T>::new(window);

    for (idx_chunk, chunk_window) in chunk_windows.enumerate() {
        let mut chunk = vec![
            Complex {
                re: T::default(),
                im: T::default()
            };
            window
        ];
        for idx_window in 0..nwindows {
            chunk[(idx_window * chunk_size)..usize::min((idx_window + 1) * chunk_size, window)]
                .copy_from_slice(
                    &chunk_window[idx_window][0..if (idx_window + 1) * chunk_size > window {
                        (idx_window + 1) * chunk_size - window
                    } else {
                        chunk_size
                    }],
                );
        }

        chunk
            .iter_mut()
            .enumerate()
            .for_each(|(idx, x)| *x = *x * Complex::from(spect_window[idx]));

        //chunk[0..chunk_size].copy_from_slice(&chunk_window[0]);
        //chunk[chunk_size..].copy_from_slice(&chunk_window[1][0..(window - chunk_size)]);
        // let spect = fft_complex(&chunk, true);
        let mut spect = chunk.clone();

        fft.fft_fwd(&mut spect).unwrap();
        let spect: Vec<T> = spect.iter().cloned().map(Complex::norm).collect();
        for (idx_bin, bin) in spect.iter().enumerate() {
            let idx_bin = if idx_bin > (window - 1) / 2 {
                idx_bin - window / 2
            } else {
                idx_bin + window / 2
            };
            //let bin = if log { *1000.0 } else { *bin };
            let color = if log {
                color_from_intensity_db(*bin, reference)
            } else {
                color_from_intensity(*bin, reference)
            };
            root.draw_pixel(
                (
                    (idx_chunk as u32 + chart_offset_x / 2) as i32,
                    fft.len() as i32 - 1 - (idx_bin as u32) as i32 + chart_offset_y as i32 / 2,
                ),
                &color,
            )
            .unwrap();
        }
    }

    let style = TextStyle::from(("sans-serif", 20).into_font()).color(&BLACK);

    //let text_size = root.estimate_text_size("0.0 Hz", &style)

    let draw_vtick = |y_offset: i32, text: &str| {
        root.draw_text(
            text,
            &style,
            (5, (window as i32 + margin as i32 - (15) + y_offset) / 2),
        )
        .unwrap();

        for i in (margin + margin_lr - 45)..(margin + margin_lr) {
            root.draw_pixel(
                (
                    (i / 2) as i32,
                    (window as i32 + margin as i32 + y_offset) / 2,
                ),
                &BLACK,
            )
            .unwrap();
        }
    };
    let draw_hz_tick = |y_offset: i32| {
        let freq = -y_offset as f64 / window as f64 / 2.0 * signal.sample_rate;
        let text = if !(-1000.0..=1000.0).contains(&freq) {
            format!("{:.1} kHz", freq / 1000.0)
        } else {
            format!("{:.1} Hz", freq)
        };
        draw_vtick(y_offset, &text);
    };

    let n_divisions = window / 100 + 1;
    draw_hz_tick(0);
    for idx in 0..=n_divisions {
        let offset = (window) as f64 * (idx as f64 / n_divisions as f64);
        draw_hz_tick(offset as i32);
        draw_hz_tick(-offset as i32);
    }

    /*draw_vtick(0, "0.0 Hz");
    draw_vtick(
        window as i32 / 5,
        &format!("{:.1} Hz", 1.0 / (window / 5) as f64 * sig.sample_rate),
    );*/

    root.present().expect("");

    /*let mut cc = ChartBuilder::on(&root)
        .margin(5)
        .set_all_label_area_size(50)
        .caption("hello", ("sans-serif", 40))
        .build_cartesian_2d(0..nchunks, 0..window)
        .unwrap();

    //println!("processing X0 for plot");

    cc.configure_mesh()
        .x_labels(20)
        .y_labels(10)
        .disable_mesh()
        .x_label_formatter(&|v| format!("{:.1}", v))
        .y_label_formatter(&|v| format!("{:.1}", v))
        .draw()
        .unwrap();*/
}

#[test]
fn test_spectrogram() -> anyhow::Result<()> {
    init_tracing();
    trace!("reading file");
    let sig = Signal::<f32>::read_wav(
        "/home/brodie/Developer/mulink-dsp-legacy/data/hello_tx_close.wav",
    )?;
    trace!("building spect");
    spectrogram(
        "plot/test/test_spectrogram/tx.png",
        sig,
        1024,
        512,
        true,
        Some(-120.0),
    );
    trace!("reading file");
    let sig = Signal::<f32>::read_wav(
        "/home/brodie/Developer/mulink-dsp-legacy/data/hello_tx_close_baseband.wav",
    )?;
    trace!("building spect");
    spectrogram(
        "plot/test/test_spectrogram/tx_bb.png",
        sig,
        512,
        256 + 128 + 64 + 32,
        true,
        Some(-120.0),
    );

    Ok(())
}
