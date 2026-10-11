use md5::Digest;

pub fn sum_block_hash(result: &mut [u8], block_hash: &[u8; 16]) {
    for i in 0..16 {
        result[i] ^= block_hash[i];
    }
}

#[allow(dead_code)]
pub fn stretch_md5(block_hash: &[u8], k: u64) -> Digest {
    let mut result = md5::compute(block_hash);
    for _ in 1..k {
        result = md5::compute(*result);
    }
    result
}
