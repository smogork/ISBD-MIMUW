use super::{EncodingError, varint};
use crate::data::VarcharColumn;

pub fn encode(col: &VarcharColumn, out: &mut Vec<u8>) {
    let data = &col.data;
    varint::put(out, data.len() as u64);

    out.reserve(col.len());
    for w in col.offsets.windows(2) {
        varint::put(out, (w[1] - w[0]) as u64);
    }

    if data.is_empty() {
        return;
    }
    let start = out.len();
    let max = lz4_flex::block::get_maximum_output_size(data.len());
    out.resize(start + max, 0);
    let n = lz4_flex::block::compress_into(data, &mut out[start..])
        .expect("output buffer sized by get_maximum_output_size");
    out.truncate(start + n);
}

pub fn decode(src: &[u8], rows: usize, out: &mut VarcharColumn) -> Result<(), EncodingError> {
    out.clear();
    let mut pos = 0usize;

    let (data_len, n) = varint::get(src)?;
    pos += n;
    let data_len = data_len as usize;

    out.offsets.reserve(rows);
    let mut acc = 0u64;
    for _ in 0..rows {
        let (len, n) = varint::get(&src[pos..])?;
        pos += n;
        acc += len;
        out.offsets.push(acc as u32);
    }
    if acc as usize != data_len {
        return Err(EncodingError::SizeMismatch {
            expected: data_len,
            got: acc as usize,
        });
    }

    if data_len == 0 {
        if pos != src.len() {
            return Err(EncodingError::TrailingBytes(src.len() - pos));
        }
        return Ok(());
    }

    out.data.resize(data_len, 0);
    let got = lz4_flex::block::decompress_into(&src[pos..], &mut out.data)?;
    if got != data_len {
        return Err(EncodingError::SizeMismatch {
            expected: data_len,
            got,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::test_util::Rng;

    fn col(strs: &[&[u8]]) -> VarcharColumn {
        let mut c = VarcharColumn::new();
        for s in strs {
            c.push(s);
        }
        c
    }

    fn roundtrip(c: &VarcharColumn) -> Vec<u8> {
        let mut out = Vec::new();
        encode(c, &mut out);
        let mut back = VarcharColumn::new();
        decode(&out, c.len(), &mut back).unwrap();
        assert_eq!(&back, c);
        out
    }

    #[test]
    fn basic() {
        roundtrip(&col(&[b"hello", b"", b"abc"]));
    }

    #[test]
    fn zero_rows_is_one_byte() {
        let out = roundtrip(&col(&[]));
        assert_eq!(out, vec![0]);
    }

    #[test]
    fn all_empty_strings_skip_lz4() {
        let out = roundtrip(&col(&[b"", b"", b""]));
        assert_eq!(out, vec![0, 0, 0, 0]);
    }

    #[test]
    fn repetitive_data_compresses_well() {
        let strs: Vec<Vec<u8>> = (0..1000)
            .map(|i| format!("name{}", i % 10).into_bytes())
            .collect();
        let refs: Vec<&[u8]> = strs.iter().map(Vec::as_slice).collect();
        let c = col(&refs);
        let out = roundtrip(&c);
        assert!(
            out.len() < c.data.len() / 4,
            "{} vs {}",
            out.len(),
            c.data.len()
        );
    }

    #[test]
    fn random_bytes_roundtrip() {
        let mut rng = Rng::new(3);
        let mut c = VarcharColumn::new();
        let mut buf = Vec::new();
        for _ in 0..500 {
            let len = rng.below(200) as usize;
            buf.clear();
            buf.extend((0..len).map(|_| rng.next_u64() as u8));
            c.push(&buf);
        }
        roundtrip(&c);
    }

    #[test]
    fn single_huge_string() {
        let big = vec![b'x'; 5_000_000];
        let c = col(&[&big]);
        let out = roundtrip(&c);
        assert!(out.len() < 50_000);
    }

    #[test]
    fn truncated_lengths() {
        let mut out = Vec::new();
        encode(&col(&[b"ab", b"cd"]), &mut out);
        out.truncate(2);
        let mut back = VarcharColumn::new();
        assert!(matches!(
            decode(&out, 2, &mut back),
            Err(EncodingError::Truncated)
        ));
    }

    #[test]
    fn too_many_rows_requested() {
        let mut out = Vec::new();
        encode(&col(&[b"ab", b"cd"]), &mut out);
        let mut back = VarcharColumn::new();
        assert!(matches!(
            decode(&out, 3, &mut back),
            Err(EncodingError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn lengths_not_matching_data_len() {
        let mut out = Vec::new();
        encode(&col(&[b"ab", b"cd"]), &mut out);
        let mut back = VarcharColumn::new();
        assert!(matches!(
            decode(&out, 1, &mut back),
            Err(EncodingError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn corrupted_lz4_block() {
        let mut out = Vec::new();
        encode(&col(&[b"hello world hello world hello world"]), &mut out);
        let last = out.len() - 1;
        out.truncate(last);
        let mut back = VarcharColumn::new();
        assert!(decode(&out, 1, &mut back).is_err());
    }

    #[test]
    fn decode_reuses_buffers() {
        let c = col(&[&b"some string"[..]; 100]);
        let mut out = Vec::new();
        encode(&c, &mut out);
        let mut back = VarcharColumn::new();
        decode(&out, 100, &mut back).unwrap();
        let (cd, co) = (back.data.capacity(), back.offsets.capacity());
        decode(&out, 100, &mut back).unwrap();
        assert_eq!((back.data.capacity(), back.offsets.capacity()), (cd, co));
    }
}
