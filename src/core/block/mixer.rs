use std::fs::create_dir_all;

use itertools::Itertools;
use num::{Complex, Float};

use crate::core::gen::chirp::euler;
use crate::core::signal::FromFunction;
use crate::plot::spectrum::spectrogram;
use crate::prelude::*;
use crate::{
    prelude::{Signal},
    
};

pub struct Mixer<T:SignalType> {
    wave_table: Signal<T>,
    freq: f64,
}

impl<T: SignalType> Mixer<T> {
    pub fn new(sample_rate: f64, freq: f64) -> Mixer<T> {
        let n_samples = sample_rate/freq.abs();

        let mut n_cycles = 1;

        loop {
            let offset = (n_cycles as f64 * n_samples) % 1.0;
            if offset.abs() < 0.01 {
                break;
            } 
            n_cycles+=1;
        }   

        let len = (n_cycles as f64 * n_samples).round() as usize;

        trace!("n_cycles: {n_cycles}, total_len: {}", n_cycles as f64 * n_samples);

        let wave_table = Signal::<T>::from_function(sample_rate, len, |t| euler(T::from_f64_(2.0*core::f64::consts::PI*freq*t)));
        Mixer {wave_table, freq}
    } 
    fn next(&mut self) -> Complex<T> {
        self.wave_table.time += 1;
        //return euler(T::from(2.0*core::f64::consts::PI*(self.wave_table.time as f64 / self.wave_table.sample_rate)*self.freq).unwrap_or_default());
        if self.wave_table.time >= self.wave_table.len() as i64 {
            self.wave_table.time = 0;
        }
        self.wave_table[self.wave_table.time as usize]
    } 
    pub fn mix(&mut self, sig: &Signal<T>) -> Signal<T> {
        //trace!("begin_mix");
        let ret =Signal::<T>::from_vec(self.wave_table.sample_rate, sig.iter().map(|x| *x*self.next()).collect_vec());
        //trace!("end_mix");
        ret
    }

}

#[test]
fn test_mixer() -> anyhow::Result<()> {
    init_tracing();
    create_dir_all("plot/test/test_mixer")?;

    let sig = Signal::<f32>::chirp_real(192000.0, 4800, 65000.0, 75000.0).complex();

    spectrogram("plot/test/test_mixer/passband.png", sig.clone(), 512, 500, true, Some(-120.0));

    let mut mixer = Mixer::<f32>::new(192000.0, -70000.0);
    let sig = mixer.mix(&sig);

    spectrogram("plot/test/test_mixer/baseband.png", sig, 512, 500, true, Some(-120.0));

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
