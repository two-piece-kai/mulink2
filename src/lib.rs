pub mod logging;
pub mod prelude;
pub mod backend {
    pub mod cpal;
}
pub mod core {
    pub mod signal;
    pub mod signal_ops;
    pub mod stream;
    pub mod block {
        pub mod buffer;
        pub mod downconv;
        pub mod fft;
        pub mod filter;
        pub mod mixer;
        pub mod refragment;
        pub mod windower;
    }
    pub mod gen {
        pub mod chirp;
        pub mod fir;
        pub mod noise;
    }
    pub mod util {
        pub mod bytes;
        pub mod compute_snr;
        pub mod find_direct_path;
        pub mod rms;
    }
}
pub mod protocol {
    pub mod mulink {
        pub mod chip_pattern;
        pub mod fsk;
        pub mod offline_decode;
        pub mod preamble;
    }
}
pub mod io {
    pub mod txt;
    pub mod wav;
}
pub mod plot {
    pub mod spectrum;
    pub mod time;
}
pub mod fs {
    pub mod init;
}
pub mod programs {
    pub mod detect_envelope;
    pub mod ir_extract;
}
pub mod robust {
    pub mod codec;
    pub mod fec;
    pub mod interleave;
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
