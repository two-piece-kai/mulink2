use std::fs::create_dir_all;

use itertools::Itertools;
use num::{Complex, Float, Zero};

use crate::core::block::filter::Filter;
use crate::core::block::mixer::Mixer;
use crate::core::block::refragment::Refragmenter;
use crate::core::gen::chirp::euler;
use crate::core::r#gen::noise::noise_complex;
use crate::core::signal::FromFunction;
use crate::plot::spectrum::spectrogram;
use crate::prelude::*;
use crate::{
    prelude::{Signal},
    
};

pub struct Downsampler<T: SignalType> {
    overflow: Signal<T>,
    //time: i64,
    ratio: usize,
}
impl<T: SignalType> Downsampler<T> {
    pub fn new(sample_rate_in: f64, down_ratio: usize) -> Downsampler<T> {
        Downsampler { overflow: Signal::new(sample_rate_in), ratio: down_ratio }
    }
    pub fn process(&mut self, mut data: impl Into<Signal<T>>) -> Option<Signal<T>> { 
        let mut data = data.into();
        self.overflow.append(&mut data);
        let chunks = self.overflow.len()/self.ratio;
        let output = self.overflow.drain(0..chunks*self.ratio).collect_vec();

        let mut output_sig = Signal::from_vec(self.overflow.sample_rate/self.ratio as f64, vec![Complex::zero(); chunks]);
        output_sig.time = self.overflow.time;
        self.overflow.time += chunks as i64;
        for (idx, s) in output.iter().enumerate() {
            output_sig[idx/self.ratio]+=*s;
        }
        // self.time += data.len() as i64;
        // self.refrag.push(&mut data);
        // let Some(mut output) = (&mut self.refrag).next() else {return None};
        // let mut output = Signal::<T>::from_vec(data.sample_rate/self.ratio as f64,  vec![output.iter().fold(Complex::<T>::zero(), |acc, x| acc+*x)/T::from(self.ratio).unwrap()]);
        // output.time /= self.ratio as i64;
        // for x in &mut self.refrag {
        //     output.push(x.iter().fold(Complex::<T>::zero(), |acc, x| acc+*x)/*/T::from(self.ratio).unwrap()*/);
        // }
        Some(output_sig)
    }
    pub fn finish(mut self) -> Option<Signal<T>> {
        None
    }
    pub fn process_and_finish(mut self, data: Signal<T>) -> Option<Signal<T>> {
        self.process(data)
    }
}

pub struct Downconv<T:SignalType> {
    mixer: Mixer<T>,
    filter: Filter<T>,
    downsampler: Downsampler<T>,
}

impl<T: SignalType> Downconv<T> {
    pub fn new(sample_rate: f64, fc: f64, bw: f64, filt_order: usize, down_ratio: usize) -> anyhow::Result<Downconv<T>> {
        let mixer = Mixer::<T>::new(sample_rate, -fc);
        let filter = Filter::<T>::lowpass(bw/2.0, filt_order, sample_rate)?;
        Ok(Downconv { mixer, filter, downsampler: Downsampler::new(sample_rate, down_ratio) })
    } 
    pub fn process(&mut self, mut data: impl Into<Signal<T>>) -> Option<Signal<T>> { 
        self.downsampler.process(self.filter.process(self.mixer.mix(&data.into()))?)
        //self.filter.process(self.mixer.mix(&data.into()))
    }
    pub fn finish(mut self) -> Option<Signal<T>> {
        self.downsampler.process_and_finish(self.filter.finish()?)
    }
    pub fn process_and_finish(mut self, data: Signal<T>) -> Option<Signal<T>> {
        self.downsampler.process_and_finish(self.filter.process_and_finish(self.mixer.mix(&data.into()))?)
    }
}

#[test]
fn test_downconv() -> anyhow::Result<()> {
    init_tracing();
    create_dir_all("plot/test/test_downconv")?;

    let mut sig = Signal::<f32>::from_vec(192000.0, vec![Complex::zero(); 4800]);
    sig.append(&mut Signal::<f32>::chirp_real(192000.0, 4800, 65000.0, 75000.0).complex());
    sig.append(&mut vec![Complex::zero(); 4800]);

    let noise = noise_complex(sig.len(), -20.0, 192000.0)?;
    sig += noise;

    spectrogram("plot/test/test_downconv/passband.png", sig.clone(), 512, 256+128, true, Some(-120.0));

    let mut downconv = Downconv::<f32>::new(192000.0, 70000.0, 14000.0, 300, 8)?;
    let sig = downconv.process_and_finish(sig).unwrap();

    spectrogram("plot/test/test_downconv/baseband.png", sig, 128, 64+32+16+4, true, Some(-120.0));

    Ok(())
}

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
