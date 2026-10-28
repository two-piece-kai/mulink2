use std::fs::create_dir_all;
use std::mem;

use itertools::Itertools;
use num::{Complex, Float, Zero};

use crate::core::block::filter::Filter;
use crate::core::block::mixer::Mixer;
use crate::core::block::refragment::Refragmenter;
use crate::core::gen::chirp::euler;
use crate::core::r#gen::noise::noise_complex;
use crate::core::signal::FromFunction;
use crate::plot::spectrum::spectrogram;
use crate::plot::time::plot_complex;
use crate::prelude::*;
use crate::{
    prelude::{Signal},
    
};

use plotters::style::AsRelative;

use crate::prelude::*;

pub struct BrickWallWindower<T: SignalType> {
    time: i64,
    //overflow: Signal<T>,
    overflow: Refragmenter<T>,
    overlap: usize,
    prev: Signal<T>, 
}
impl<T:SignalType> BrickWallWindower<T> {
    pub fn new(sample_rate: f64, overlap: usize) -> BrickWallWindower<T> {
        // BrickWallWindower { time: -(overlap as i64), overflow: Signal::new(sample_rate), overlap, prev: Signal::from_vec(sample_rate, vec![Complex::<T>::zero(); overlap]) }
        BrickWallWindower { time: -(overlap as i64), overflow: Refragmenter::new(sample_rate, overlap), overlap, prev: Signal::from_vec(sample_rate, vec![Complex::<T>::zero(); overlap]) }
    }
    // pub fn push_d(&mut self, sig: &mut Signal<T>) {
    //     // self.overflow.append(sig);
    // }
    pub fn push(&mut self, sig: &mut Signal<T>) {
        self.overflow.push(sig);
    }
    pub fn finish(mut self) -> Option<Signal<T>> {
        let mut sig = self.overflow.finish()?;
            let mut prev = mem::replace(&mut self.prev, sig.clone());
            prev.append(&mut sig);
            prev.time = self.time;
            Some(prev)
        // let len = self.overflow.len();
        // if len != 0 {
        //     self.overflow.append(&mut vec![num::Complex::<T>::zero(); self.overlap-len]);
        //     self.overflow.time = self.time;
        //     self.time += self.overlap as i64;
        //     Some(self.overflow)
        // } else {
        //     None
        // }
        //None
    }
}
impl<T:SignalType> Iterator for &mut BrickWallWindower<T> {
    type Item = Signal<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // if self.overflow.len() >= self.overlap{
        //     let mut sig = self.prev.clone();
        //     self.prev.clone_from_slice(self.overflow.drain(0..self.overlap).as_slice());
        //     sig.append(&mut self.prev.clone());
        //     //let mut sig = Signal::from_vec(self.overflow.sample_rate, self.overflow.drain(0..self.overlap).collect_vec());
        //     sig.time = self.time;
        //     self.time += self.overlap as i64;
        //     Some(sig)
        // } else {
        //     None
        // }

        //if self.overflow.len() >= self.overlap{
            let mut sig = (&mut self.overflow).next()?;
            let mut prev = mem::replace(&mut self.prev, sig.clone());
            prev.append(&mut sig);
            prev.time = self.time;
            self.time += self.overlap as i64;
            Some(prev)
        //} else {
        //    None
        //}
    }
}

