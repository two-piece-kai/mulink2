use std::ops::Index;

use log::trace;

use crate::logging::init_tracing;

// use anyhow::Ok;

#[derive(Clone, Copy, Default, Debug)]
pub struct SymbolPair {
    pub idx: usize,
    pub f0: f64,
    pub f1: f64,
    pub guard_f0: (f64, f64),
    pub guard_f1: (f64, f64),
}
pub struct MuLinkHopPattern {
    idx: u64,
    pub tab: Vec<SymbolPair>,
    n_hops: usize,
}
impl MuLinkHopPattern {
    pub fn new(n_hops: usize, fc: f64, bw: f64, sampling_rate: f64) -> MuLinkHopPattern {
        let max = (fc + bw / 2.0) / (sampling_rate / 2.0);
        let min = (fc - bw / 2.0) / (sampling_rate / 2.0);
        let step = (bw / (sampling_rate / 2.0)) / (n_hops as f64 * 2.0 - 1.0);
        let guard_int = step / 3.0; // aribtrary. Min value would be step/2.0
                                    // let guard_int = step / 3.0;
        let mut tab = vec![SymbolPair::default(); n_hops];
        tab.iter_mut().enumerate().for_each(|(idx, v)| {
            let f0 = min + step * (idx as f64 * 2.0);
            let f1 = f0 + step;
            let guard_f0 = (f0 - guard_int, f0 + guard_int);
            let guard_f1 = (f1 - guard_int, f1 + guard_int);
            *v = SymbolPair {
                idx,
                f0,
                f1,
                guard_f0,
                guard_f1,
            }
        });
        MuLinkHopPattern {
            idx: 0,
            tab,
            n_hops,
        }
    }
}
impl Index<usize> for MuLinkHopPattern {
    type Output = SymbolPair;
    fn index(&self, idx: usize) -> &SymbolPair {
        &self.tab[(idx * 3) % self.n_hops]
    }
}

#[test]
fn test_hop_pattern() -> anyhow::Result<()> {
    init_tracing();
    trace!("test_hop_pattern");
    let tab = MuLinkHopPattern::new(13, 70000.0, 10000.0, 192000.0);
    for idx in 0..13 {
        // if idx % 3 == 2 {
        let pair = tab.tab[idx];
        trace!(
            "{} = ({} [{} {}], {} [{} {}])",
            pair.idx,
            pair.f0 * 192000.0 / 2.0,
            pair.guard_f0.0 * 192000.0 / 2.0,
            pair.guard_f0.1 * 192000.0 / 2.0,
            pair.f1 * 192000.0 / 2.0,
            pair.guard_f1.0 * 192000.0 / 2.0,
            pair.guard_f1.1 * 192000.0 / 2.0,
        );
        // } else {
        // print!("{:?}", tab[idx])
        // }
    }
    let tab = MuLinkHopPattern::new(13, 0.0, 10000.0, 24000.0);
    for idx in 0..13 {
        // if idx % 3 == 2 {
        let pair = tab.tab[idx];
        trace!(
            "{} = ({}, {})",
            pair.idx,
            pair.f0 * 24000.0 / 2.0,
            pair.f1 * 24000.0 / 2.0
        );
        // } else {
        // print!("{:?}", tab[idx])
        // }
    }
    Ok(())
}
