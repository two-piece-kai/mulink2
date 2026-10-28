use std::{os::raw::c_void, ptr::null_mut};

use code_rs::{
    bits::Dibit,
    coding::trellis::{DibitHistory, DibitStates, DibitWalks, ViterbiDecoder},
};
use itertools::Itertools;
use log::trace;
// use rand::Fill;
use rkyv::de;

use crate::{
    core::util::bytes::{bits_to_bytes, bytes_to_bits},
    logging::{init_logging, init_tracing},
};

const TRELLIS_OUTPUTS: [u8; 512] = [
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2,
    3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1,
    3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1,
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2,
    2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0,
    1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3,
    1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3,
    2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0,
    3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1,
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2,
    0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3, 1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2,
    3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1,
    1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3,
    2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0,
    2, 1, 0, 3, 2, 1, 0, 3, 0, 3, 2, 1, 0, 3, 2, 1, 3, 0, 1, 2, 3, 0, 1, 2, 1, 2, 3, 0, 1, 2, 3, 0,
    1, 2, 3, 0, 1, 2, 3, 0, 3, 0, 1, 2, 3, 0, 1, 2, 0, 3, 2, 1, 0, 3, 2, 1, 2, 1, 0, 3, 2, 1, 0, 3,
];

const TRELLIS_NEXT_STATES: [u8; 512] = [
    0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24, 25, 25, 26,
    26, 27, 27, 28, 28, 29, 29, 30, 30, 31, 31, 32, 32, 33, 33, 34, 34, 35, 35, 36, 36, 37, 37, 38,
    38, 39, 39, 40, 40, 41, 41, 42, 42, 43, 43, 44, 44, 45, 45, 46, 46, 47, 47, 48, 48, 49, 49, 50,
    50, 51, 51, 52, 52, 53, 53, 54, 54, 55, 55, 56, 56, 57, 57, 58, 58, 59, 59, 60, 60, 61, 61, 62,
    62, 63, 63, 64, 64, 65, 65, 66, 66, 67, 67, 68, 68, 69, 69, 70, 70, 71, 71, 72, 72, 73, 73, 74,
    74, 75, 75, 76, 76, 77, 77, 78, 78, 79, 79, 80, 80, 81, 81, 82, 82, 83, 83, 84, 84, 85, 85, 86,
    86, 87, 87, 88, 88, 89, 89, 90, 90, 91, 91, 92, 92, 93, 93, 94, 94, 95, 95, 96, 96, 97, 97, 98,
    98, 99, 99, 100, 100, 101, 101, 102, 102, 103, 103, 104, 104, 105, 105, 106, 106, 107, 107,
    108, 108, 109, 109, 110, 110, 111, 111, 112, 112, 113, 113, 114, 114, 115, 115, 116, 116, 117,
    117, 118, 118, 119, 119, 120, 120, 121, 121, 122, 122, 123, 123, 124, 124, 125, 125, 126, 126,
    127, 127, 128, 128, 129, 129, 130, 130, 131, 131, 132, 132, 133, 133, 134, 134, 135, 135, 136,
    136, 137, 137, 138, 138, 139, 139, 140, 140, 141, 141, 142, 142, 143, 143, 144, 144, 145, 145,
    146, 146, 147, 147, 148, 148, 149, 149, 150, 150, 151, 151, 152, 152, 153, 153, 154, 154, 155,
    155, 156, 156, 157, 157, 158, 158, 159, 159, 160, 160, 161, 161, 162, 162, 163, 163, 164, 164,
    165, 165, 166, 166, 167, 167, 168, 168, 169, 169, 170, 170, 171, 171, 172, 172, 173, 173, 174,
    174, 175, 175, 176, 176, 177, 177, 178, 178, 179, 179, 180, 180, 181, 181, 182, 182, 183, 183,
    184, 184, 185, 185, 186, 186, 187, 187, 188, 188, 189, 189, 190, 190, 191, 191, 192, 192, 193,
    193, 194, 194, 195, 195, 196, 196, 197, 197, 198, 198, 199, 199, 200, 200, 201, 201, 202, 202,
    203, 203, 204, 204, 205, 205, 206, 206, 207, 207, 208, 208, 209, 209, 210, 210, 211, 211, 212,
    212, 213, 213, 214, 214, 215, 215, 216, 216, 217, 217, 218, 218, 219, 219, 220, 220, 221, 221,
    222, 222, 223, 223, 224, 224, 225, 225, 226, 226, 227, 227, 228, 228, 229, 229, 230, 230, 231,
    231, 232, 232, 233, 233, 234, 234, 235, 235, 236, 236, 237, 237, 238, 238, 239, 239, 240, 240,
    241, 241, 242, 242, 243, 243, 244, 244, 245, 245, 246, 246, 247, 247, 248, 248, 249, 249, 250,
    250, 251, 251, 252, 252, 253, 253, 254, 254, 255, 255,
];

