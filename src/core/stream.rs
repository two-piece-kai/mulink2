use core::{f32, f64};
use std::{
    marker::PhantomData,
    sync::{atomic::AtomicI64, Arc, Mutex, MutexGuard, PoisonError},
    thread,
};

use anyhow::Result;

use crossbeam::channel::{Receiver, Sender};
use log::{info, trace};

use crate::{
    core::signal::{FromFunction, FromVec, Signal, SignalType},
    logging::init_tracing,
};

pub struct Topic<T> {
    subscribers: Arc<Mutex<Vec<Sender<Arc<T>>>>>,
    publisher: Sender<T>,
}
impl<T: Send + Sync + Clone + 'static> Topic<T> {
    pub fn new() -> Self {
        let (send, recv) = crossbeam::channel::unbounded::<T>();
        let topic = Topic {
            publisher: send,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        };
        Self::launch_daemon(topic.subscribers.clone(), recv);
        topic
    }
    pub fn get_subscriber(&self) -> Receiver<Arc<T>> {
        let (send, recv) = crossbeam::channel::unbounded::<Arc<T>>();
        self.subscribers.lock().unwrap().push(send);
        recv
    }
    pub fn get_publisher(&self) -> Sender<T> {
        self.publisher.clone()
    }
    fn launch_daemon(subs: Arc<Mutex<Vec<Sender<Arc<T>>>>>, recv: Receiver<T>) {
        thread::spawn(move || {
            let subs = subs.clone();
            'service_topic: loop {
                let msg = recv.recv();
                let Ok(msg) = msg else {
                    break 'service_topic;
                };
                let msg = Arc::new(msg);

                let mut subs = subs.lock().unwrap();
                // trace!(
                //     "Dispatching message from Topic<T> to {} subscribers",
                //     subs.len()
                // );
                subs.retain_mut(|sub| {
                    let result = sub.send(msg.clone());
                    if result.is_ok() {
                        true
                    } else {
                        //info!("dropped a sub");
                        false
                    }
                });
            }
        });
    }
}
impl<T: Send + Sync + Clone + 'static> Default for Topic<T> {
    fn default() -> Self {
        Self::new()
    }
}
pub struct AudioTxGuard<T: SignalType> {
    tx: Sender<Signal<T>>,
    time: Arc<AtomicI64>,
}
impl<T: SignalType> AudioTxGuard<T> {
    pub fn send(&self, mut sig: Signal<T>) -> anyhow::Result<()> {
        sig.time = self.time.load(std::sync::atomic::Ordering::Relaxed);
        self.time
            .fetch_add(sig.len() as i64, std::sync::atomic::Ordering::Relaxed);
        Ok(self.tx.send(sig)?)
    }
}
pub struct AudioStream<T: SignalType> {
    tx: Mutex<AudioTxGuard<T>>,
    topic: Topic<Signal<T>>,
    time: Arc<AtomicI64>,
}
impl<T: SignalType> AudioStream<T> {
    pub fn new() -> AudioStream<T> {
        // trace!("AudioStream::new()");
        let topic = Topic::<Signal<T>>::new();
        let time = Arc::new(AtomicI64::new(0));
        let tx = Mutex::new(AudioTxGuard {
            tx: topic.get_publisher(),
            time: time.clone(),
        });
        AudioStream { tx, topic, time }
    }
    pub fn get_subscriber(&self) -> Receiver<Arc<Signal<T>>> {
        self.topic.get_subscriber()
    }
    pub fn lock(&self) -> anyhow::Result<MutexGuard<'_, AudioTxGuard<T>>> {
        if let Ok(tx) = self.tx.lock() {
            Ok(tx)
        } else {
            Err(anyhow::anyhow!("mulink-dsp::audiostream_tx_lock_failure"))
        }
    }
    pub fn time(&self) -> i64 {
        self.time.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[test]
fn test_audiostream() -> anyhow::Result<()> {
    init_tracing();
    info!("Unit test: test_audiostream");
    // Create a new AudioStream (yields chunks of Complex<f32>)
    trace!("INST stream");
    let stream = AudioStream::<f32>::new();

    // Create two subscribers to the AudioStream
    trace!("INST subs");
    let sub1 = stream.get_subscriber();
    let sub2 = stream.get_subscriber();

    // Transmit something into the AudioStream
    trace!("lock1");
    {
        // Lock / "Reserve" the transmit handle of the AudioStream and then transmit a generated signal
        let tx = stream.lock()?;
        // Syntax showing off how to generate a Signal by passing a sample rate, len, and a closure
        tx.send(Signal::from_function(192000.0, 256, |x| {
            f32::sin(75000.0 * f32::consts::PI * 2.0 * x as f32)
        }))?;
    }

    // Receive messages & validate that they are tagged with t=0
    trace!("recv1");
    let msg1 = sub1.recv()?;
    assert_eq!(msg1.time, 0);
    let msg2 = sub2.recv()?;
    assert_eq!(msg2.time, 0);

    // Send another signal, just vector of zeros
    trace!("lock2");
    {
        let tx = stream.lock()?;
        tx.send(Signal::from_vec(192000.0, vec![0.0; 256]))?;
    }

    // Receive messages & validate that they are tagged with t=256
    trace!("recv2");
    let msg1 = sub1.recv()?;
    assert_eq!(msg1.time, 256);
    let msg2 = sub2.recv()?;
    assert_eq!(msg2.time, 256);

    // Query AudioStream time offset
    trace!("time: {}", stream.time());
    assert_eq!(stream.time(), 512);

    Ok(())
}
