use super::{FileError, VERSION};
use crate::data::{ColumnType, Field, Schema};
use crate::encoding::varint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkMeta {
    pub offset: u64,
    pub compressed_len: u64,
    pub uncompressed_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroupMeta {
    pub rows: u64,
    pub chunks: Vec<ChunkMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFooter {
    pub schema: Schema,
    pub row_groups: Vec<RowGroupMeta>,
}

fn type_tag(ty: ColumnType) -> u8 {
    match ty {
        ColumnType::Int64 => 0,
        ColumnType::Varchar => 1,
    }
}

fn type_from_tag(tag: u8) -> Result<ColumnType, FileError> {
    match tag {
        0 => Ok(ColumnType::Int64),
        1 => Ok(ColumnType::Varchar),
        t => Err(FileError::Corrupt(format!("unknown column type tag {t}"))),
    }
}

struct Cursor<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn varint(&mut self) -> Result<u64, FileError> {
        let (v, n) = varint::get(&self.src[self.pos..])
            .map_err(|_| FileError::Corrupt("footer truncated".into()))?;
        self.pos += n;
        Ok(v)
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8], FileError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.src.len())
            .ok_or_else(|| FileError::Corrupt("footer truncated".into()))?;
        let out = &self.src[self.pos..end];
        self.pos = end;
        Ok(out)
    }
}

impl FileFooter {
    pub fn total_rows(&self) -> u64 {
        self.row_groups.iter().map(|g| g.rows).sum()
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&VERSION.to_le_bytes());
        varint::put(out, self.schema.len() as u64);
        for f in self.schema.fields() {
            out.push(type_tag(f.ty));
            varint::put(out, f.name.len() as u64);
            out.extend_from_slice(f.name.as_bytes());
        }
        varint::put(out, self.row_groups.len() as u64);
        for g in &self.row_groups {
            varint::put(out, g.rows);
            debug_assert_eq!(g.chunks.len(), self.schema.len());
            for c in &g.chunks {
                varint::put(out, c.offset);
                varint::put(out, c.compressed_len);
                varint::put(out, c.uncompressed_len);
            }
        }
    }

    pub fn encoded_len(&self) -> usize {
        let mut buf = Vec::new();
        self.encode(&mut buf);
        buf.len()
    }

    pub fn decode(src: &[u8]) -> Result<Self, FileError> {
        let mut c = Cursor { src, pos: 0 };
        let ver = u16::from_le_bytes(c.bytes(2)?.try_into().unwrap());
        if ver != VERSION {
            return Err(FileError::BadVersion(ver));
        }

        let num_columns = c.varint()? as usize;
        let mut fields = Vec::with_capacity(num_columns);
        for _ in 0..num_columns {
            let ty = type_from_tag(c.bytes(1)?[0])?;
            let name_len = c.varint()? as usize;
            let name = std::str::from_utf8(c.bytes(name_len)?)
                .map_err(|_| FileError::Corrupt("column name is not utf-8".into()))?;
            fields.push(Field::new(name, ty));
        }
        let schema = Schema::new(fields);

        let num_groups = c.varint()? as usize;
        let mut row_groups = Vec::with_capacity(num_groups);
        for _ in 0..num_groups {
            let rows = c.varint()?;
            let mut chunks = Vec::with_capacity(num_columns);
            for _ in 0..num_columns {
                chunks.push(ChunkMeta {
                    offset: c.varint()?,
                    compressed_len: c.varint()?,
                    uncompressed_len: c.varint()?,
                });
            }
            row_groups.push(RowGroupMeta { rows, chunks });
        }

        if c.pos != src.len() {
            return Err(FileError::Corrupt(format!(
                "{} trailing footer bytes",
                src.len() - c.pos
            )));
        }
        Ok(Self { schema, row_groups })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FileFooter {
        FileFooter {
            schema: Schema::new(vec![
                Field::new("id", ColumnType::Int64),
                Field::new("name", ColumnType::Varchar),
            ]),
            row_groups: vec![
                RowGroupMeta {
                    rows: 1000,
                    chunks: vec![
                        ChunkMeta {
                            offset: 6,
                            compressed_len: 1003,
                            uncompressed_len: 8000,
                        },
                        ChunkMeta {
                            offset: 1009,
                            compressed_len: 2500,
                            uncompressed_len: 7000,
                        },
                    ],
                },
                RowGroupMeta {
                    rows: 7,
                    chunks: vec![
                        ChunkMeta {
                            offset: 3509,
                            compressed_len: 9,
                            uncompressed_len: 56,
                        },
                        ChunkMeta {
                            offset: 3518,
                            compressed_len: 30,
                            uncompressed_len: 40,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn roundtrip() {
        let f = sample();
        let mut buf = Vec::new();
        f.encode(&mut buf);
        assert_eq!(buf.len(), f.encoded_len());
        assert_eq!(FileFooter::decode(&buf).unwrap(), f);
        assert_eq!(f.total_rows(), 1007);
    }

    #[test]
    fn empty_footer() {
        let f = FileFooter {
            schema: Schema::new(vec![]),
            row_groups: vec![],
        };
        let mut buf = Vec::new();
        f.encode(&mut buf);
        assert_eq!(buf, vec![1, 0, 0, 0]);
        assert_eq!(FileFooter::decode(&buf).unwrap(), f);
    }

    #[test]
    fn rejects_bad_version_truncation_and_trailing() {
        let mut buf = Vec::new();
        sample().encode(&mut buf);

        let mut bad = buf.clone();
        bad[0] = 99;
        assert!(matches!(
            FileFooter::decode(&bad),
            Err(FileError::BadVersion(99))
        ));

        assert!(matches!(
            FileFooter::decode(&buf[..buf.len() - 1]),
            Err(FileError::Corrupt(_))
        ));
        assert!(matches!(
            FileFooter::decode(&buf[..5]),
            Err(FileError::Corrupt(_))
        ));

        let mut extra = buf.clone();
        extra.push(0);
        assert!(matches!(
            FileFooter::decode(&extra),
            Err(FileError::Corrupt(_))
        ));

        let mut badtag = buf.clone();
        badtag[3] = 7;
        assert!(matches!(
            FileFooter::decode(&badtag),
            Err(FileError::Corrupt(_))
        ));
    }
}
