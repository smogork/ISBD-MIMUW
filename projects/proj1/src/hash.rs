use md5::Digest;

pub fn sum_block_hash(result: &mut Vec<u8>, block_hash: &[u8; 16]) {
    for i in 0..16 {
        result[i] ^= block_hash[i];
    }
}

pub fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).unwrap_or(default.to_string()).parse().unwrap()
}

pub fn stretch_md5(block_hash: &[u8], k: u64) -> Digest {
    let mut result = md5::compute(block_hash);
    for _ in 1..k {
        result = md5::compute(*result);
    }
    result
}
