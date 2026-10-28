use std::{
    collections::VecDeque,
    io,
    ops::{Deref, DerefMut, Index, Range},
};

use anyhow::anyhow;
use itertools::Itertools;
use num::Complex;

use crate::prelude::{FromVec, Signal, SignalType};

pub struct HistoryBuffer<T: SignalType> {
    buffer: VecDeque<Complex<T>>,
    pub time: i64,
    pub len: u64,
    pub sample_rate: f64,
}
// impl<T: SignalType> Deref for HistoryBuffer<T> {
//     type Target = VecDeque<Complex<T>>;
//     fn deref(&self) -> &Self::Target {
//         &self.buffer
//     }
// }
// impl<T: SignalType> DerefMut for HistoryBuffer<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.buffer
//     }
// }
impl<T: SignalType> HistoryBuffer<T> {
    pub fn new(sample_rate: f64, len: f64) -> HistoryBuffer<T> {
        HistoryBuffer {
            buffer: VecDeque::new(),
            time: 0,
            sample_rate,
            len: (sample_rate * len).round() as u64,
        }
    }
    // pub fn read(&mut self, len: usize) -> anyhow::Result<Signal<T>> {
    //     let vec: Vec<Complex<T>> = self.buffer.read(len)?;
    //     self.time += len as i64;
    //     Ok(Signal::from_vec(self.sample_rate, vec))
    // }
    fn trim(&mut self) {
        let len = self.buffer.len() as i64 - self.len as i64;
        if len > 0 {
            self.time += len;
            self.buffer.drain(..len as usize);
        }
    }
    pub fn write(&mut self, buf: &[Complex<T>]) -> anyhow::Result<usize> {
        let rv = self.buffer.write(buf);
        self.trim();
        rv
    }
    pub fn get(&self, index: Range<i64>) -> Option<Signal<T>> {
        let start_idx = index.start - self.time;
        let end_idx = index.end - self.time;
        if (0..self.buffer.len() as i64).contains(&end_idx)
            && (0..self.buffer.len() as i64).contains(&start_idx)
        {
            let mut sig = Signal::from_vec(
                self.sample_rate,
                self.buffer
                    .range(start_idx as usize..end_idx as usize)
                    .cloned()
                    .collect_vec(),
            );
            sig.time = start_idx;
            Some(sig)
        } else {
            None
        }
    }
    pub fn full(&self) -> Signal<T> {
        Signal::from_vec(
            self.sample_rate,
            self.buffer.range(..).cloned().collect_vec(),
        )
    }
}
// impl<T: SignalType> Index<Range<i64>> for HistoryBuffer<T> {
//     type Output = Option<Signal<T>>;

// }

pub struct SignalBuffer<T: SignalType> {
    buffer: VecDeque<Complex<T>>,
    pub time: i64,
    pub sample_rate: f64,
}
impl<T: SignalType> Deref for SignalBuffer<T> {
    type Target = VecDeque<Complex<T>>;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
impl<T: SignalType> DerefMut for SignalBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
impl<T: SignalType> SignalBuffer<T> {
    pub fn new(sample_rate: f64) -> SignalBuffer<T> {
        SignalBuffer {
            buffer: VecDeque::new(),
            time: 0,
            sample_rate,
        }
    }
    pub fn read(&mut self, len: usize) -> anyhow::Result<Signal<T>> {
        let vec: Vec<Complex<T>> = self.buffer.read(len)?;
        let mut sig = Signal::from_vec(self.sample_rate, vec);
        sig.time = self.time;
        self.time += len as i64;
        Ok(sig)
    }
    pub fn write(&mut self, buf: &[Complex<T>]) -> anyhow::Result<usize> {
        self.buffer.write(buf)
    }
}

pub trait BufferRead<T: Clone + Default> {
    fn read(&mut self, len: usize) -> anyhow::Result<Vec<T>> {
        let mut vec = vec![T::default(); len];
        self.read_into(&mut vec)?;
        Ok(vec)
    }
    fn read_into(&mut self, buf: &mut [T]) -> anyhow::Result<usize>;
}
pub trait BufferWrite<T: Clone + Default> {
    fn write(&mut self, buf: &[T]) -> anyhow::Result<usize>;
}

impl<T: Clone + Default> BufferRead<T> for VecDeque<T> {
    fn read_into(&mut self, buf: &mut [T]) -> anyhow::Result<usize> {
        let (front, back) = self.as_slices();

        // Use only the front buffer if it is big enough to fill `buf`, else use
        // the back buffer too.
        match buf.split_at_mut_checked(front.len()) {
            None => buf.clone_from_slice(&front[..buf.len()]),
            Some((buf_front, buf_back)) => match back.split_at_checked(buf_back.len()) {
                Some((back, _)) => {
                    buf_front.clone_from_slice(front);
                    buf_back.clone_from_slice(back);
                }
                None => {
                    //self.clear();
                    return Err(anyhow!("READ_EXACT_EOF"));
                }
            },
        }

        self.drain(..buf.len());
        Ok(buf.len())
    }
}
impl<T: Clone + Default> BufferWrite<T> for VecDeque<T> {
    fn write(&mut self, buf: &[T]) -> anyhow::Result<usize> {
        self.extend(buf.iter().cloned());
        Ok(buf.len())
    }
}
// fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
//      let (front, back) = self.as_slices();

//      // Use only the front buffer if it is big enough to fill `buf`, else use
//       // the back buffer too.
//     match buf.split_at_mut_checked(front.len()) {
//         None => buf.copy_from_slice(&front[..buf.len()]),
//          Some((buf_front, buf_back)) => match back.split_at_checked(buf_back.len()) {
//              Some((back, _)) => {
//                   buf_front.copy_from_slice(front);
//                   buf_back.copy_from_slice(back);
//             }
//                None => {
//                   self.clear();
//                   return Err(io::Error::READ_EXACT_EOF);
//               }
//         },
// }

//        self.drain(..buf.len());
//       Ok(())
// }