const TRELLIS_STATE_COUNT: u32 = 256;

pub fn conv_encode(bits: &[u8]) -> Vec<u8> {
    let mut state: u32 = 0;
    // let mut bytes_ = vec![0; 8];
    // bytes_.extend_from_slice(bytes);
    // let bytes = bytes_;
    let mut bytes = bits.to_vec();
    bytes.extend_from_slice(&[0; 8]); // flush
    let mut out = vec![0_u8; bytes.len() * 2];

    for n in 0..bytes.len() {
        let idx = (state + bytes[n] as u32 * TRELLIS_STATE_COUNT) as usize;
        out[n << 1] = (TRELLIS_OUTPUTS[idx] >> 1) & 1;
        out[(n << 1) + 1] = TRELLIS_OUTPUTS[idx] & 1;
        state = TRELLIS_NEXT_STATES[idx] as u32;
    }
    out
}

const MAX_METRIC: u32 = 999999u32;
pub fn conv_decode(bits: &[u8]) -> Vec<u8> {
    let jtr = _janus_trellis_default();
    let mut jvt = _janus_viterbi_new(jtr, 8, 9 * 5);
    let mut out = vec![0; bits.len() / 2];
    let tbk_depth = (9 * 5).min(out.len() as u32);
    _janus_viterbi_execute(
        &mut jvt, bits, // output.len() as u32,
        &mut out, tbk_depth,
    );
    out
}

#[test]
fn test_janus_fec() {
    init_tracing();
    let input: Vec<u8> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
    ];
    let input = bytes_to_bits(&input);
    // let input: Vec<u8> = vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
    // let mut input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0];
    // input.append(&mut vec![0; 48]);

    trace!("input: {:?}", (&input));
    // trace!("input: {:?}", bits_to_bytes(&bytes_to_bits(&input)));
    // trace!("input: {:?}", &input);
    // let output = bits_to_bytes(&conv_encode(&bytes_to_bits(&input)));
    let mut output = conv_encode(&input);
    trace!("encoded: {:?}", output);
    output.iter_mut().for_each(|v| *v *= 255);
    // let decode = conv_decode(&output);
    // trace!("decoded: {:?}", decode);

    // let mut jtr = janus_trellis_default();
    let decoded = conv_decode(&output);

    let decoded = bits_to_bytes(&decoded);
    trace!("decoded (c): {:?}", decoded);
    // janus_viterbi_free(jvt);
    // jvct();
}

static c_default_trellis: JanusTrellis = JanusTrellis {
    inp_sym_count: 2,
    out_sym_count: 4,
    state_count: 256,
    outputs: &TRELLIS_OUTPUTS, //janus_u8_ptr(TRELLIS_OUTPUTS.as_ptr()),
    next_states: &TRELLIS_NEXT_STATES,
};

const fn _janus_trellis_default() -> &'static JanusTrellis {
    &c_default_trellis
}

