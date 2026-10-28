use chrono::{Datelike, TimeDelta, Timelike};
use crossbeam::channel::Sender;
use env_logger::Env;
pub use log::{debug, error, info, trace, warn};
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        file::FileAppender,
        Append,
    },
    config::Appender,
    Config,
};
use std::{
    fs::create_dir_all,
    sync::{Arc, Once, OnceLock},
    thread,
};

use crate::{
    core::block::buffer::SignalBuffer,
    prelude::{Signal, SignalType},
};

static INIT: Once = Once::new();
pub static START_TIME: OnceLock<chrono::DateTime<chrono::Utc>> = OnceLock::new();
pub static DATE_STR: OnceLock<String> = OnceLock::new();
pub static BASE_LOG_DIR: OnceLock<String> = OnceLock::new();
pub static WAV_LOG_DIR: OnceLock<String> = OnceLock::new();

fn setup_logging(lvl: &str) {
    INIT.call_once(|| {
        let env = Env::default().filter_or("MULINK_LOG_LVL", lvl);
        env_logger::init_from_env(env);
    });
}
fn setup_logging_file() -> anyhow::Result<()> {
    // let now = chrono::Utc::now();
    INIT.call_once(|| {
        let now = START_TIME.get_or_init(chrono::Utc::now);
        let today =
            DATE_STR.get_or_init(|| format!("{}-{}-{}", now.year(), now.month(), now.day()));
        // let today =
        let _time = format!("{}-{}-{:0<2}", now.hour(), now.minute(), now.second());
        let _base_log_dir = BASE_LOG_DIR.get_or_init(|| format!("mulink-logs/{today}/"));
        let wav_log_dir = WAV_LOG_DIR.get_or_init(|| format!("mulink-logs/{today}/wav/"));
        create_dir_all(wav_log_dir).unwrap();

        let main_log_file = format!("{_base_log_dir}phy.log");
        // let base_plotting_dir = &format!("auto/{today}/");

        // let now = Local::now();
        let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
        let logfile = FileAppender::builder().build(main_log_file).unwrap();
        log4rs::init_config(
            Config::builder()
                .appender(Appender::builder().build("logfile", Box::new(logfile)))
                .appender(Appender::builder().build("stderr", Box::new(stderr)))
                .build(
                    log4rs::config::Root::builder()
                        .appender("logfile")
                        .appender("stderr")
                        .build(log::LevelFilter::Trace),
                )
                .unwrap(),
        )
        .unwrap();
    });
    Ok(())
}
pub fn init_logging() -> anyhow::Result<()> {
    // setup_logging("info");
    setup_logging_file()
}

pub fn init_tracing() -> anyhow::Result<()> {
    // setup_logging("trace");
    setup_logging_file()
}

pub fn init_bb_wav_logger<T: SignalType>(len_ms: f64) -> anyhow::Result<Sender<Signal<T>>> {
    let (s, r) = crossbeam::channel::unbounded::<Signal<T>>();
    thread::spawn(move || {
        let len_samples = ((len_ms / 1000.0) * 24000.0).round() as usize;
        let mut buffer = SignalBuffer::<T>::new(24000.0);
        for sig in r {
            buffer.write(&sig).unwrap();
            if buffer.len() >= len_samples {
                let sig = buffer.read(len_samples).unwrap();
                let st = *START_TIME.get().unwrap();
                let now = st
                    .checked_add_signed(TimeDelta::milliseconds(
                        (sig.time as f64 / 24.0).round() as i64
                    ))
                    .unwrap();
                let base_dir = WAV_LOG_DIR.get().unwrap();
                let file_name = format!(
                    "{base_dir}/{}-{}-{:0<2}.wav",
                    now.hour(),
                    now.minute(),
                    now.second()
                );
                sig.write_wav(&file_name, hound::SampleFormat::Float, false)
                    .unwrap();
            }
        }
    });

    Ok(s)
}
