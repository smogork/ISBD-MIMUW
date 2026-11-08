use super::{EncodingError, varint};

pub fn encode(values: &[i64], out: &mut Vec<u8>) {
    out.reserve(values.len() * 2);
    let mut prev = 0i64;
    for &v in values {
        let delta = v.wrapping_sub(prev);
        varint::put(out, varint::zigzag(delta));
        prev = v;
    }
}

pub fn decode(src: &[u8], rows: usize, out: &mut Vec<i64>) -> Result<(), EncodingError> {
    out.clear();
    out.reserve(rows);
    let mut pos = 0usize;
    let mut prev = 0i64;
    for _ in 0..rows {
        let (zz, n) = varint::get(&src[pos..])?;
        pos += n;
        prev = prev.wrapping_add(varint::unzigzag(zz));
        out.push(prev);
    }
    if pos != src.len() {
        return Err(EncodingError::TrailingBytes(src.len() - pos));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::test_util::Rng;

    fn roundtrip(values: &[i64]) -> Vec<u8> {
        let mut out = Vec::new();
        encode(values, &mut out);
        let mut back = Vec::new();
        decode(&out, values.len(), &mut back).unwrap();
        assert_eq!(back, values);
        out
    }

    #[test]
    fn empty() {
        assert!(roundtrip(&[]).is_empty());
    }

    #[test]
    fn extremes_and_wraparound() {
        roundtrip(&[i64::MIN, i64::MAX, i64::MIN, 0, i64::MAX, -1, 1]);
    }

    #[test]
    fn sorted_sequence_is_one_byte_per_value() {
        let values: Vec<i64> = (1_000_000..1_001_000).collect();
        let out = roundtrip(&values);
        assert_eq!(out.len(), 3 + 999);
    }

    #[test]
    fn constant_column_is_tiny() {
        let values = vec![12345i64; 10_000];
        let out = roundtrip(&values);
        assert_eq!(out.len(), 3 + 9_999);
    }

    #[test]
    fn random_values() {
        let mut rng = Rng::new(7);
        let values: Vec<i64> = (0..10_000).map(|_| rng.next_i64()).collect();
        let out = roundtrip(&values);
        assert!(out.len() > 9 * values.len());
    }

    #[test]
    fn small_random_deltas_compress() {
        let mut rng = Rng::new(11);
        let mut v = 0i64;
        let values: Vec<i64> = (0..10_000)
            .map(|_| {
                v += rng.below(100) as i64 - 50;
                v
            })
            .collect();
        let out = roundtrip(&values);
        assert!(out.len() <= values.len());
    }

    #[test]
    fn truncated_input() {
        let mut out = Vec::new();
        encode(&[1, 2, 3], &mut out);
        let mut back = Vec::new();
        assert!(matches!(
            decode(&out, 4, &mut back),
            Err(EncodingError::Truncated)
        ));
    }

    #[test]
    fn trailing_bytes() {
        let mut out = Vec::new();
        encode(&[1, 2, 3], &mut out);
        out.push(0);
        let mut back = Vec::new();
        assert!(matches!(
            decode(&out, 3, &mut back),
            Err(EncodingError::TrailingBytes(1))
        ));
    }

    #[test]
    fn decode_reuses_buffer() {
        let mut out = Vec::new();
        encode(&[1; 1000], &mut out);
        let mut back = Vec::new();
        decode(&out, 1000, &mut back).unwrap();
        let cap = back.capacity();
        decode(&out, 1000, &mut back).unwrap();
        assert_eq!(back.capacity(), cap);
    }
}
