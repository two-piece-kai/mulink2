use std::{
    collections::VecDeque,
    iter::FusedIterator,
    path::Iter,
    sync::{atomic::AtomicI64, Arc},
};

use std::io::Write;

use itertools::Itertools;
use num::{Complex, Zero};
use plotters::style::AsRelative;

use crate::{core::signal::FromFunction, prelude::*};

pub struct Refragmenter<T: SignalType> {
    time: AtomicI64,
    overflow: Signal<T>,
    buffer: VecDeque<Signal<T>>,
    frag_len: usize,
}
impl<T: SignalType> Refragmenter<T> {
    pub fn new(sample_rate: f64, frag_len: usize) -> Refragmenter<T> {
        Refragmenter {
            time: AtomicI64::new(0),
            overflow: Signal::new(sample_rate),
            buffer: VecDeque::new(),
            frag_len,
        }
    }
    // pub fn push_d(&mut self, sig: &mut Signal<T>) {
    //     *self.buffer.write(sig);
    // }
    pub fn push(&mut self, sig: &mut Signal<T>) {
        //trace!("{}..{}..{}", self.frag_len, self.overflow.len(), sig.len());
        let needed_len = self.frag_len as usize - self.overflow.len() as usize;
        //trace!("{needed_len}");
        //if !self.overflow.is_empty() {
        if (sig.len() as usize) < needed_len {
            self.overflow.append(sig);
            return;
        } else {
            self.overflow
                .extend_from_slice(&sig[0..needed_len as usize]);
        }
        //}
        let n_frags = (sig.len() - needed_len) / self.frag_len;
        //trace!("{n_frags} :: {}", self.overflow.len());
        let frags_range = needed_len..(needed_len + n_frags * self.frag_len);
        self.buffer.push_back(std::mem::replace(
            &mut self.overflow,
            Signal::<T>::from_vec(sig.sample_rate, sig[frags_range.end..].to_vec()),
        ));
        for frag in 0..n_frags {
            self.buffer.push_back(Signal::from_vec(
                sig.sample_rate,
                sig[needed_len + frag * self.frag_len..needed_len + (frag + 1) * self.frag_len]
                    .to_vec(),
            ));
        }
    }
    pub fn finish(mut self) -> Option<Signal<T>> {
        let len = self.overflow.len();
        if len != 0 {
            self.overflow
                .append(&mut vec![num::Complex::<T>::zero(); self.frag_len - len]);
            self.overflow.time = self
                .time
                .fetch_add(self.frag_len as i64, std::sync::atomic::Ordering::Relaxed);
            Some(self.overflow)
        } else {
            None
        }
    }
}
impl<T: SignalType> Iterator for &mut Refragmenter<T> {
    type Item = Signal<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut sig = self.buffer.pop_front()?;
        sig.time = self
            .time
            .fetch_add(self.frag_len as i64, std::sync::atomic::Ordering::Relaxed);
        Some(sig)

        // if self.overflow.len() >= self.frag_len{
        //     let mut sig = Signal::from_vec(self.overflow.sample_rate, self.overflow.drain(0..self.frag_len).as_slice().to_vec());
        //     //let mut sig = Signal::from_vec(self.overflow.sample_rate, self.overflow.drain(0..self.frag_len).collect_vec());
        //     sig.time = self.time.fetch_add(self.frag_len as i64, std::sync::atomic::Ordering::Relaxed);
        //     Some(sig)
        // } else {
        //     None
        // }
    }
}

#[test]
fn test_refrag() -> anyhow::Result<()> {
    init_tracing();
    trace!("Unit test: test_refrag");
    let mut sig = Signal::from_function(192000.0, 512, |x| {
        f32::sin(192000.0 / 4.0 * core::f32::consts::PI * 2.0 * x as f32)
    });
    let mut sig2 = Signal::from_function(192000.0, 512, |x| {
        f32::sin(192000.0 / 4.0 * core::f32::consts::PI * 2.0 * x as f32)
    });
    let mut refrag = Refragmenter::new(192000.0, 192);
    refrag.push(&mut sig);
    for frag in &mut refrag {
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    refrag.push(&mut sig2);
    for frag in &mut refrag {
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    if let Some(frag) = refrag.finish() {
        trace!("finish: {}x{}", frag.time, frag.len());
    }

    trace!("R2");

    let mut sig = Signal::from_function(192000.0, 512, |x| {
        f32::sin(192000.0 / 4.0 * core::f32::consts::PI * 2.0 * x as f32)
    });
    let mut sig2 = Signal::from_function(192000.0, 512, |x| {
        f32::sin(192000.0 / 4.0 * core::f32::consts::PI * 2.0 * x as f32)
    });
    let mut refrag = Refragmenter::new(192000.0, 256);
    refrag.push(&mut sig);
    for frag in &mut refrag {
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    refrag.push(&mut sig2);
    for frag in &mut refrag {
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    if let Some(frag) = refrag.finish() {
        trace!("finish: {}x{}", frag.time, frag.len());
    }
    Ok(())
}