#[repr(C)]
struct JanusTrellis {
    // Number of input symbols.
    inp_sym_count: u32,
    // Number of output symbols.
    out_sym_count: u32,
    // Number of states.
    state_count: u32,
    // Outputs for all combinations of current state and current input.
    outputs: &'static [u8],
    // Next states for all combinations of current state and current input.
    next_states: &'static [u8],
}
#[repr(C)]
struct JanusViterbi {
    // Trellis structure.
    trellis: &'static JanusTrellis,
    // Soft decision bits.
    sdc_bits: u32,
    // Branch metric.
    met_brc: Vec<u32>,
    // Temporary metric.
    met_tmp: Vec<u32>,
    // State metric.
    met_sta: Vec<u32>,
    // Traceback depth.
    tbk_depth: u32,
    // Traceback input.
    tbk_inp: Vec<u32>,
    // Traceback state.
    tbk_sta: Vec<u32>,
    // Number of bits per output symbol.
    k: u32,
    // Number of bits per input symbol.
    n: u32,
}

fn _janus_viterbi_new(
    trellis: &'static JanusTrellis,
    sdc_bits: u32,
    tbk_depth: u32,
) -> JanusViterbi {
    let mut vit = JanusViterbi {
        trellis,
        k: trellis.inp_sym_count.ilog2(),
        n: trellis.out_sym_count.ilog2(),
        sdc_bits,
        met_brc: vec![0; (1 << trellis.out_sym_count) as usize],
        met_tmp: vec![0; trellis.state_count as usize],
        met_sta: vec![0; trellis.state_count as usize],
        tbk_inp: Vec::new(),
        tbk_sta: Vec::new(),
        tbk_depth: 0,
    };
    _reset_traceback(&mut vit, tbk_depth);
    _reset_metric(&mut vit);
    vit
}

fn _janus_viterbi_execute(vit: &mut JanusViterbi, inp: &[u8], out: &mut [u8], tbk_depth: u32) {
    let blk_size = inp.len() as u32 / vit.n;
    // Traceback index.

    let mut tbk_idx: u32 = 0;
    // Minimum state.
    let mut sta_min: u32 = 0;
    let mut tbk_last = 0;
    let mut out_offs = 0;
    let mut input = 0;

    _reset_traceback(vit, tbk_depth);
    _reset_metric(vit);

    for i in 0..blk_size {
        _branch_metric(vit, inp, i);
        sta_min = _update_state_metric(vit, &mut tbk_idx) as u32;
        tbk_last = _traceback(vit, out, blk_size, i, sta_min, &mut tbk_idx);
    }

    for i in 0..vit.tbk_depth {
        out_offs = (blk_size - i - 1) * vit.k;

        input = vit.tbk_inp[(sta_min as i32 + (tbk_last * vit.trellis.state_count as i32)) as usize]
            as i32;
        for j in 0..vit.k {
            out[(out_offs + vit.k - 1 - j) as usize] = (input & 1) as u8;
            input >>= 1;
        }

        sta_min =
            vit.tbk_sta[(sta_min as i32 + (tbk_last * vit.trellis.state_count as i32)) as usize];
        tbk_last = if tbk_last > 0 {
            tbk_last - 1
        } else {
            vit.tbk_depth as i32
        };
    }
}

fn _reset_traceback(vit: &mut JanusViterbi, tbk_depth: u32) {
    if vit.tbk_depth == tbk_depth {
        vit.tbk_inp.fill(0);
        vit.tbk_sta.fill(0);
        return;
    }

    vit.tbk_inp = vec![0; vit.trellis.state_count as usize * (tbk_depth + 1) as usize];
    vit.tbk_sta = vec![0; vit.trellis.state_count as usize * (tbk_depth + 1) as usize];

    vit.tbk_depth = tbk_depth;
}

