const c_prime_min: u32 = 5;

fn _janus_interleave_q(length: u32) -> u32 {
    // Find the lowest prime number which is not a factor of length.
    // @fixme: there's something more to this.
    // and it was never fixed. Oops! -BA
    let mut k = c_prime_min;
    let mut prime = 0;

    loop {
        prime = _janus_utils_primes_get_previous(k);
        if ((length / prime) < prime) && !length.is_multiple_of(prime) {
            break;
        }
        k += 1;
    }

    prime
}

pub fn interleave(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0; bits.len()];
    _janus_interleave(bits, &mut out);
    out
}
pub fn deinterleave(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0; bits.len()];
    _janus_deinterleave(bits, &mut out);
    out
}

fn _janus_interleave(inp: &[u8], out: &mut [u8]) {
    let prime = _janus_interleave_q(inp.len() as u32);
    //   unsigned i = 0;
    let mut perm = vec![0_u32; inp.len()]; //(unsigned*)malloc(sizeof(unsigned) * inp_len);

    // Build permutation table.
    perm[0] = 0;
    //   for (i = 1; i < inp_len; ++i)
    for i in 1..inp.len() {
        perm[i] = (perm[i - 1] + prime) % inp.len() as u32;
    }

    // Perform permutation.
    //   for (i = 0; i < inp_len; ++i)
    for i in 0..inp.len() {
        out[i] = inp[perm[i] as usize];
    }

    //   free(perm);
}

fn _janus_deinterleave(inp: &[u8], out: &mut [u8]) {
    let q = _janus_interleave_q(inp.len() as u32);
    //   let i = 0;
    let mut idx = 0;

    out[0] = inp[0];
    //   for (i = 1; i < inp_len; ++i)
    for i in 1..inp.len() {
        idx = (idx + q) % inp.len() as u32;
        out[idx as usize] = inp[i];
    }
}

const c_prime_table: [u32; 32] = [
    0xa08a28ac, 0x28208a20, 0x02088288, 0x800228a2, 0x20a00a08, 0x80282088, 0x800800a2, 0x08028228,
    0x0a20a082, 0x22880020, 0x28020800, 0x88208082, 0x02022020, 0x08828028, 0x8008a202, 0x20880880,
    0x20000a00, 0x0a082008, 0x82820802, 0x00800a20, 0x0028208a, 0x20080822, 0x20808020, 0x02208088,
    0x20080022, 0x28a00a00, 0x8a200080, 0x008a2000, 0x00808800, 0x02082202, 0x80820880, 0x28220020,
];

fn _janus_utils_primes_is_prime(nr: u32) -> bool {
    (c_prime_table[(nr >> 5) as usize] & (1 << (nr & 31))) != 0
}

fn _janus_utils_primes_get_previous(mut nr: u32) -> u32 {
    while nr != 0 {
        if _janus_utils_primes_is_prime(nr) {
            return nr;
        }
        nr -= 1;
    }
    0
}
