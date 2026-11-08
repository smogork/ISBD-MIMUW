pub mod int64;
pub mod varchar;
pub mod varint;

use crate::data::{Column, ColumnType};

#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("chunk truncated")]
    Truncated,
    #[error("chunk has {0} trailing bytes")]
    TrailingBytes(usize),
    #[error("varint too long")]
    VarintTooLong,
    #[error("lz4 decompression failed: {0}")]
    Lz4(#[from] lz4_flex::block::DecompressError),
    #[error("decompressed size mismatch: expected {expected}, got {got}")]
    SizeMismatch { expected: usize, got: usize },
}

pub fn encode_column(col: &Column, out: &mut Vec<u8>) {
    match col {
        Column::Int64(v) => int64::encode(v, out),
        Column::Varchar(v) => varchar::encode(v, out),
    }
}

pub fn decode_column(
    ty: ColumnType,
    src: &[u8],
    rows: usize,
    out: &mut Column,
) -> Result<(), EncodingError> {
    match (ty, out) {
        (ColumnType::Int64, Column::Int64(v)) => int64::decode(src, rows, v),
        (ColumnType::Varchar, Column::Varchar(v)) => varchar::decode(src, rows, v),
        _ => panic!("column type mismatch in decode_column"),
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    pub struct Rng(u64);

    impl Rng {
        pub fn new(seed: u64) -> Self {
            Self(seed.max(1))
        }
        pub fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545F4914F6CDD1D)
        }
        pub fn next_i64(&mut self) -> i64 {
            self.next_u64() as i64
        }
        pub fn below(&mut self, n: u64) -> u64 {
            self.next_u64() % n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::VarcharColumn;

    #[test]
    fn dispatch_roundtrip_both_types() {
        let mut out = Vec::new();

        let ints = Column::Int64(vec![5, -3, 1_000_000, i64::MIN]);
        encode_column(&ints, &mut out);
        let mut back = Column::new(ColumnType::Int64);
        decode_column(ColumnType::Int64, &out, 4, &mut back).unwrap();
        assert_eq!(back, ints);

        out.clear();
        let mut s = VarcharColumn::new();
        s.push(b"hello");
        s.push(b"");
        s.push(b"abc");
        let strs = Column::Varchar(s);
        encode_column(&strs, &mut out);
        let mut back = Column::new(ColumnType::Varchar);
        decode_column(ColumnType::Varchar, &out, 3, &mut back).unwrap();
        assert_eq!(back, strs);
    }

    #[test]
    #[should_panic(expected = "column type mismatch")]
    fn dispatch_type_mismatch_panics() {
        let mut out = Column::new(ColumnType::Int64);
        let _ = decode_column(ColumnType::Varchar, &[], 0, &mut out);
    }
}
