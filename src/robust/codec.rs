use log::trace;

use crate::{
    core::util::bytes::{bits_to_bytes, bytes_to_bits},
    logging::init_tracing,
    robust::{
        fec::{conv_decode, conv_encode},
        interleave::{deinterleave, interleave},
    },
};

pub fn janus_encode_bytes(bytes: &[u8]) -> Vec<u8> {
    interleave(&conv_encode(&bytes_to_bits(bytes)))
    // conv_encode(&bytes_to_bits(bytes))
}
pub fn janus_decode_bytes(bit_probabilities: &[u8]) -> anyhow::Result<Vec<u8>> {
    // let decoded = &conv_decode(&deinterleave(bits));
    let decoded = &conv_decode(&deinterleave(bit_probabilities));
    bits_to_bytes(&decoded[0..decoded.len() - 8]) //[0..decoded.len() - 8])
}

#[test]
fn test_codec() {
    // init_tracing();

    // init_tracing();

    let input = "Hello World";
    let mut encoded = janus_encode_bytes(input.as_bytes());
    encoded.iter_mut().for_each(|v| *v *= 255);
    let decoded = janus_decode_bytes(&encoded);
    trace!(
        "decoded: {:?}",
        String::from_utf8(decoded.unwrap()).unwrap()
    );
}