#[test]
fn test_brickwall() -> anyhow::Result<()> {
    init_tracing();
    create_dir_all("plot/test/test_windower")?;
    trace!("Unit test: test_brickwall_windower");
    //let mut sig = Signal::from_function(192000.0, 512, |x| f32::sin(192000.0/4.0 * core::f32::consts::PI * 2.0 * x as f32));
    //let mut sig2 = Signal::from_function(192000.0, 512, |x| f32::sin(192000.0/4.0 * core::f32::consts::PI * 2.0 * x as f32));
    let mut sig = Signal::<f32>::chirp_complex(128.0, 512, -64.0, 64.0);
    let mut sig2 = Signal::<f32>::chirp_complex(128.0, 512, -64.0, 64.0);
    let mut refrag = BrickWallWindower::new(192000.0, 256);

    let mut idx = 0;
    refrag.push(&mut sig);
    for frag in &mut refrag {
        idx += 1;
        plot_complex(&format!("plot/test/test_windower/{idx}.png"), "window", &frag);
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    refrag.push(&mut sig2);
    for frag in &mut refrag {
        idx += 1;
        plot_complex(&format!("plot/test/test_windower/{idx}.png"), "window", &frag);
        trace!("frag: {}x{}", frag.time, frag.len());
    }
    if let Some(frag) = refrag.finish() {
        idx += 1;
        plot_complex(&format!("plot/test/test_windower/{idx}.png"), "window", &frag);
        trace!("finish: {}x{}", frag.time, frag.len());
    }
    Ok(())
}

// pub struct BrickWallWindower<T: SignalType> {
//     refrag: Refragmenter<T>,
//     time: i64,
//     ratio: usize,
//     prev: Signal<T>,
// }
// impl<T: SignalType> BrickWallWindower<T> {
//     pub fn new(sample_rate: f64, overlap: usize) -> BrickWallWindower<T> {
//         BrickWallWindower { refrag: Refragmenter::new(sample_rate, overlap), time: 0, ratio: overlap, prev: Signal::from_vec(sample_rate, vec![Complex::<T>::zero(); overlap]) }
//     }
//     pub fn process(&mut self, mut data: impl Into<Signal<T>>) -> Option<Vec<Signal<T>>> { 
//         let mut data = data.into();
//         self.time += data.len() as i64;
//         self.refrag.push(&mut data);
//         let mut output = (&mut self.refrag).next()?;
//         let mut output = Signal::<T>::from_vec(data.sample_rate/self.ratio as f64,  vec![output.iter().fold(Complex::<T>::zero(), |acc, x| acc+*x)/T::from(self.ratio).unwrap()]);
//         output.time /= self.ratio as i64;
//         for x in &mut self.refrag {
//             output.push(x.iter().fold(Complex::<T>::zero(), |acc, x| acc+*x)/*/T::from(self.ratio).unwrap()*/);
//         }
//         Some(output)
//     }
//     pub fn finish(mut self) -> Option<Vec<Signal<T>>> {
//         None
//     }
//     pub fn process_and_finish(mut self, data: Signal<T>) -> Option<Vec<Signal<T>>> {
//         self.process(data)
//     }
// }

// pub struct Downconv<T:SignalType> {
//     mixer: Mixer<T>,
//     filter: Filter<T>,
//     downsampler: BrickWallWindower<T>,
// }

// impl<T: SignalType> Downconv<T> {
//     pub fn new(sample_rate: f64, fc: f64, bw: f64, filt_order: usize, down_ratio: usize) -> anyhow::Result<Downconv<T>> {
//         let mixer = Mixer::<T>::new(sample_rate, -fc);
//         let filter = Filter::<T>::lowpass(bw/2.0, filt_order, sample_rate)?;
//         Ok(Downconv { mixer, filter, downsampler: BrickWallWindower::new(sample_rate, down_ratio) })
//     } 
//     pub fn process(&mut self, mut data: impl Into<Signal<T>>) -> Option<Signal<T>> { 
//         self.downsampler.process(self.filter.process(self.mixer.mix(&data.into()))?)
//     }
//     pub fn finish(mut self) -> Option<Signal<T>> {
//         self.downsampler.process_and_finish(self.filter.finish()?)
//     }
//     pub fn process_and_finish(mut self, data: Signal<T>) -> Option<Signal<T>> {
//         self.downsampler.process_and_finish(self.filter.process_and_finish(self.mixer.mix(&data.into()))?)
//     }
// }

// #[test]
// fn test_downconv() -> anyhow::Result<()> {
//     init_tracing();
//     create_dir_all("plot/test/test_downconv")?;

//     let mut sig = Signal::<f32>::from_vec(192000.0, vec![Complex::zero(); 4800]);
//     sig.append(&mut Signal::<f32>::chirp_real(192000.0, 4800, 65000.0, 75000.0).complex());
//     sig.append(&mut vec![Complex::zero(); 4800]);

//     let noise = noise_complex(sig.len(), -20.0, 192000.0)?;
//     sig += noise;

//     spectrogram("plot/test/test_downconv/passband.png", sig.clone(), 512, 256+128, true, Some(-120.0));

//     let mut downconv = Downconv::<f32>::new(192000.0, 70000.0, 14000.0, 300, 8)?;
//     let sig = downconv.process_and_finish(sig).unwrap();

//     spectrogram("plot/test/test_downconv/baseband.png", sig, 128, 64+32+16+4, true, Some(-120.0));

//     Ok(())
// }

// pub struct Mixer<T> {
//     width: usize,
//     normalized_freq: f64,
//     wave_table: Signal<T>,
//     wrap: usize,
//     phase_mem_offset: usize,
// }
// impl<T: SignalType> Mixer<T> {
//     pub fn new(width: usize, normalized_freq: f64) -> Mixer<T> {
//         let mut min = 1.0;
//         let mut wrap = 0;
//         for i in 1..width {
//             let test_min = ((i as f64) * normalized_freq) % 1.0;
//             if (test_min) < min {
//                 //println!("test:{test_min}min:{min}");
//                 min = test_min;
//                 wrap = i;
//             }
//         }
//         //println!("Wrap: {wrap}");
//         return Mixer {
//             width,
//             normalized_freq,
//             signal: Signal::from_vec(
//                 0.0,
//                 gen_complex(wrap + width, 1.0, -normalized_freq)
//                     .iter()
//                     .map(|x| Complex {
//                         re: T::from_f32(x.re).unwrap_or_default(),
//                         im: T::from_f32(x.im).unwrap_or_default(),
//                     })
//                     .collect(),
//             ),
//             wrap,
//             phase_mem_offset: 0,
//         };
//     }
//     pub fn process_segment(&mut self, data: &ComplexSignal<T>) -> ComplexSignal<T> {
//         let output: Vec<Complex<T>> = data
//             .iter()
//             .zip(&self.signal[self.phase_mem_offset..(self.phase_mem_offset + data.len())])
//             .map(|(sample_rx, sample_fc)| sample_rx * *sample_fc)
//             .collect();
//         /*let output = (&data)
//         .into_iter()
//         .zip(&self.signal[self.phase_mem_offset..(self.phase_mem_offset + self.width)])
//         .map(|(sample_rx, sample_fc)| {
//             Complex {
//                 re: *sample_rx,
//                 im: 0.0,
//             } * sample_fc
//         })
//         .collect();*/
//         self.phase_mem_offset = (self.phase_mem_offset + data.len()) % self.wrap;
//         ComplexSignal::from_vec(data.sample_rate, output)
//     }
//     pub fn process(&mut self, data: &mut ComplexSignal<T>) -> ComplexSignal<T> {
//         let mut output: ComplexSignal<T> = ComplexSignal::new(data.sample_rate);

//         data.chunks(self.width)
//             .map(|x| self.process_segment(&ComplexSignal::from_vec(data.sample_rate, x.to_vec())))
//             .for_each(|mut x| output.append(&mut x));

//         return output;
//     }
// }
