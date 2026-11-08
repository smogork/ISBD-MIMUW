use super::EncodingError;

pub const MAX_VARINT_LEN: usize = 10;

#[inline]
pub fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

#[inline]
pub fn unzigzag(u: u64) -> i64 {
    ((u >> 1) as i64) ^ -((u & 1) as i64)
}

#[inline]
pub fn put(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

#[inline]
pub fn get(src: &[u8]) -> Result<(u64, usize), EncodingError> {
    let mut result = 0u64;
    let mut shift = 0u32;
    for (i, &b) in src.iter().enumerate() {
        if i >= MAX_VARINT_LEN {
            return Err(EncodingError::VarintTooLong);
        }
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
    }
    Err(EncodingError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zigzag_small_values() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zigzag(-1), 1);
        assert_eq!(zigzag(1), 2);
        assert_eq!(zigzag(-2), 3);
        assert_eq!(zigzag(2), 4);
        assert_eq!(zigzag(i64::MAX), u64::MAX - 1);
        assert_eq!(zigzag(i64::MIN), u64::MAX);
        for v in [
            0,
            1,
            -1,
            42,
            -42,
            i64::MAX,
            i64::MIN,
            i64::MAX - 1,
            i64::MIN + 1,
        ] {
            assert_eq!(unzigzag(zigzag(v)), v);
        }
    }

    #[test]
    fn varint_boundaries() {
        let cases: [(u64, usize); 7] = [
            (0, 1),
            (127, 1),
            (128, 2),
            (16_383, 2),
            (16_384, 3),
            (u32::MAX as u64, 5),
            (u64::MAX, 10),
        ];
        for (v, expected_len) in cases {
            let mut out = Vec::new();
            put(&mut out, v);
            assert_eq!(out.len(), expected_len, "len for {v}");
            let (back, n) = get(&out).unwrap();
            assert_eq!(back, v);
            assert_eq!(n, expected_len);
        }
    }

    #[test]
    fn varint_concatenated() {
        let mut out = Vec::new();
        put(&mut out, 300);
        put(&mut out, 1);
        put(&mut out, u64::MAX);
        let (a, n1) = get(&out).unwrap();
        let (b, n2) = get(&out[n1..]).unwrap();
        let (c, n3) = get(&out[n1 + n2..]).unwrap();
        assert_eq!((a, b, c), (300, 1, u64::MAX));
        assert_eq!(n1 + n2 + n3, out.len());
    }

    #[test]
    fn varint_errors() {
        assert!(matches!(get(&[]), Err(EncodingError::Truncated)));
        assert!(matches!(get(&[0x80, 0x80]), Err(EncodingError::Truncated)));
        assert!(matches!(
            get(&[0xff; 11]),
            Err(EncodingError::VarintTooLong)
        ));
    }
}