fn _reset_metric(vit: &mut JanusViterbi) {
    vit.met_sta.fill(0);
}

fn _branch_metric(vit: &mut JanusViterbi, inp: &[u8], idx: u32) {
    let inp_off = idx * vit.n;

    let mut tmp = 0;

    for i in 0..(1 << vit.n) {
        tmp = i;
        vit.met_brc[i] = 0;

        // for (j = 0; j < vit->n; ++j) {
        for j in 0..vit.n {
            if tmp & 1 != 0 {
                vit.met_brc[i] +=
                    ((1 << vit.sdc_bits) - 1) - inp[(inp_off + vit.n - j - 1) as usize] as u32;
            //inp[inp_off + vit.n - j - 1];
            } else {
                vit.met_brc[i] += inp[(inp_off + vit.n - j - 1) as usize] as u32;
            }

            tmp >>= 1;
        }
    }
}

fn _update_state_metric(vit: &mut JanusViterbi, tbk_idx: &mut u32) -> i32 {
    let mut sta_nxt = 0;
    // Current output.

    let mut out_cur = 0;

    let mut renorm = MAX_METRIC;

    let mut sta_min = 0;

    for i in 0..vit.trellis.state_count as usize {
        vit.met_tmp[i] = MAX_METRIC;
    }

    for i in 0..vit.trellis.state_count as usize {
        for j in 0..(1 << vit.k) as usize {
            sta_nxt = vit.trellis.next_states[j * vit.trellis.state_count as usize + i];
            out_cur = vit.trellis.outputs[j * vit.trellis.state_count as usize + i];

            if vit.met_sta[i] + vit.met_brc[out_cur as usize] < vit.met_tmp[sta_nxt as usize] {
                vit.met_tmp[sta_nxt as usize] = vit.met_sta[i] + vit.met_brc[out_cur as usize];
                vit.tbk_sta
                    [sta_nxt as usize + (*tbk_idx as usize * vit.trellis.state_count as usize)] =
                    i as u32;
                vit.tbk_inp
                    [sta_nxt as usize + (*tbk_idx as usize * vit.trellis.state_count as usize)] =
                    j as u32;

                if vit.met_tmp[sta_nxt as usize] < renorm {
                    renorm = vit.met_tmp[sta_nxt as usize];
                }
            }
        }
    }

    for i in 0..vit.trellis.state_count as usize {
        vit.met_sta[i] = vit.met_tmp[i] - renorm;
        if vit.met_sta[i] == 0 {
            sta_min = i;
        }
    }

    sta_min as i32
}

fn _traceback(
    vit: &mut JanusViterbi,
    out: &mut [u8],
    blk_size: u32,
    blk_idx: u32,
    mut sta_min: u32,
    tbk_idx: &mut u32,
) -> i32 {
    let mut tbwork = *tbk_idx;
    //   int input = 0;
    let mut input = 0;
    //   int tbk_last = tbwork;
    let mut tbk_last = tbwork;

    let mut blk_out = 0;

    if (blk_idx >= vit.tbk_depth) {
        for i in 0..vit.tbk_depth + 1 {
            input = (vit.tbk_inp
                [sta_min as usize + (tbwork as usize * vit.trellis.state_count as usize)])
                as i32;
            sta_min = (vit.tbk_sta
                [sta_min as usize + (tbwork as usize * vit.trellis.state_count as usize)])
                as u32;
            tbwork = if tbwork > 0 {
                tbwork - 1
            } else {
                vit.tbk_depth
            };
        }

        blk_out = (((blk_idx - vit.tbk_depth) % blk_size) * vit.k) as usize;

        for i in 0..vit.k {
            out[blk_out + ((vit.k - 1 - i) as usize)] = (input & 1) as u8;
            input >>= 1;
        }
    }

    *tbk_idx = if *tbk_idx < vit.tbk_depth {
        *tbk_idx + 1
    } else {
        0
    };

    tbk_last as i32
}
