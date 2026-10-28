use std::{
    sync::{
        mpsc::{self, channel, Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, StreamConfig, StreamError,
};
use log::{trace, warn};
use num::Complex;

use crate::{
    core::{signal::RealSignal, stream::AudioStream},
    prelude::{FromVec, Signal},
};

pub struct CpalBackend {
    //rx_stream: Arc<AudioStream<f32>>,
}
impl CpalBackend {
    pub fn duplex(channels: usize, channel_select: usize, sample_rate: f64) -> CpalBackend {
        //let rx_stream = Arc::new(AudioStream::<f32>::new());
        CpalBackend {}
    }
    pub fn capture(
        channels: usize,
        channel_select: usize,
        sample_rate: f64,
    ) -> Receiver<Signal<f32>> {
        //let rx_stream = Arc::new(AudioStream::<f32>::new());
        let (audio_tx, audio_rx) = channel();
        Self::audio_capture_watchdog(channel_select, channels, sample_rate, audio_tx);
        //CpalBackend {}
        audio_rx
    }
    pub fn playback(
        channels: usize,
        // channel_select: usize,
        sample_rate: f64,
    ) -> Sender<RealSignal<f32>> {
        //let rx_stream = Arc::new(AudioStream::<f32>::new());
        let (audio_tx, audio_rx) = channel();
        Self::audio_playback_watchdog(channels, sample_rate, audio_rx);
        audio_tx
    }
    fn audio_capture_watchdog(
        channel_select: usize,
        channels: usize,
        sample_rate: f64,
        stream: Sender<Signal<f32>>,
    ) {
        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate.round() as u32),
            buffer_size: cpal::BufferSize::Fixed(8192),
        };
        let (raw_audio_tx, raw_audio_rx) = mpsc::channel::<Vec<i32>>();
        std::thread::spawn(move || {
            Self::run_audio_capture_conversion(
                channel_select,
                channels,
                sample_rate,
                raw_audio_rx,
                stream,
            );
        });
        std::thread::spawn(move || loop {
            if let Err(err) = Self::run_audio_capture(raw_audio_tx.clone(), config.clone()) {
                trace!("{err}");
            };
        });
    }
    fn run_audio_capture_conversion(
        channel_select: usize,
        channels: usize,
        sample_rate: f64,
        raw_audio_rx: Receiver<Vec<i32>>,
        stream: Sender<Signal<f32>>,
    ) {
        trace!("Audio conversion thread started");
        // let stream = stream.lock().unwrap();
        //let mut sig = Signal::from_vec(sample_rate, vec![0.0_f32; 8192]);
        for block in raw_audio_rx {
            let mut sig = Signal::from_vec(sample_rate, vec![0.0_f32; block.len() / channels]);
            for (idx, sample) in block.chunks_exact(channels).enumerate() {
                sig[idx] = Complex {
                    re: (sample[channel_select]) as f32, // / 8_388_607.0,
                    im: 0.0,
                };
            }
            if let Err(e) = stream.send(sig.clone()) {
                trace!("{e}");
            };
        }
        warn!("Audio conversion thread shutting down");
    }
    fn run_audio_capture(stream: Sender<Vec<i32>>, config: StreamConfig) -> anyhow::Result<()> {
        let (error_channel_tx, error_channel_rx) = mpsc::channel::<anyhow::Error>();
        let error_channel_tx = Arc::new(Mutex::new(error_channel_tx));
        let error_channel_data_cb_tx = error_channel_tx.clone();

        let data_callback = move |data: &[i32], _info: &InputCallbackInfo| {
            if let Err(err) = stream.send(data.to_vec()) {
                error_channel_data_cb_tx
                    .lock()
                    .unwrap()
                    .send(anyhow!(err))
                    .unwrap();
            };
        };
        let error_callback = move |err: StreamError| {
            error_channel_tx.lock().unwrap().send(anyhow!(err)).unwrap();
        };

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(anyhow!("Failed to get default input device"))?;

        let stream = device.build_input_stream(
            &config,
            data_callback,
            error_callback,
            Some(Duration::from_secs(30)),
        )?;
        stream.play()?;

        let error = error_channel_rx.recv()?;
        trace!("{error}");
        drop(stream);
        Err(error)
    }

    fn audio_playback_watchdog(
        channels: usize,
        // _channel_select: usize,
        sample_rate: f64,
        stream: Receiver<RealSignal<f32>>,
    ) {
        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate.round() as u32),
            buffer_size: cpal::BufferSize::Fixed(8192),
        };
        // let (raw_audio_tx, raw_audio_rx) = mpsc::channel::<Vec<i32>>();
        // std::thread::spawn(move || {
        //     Self::run_audio_capture_conversion(
        //         channel_select,
        //         channels,
        //         sample_rate,
        //         raw_audio_rx,
        //         stream,
        //     );
        // });
        let rx = Arc::new(Mutex::new(stream));
        std::thread::spawn(move || loop {
            if let Err(err) = Self::run_audio_playback(rx.clone(), config.clone()) {
                trace!("{err}");
            };
        });
    }
    fn run_audio_playback(
        stream: Arc<Mutex<Receiver<RealSignal<f32>>>>,
        config: StreamConfig,
        // channel_select: usize,
    ) -> anyhow::Result<()> {
        let (error_channel_tx, error_channel_rx) = mpsc::channel();
        let error_channel_tx = Arc::new(Mutex::new(error_channel_tx));

        let audio_out_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        let playback_cb_condvar = error_channel_tx.clone();
        let error_cb_condvar = error_channel_tx.clone();
        let error_cb = move |err: StreamError| {
            println!("{}", err);
            error_cb_condvar.lock().unwrap().send(anyhow!(err)).unwrap();
        };

        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        let stream = device
            .build_output_stream_raw(
                &config,
                cpal::SampleFormat::I32,
                move |data, _info| {
                    let mut audio_out = audio_out_buffer.lock().unwrap();

                    //let mut buf = [0_u8; 4096 * size_of::<ml_float>()];
                    // Receive new samples from client
                    //let result = tcp_stream.lock().unwrap().read(&mut buf);
                    let result = stream.lock().unwrap().try_recv();
                    if let Ok(mut audio) = result {
                        //info!("CPAL BACKEND GOT AUDIO: {}", audio.len());
                        //let mut audio = audio.to_vec();
                        audio_out.append(&mut audio);
                    };
                    if let Some(data_slice) = data.as_slice_mut::<i32>() {
                        //println!("writ {}", data_slice.len());

                        let data_write_len =
                            i64::min(audio_out.len() as i64, (data_slice.len() / 2) as i64)
                                as usize;
                        // if audio_out.len() != 0 {
                        //     info!(
                        //         "audio_out:{}, data_wrt_len:{}",
                        //         audio_out.len(),
                        //         data_write_len
                        //     );
                        // }
                        for idx in 0..data_write_len {
                            data_slice[idx * 2] = (audio_out[idx] as f64 * i32::MAX as f64) as i32;
                            data_slice[idx * 2 + 1] =
                                (audio_out[idx] as f64 * i32::MAX as f64) as i32;
                        }
                        for idx in data_write_len..(data_slice.len() / 2) {
                            data_slice[idx * 2] = 0;
                            data_slice[idx * 2 + 1] = 0;
                        }
                        drop(audio_out.drain(0..data_write_len));
                        // Write back to client to let them know out current buffer size.
                        //let buffer_len = audio_out.len() as u32;
                        // let Ok(()) = tcp_stream
                        //     .lock()
                        //     .unwrap()
                        //     .write_all(&buffer_len.to_le_bytes())
                        // else {
                        //     playback_cb_condvar
                        //         .lock()
                        //         .unwrap()
                        //         .send(AudioServerError::SocketClosed)
                        //         .unwrap();
                        //     return;
                        // };
                    }
                },
                error_cb,
                Some(Duration::from_secs(5)),
            )
            .unwrap();
        stream.play().unwrap();

        let error = error_channel_rx.recv().unwrap();
        drop(stream);
        Err(error)
    }
}
