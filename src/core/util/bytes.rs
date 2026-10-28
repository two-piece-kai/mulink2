pub fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut output = vec![0_u8; bytes.len() * 8];
    for (byte_idx, byte) in bytes.iter().enumerate() {
        for bit in 0..8 {
            output[byte_idx * 8 + bit] = (byte >> bit) & 1;
        }
    }
    output
}

pub fn bits_to_bytes(bits: &[u8]) -> anyhow::Result<Vec<u8>> {
    if bits.len() % 8 != 0 {
        return Err(anyhow::anyhow!("Bytes mismatch"));
    }
    let mut output = vec![0_u8; bits.len() / 8];
    for (bit_idx, bit_value) in bits.iter().enumerate() {
        let bit = bit_idx % 8;
        let byte_idx = bit_idx / 8;
        output[byte_idx] |= bit_value << bit;
    }
    Ok(output)
}
