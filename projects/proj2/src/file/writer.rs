use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::{ChunkMeta, FileError, FileFooter, HEADER_LEN, MAGIC, RowGroupMeta, VERSION};
use crate::data::{Column, RecordBatch, Schema};
use crate::encoding;

pub struct FileWriter {
    out: BufWriter<File>,
    pos: u64,
    footer: FileFooter,
    scratch: Vec<u8>,
}

impl FileWriter {
    pub fn create(path: impl AsRef<Path>, schema: &Schema) -> Result<Self, FileError> {
        let file = File::create(path)?;
        let mut out = BufWriter::with_capacity(1 << 20, file);
        out.write_all(MAGIC)?;
        out.write_all(&VERSION.to_le_bytes())?;
        Ok(Self {
            out,
            pos: HEADER_LEN as u64,
            footer: FileFooter {
                schema: schema.clone(),
                row_groups: Vec::new(),
            },
            scratch: Vec::new(),
        })
    }

    pub fn schema(&self) -> &Schema {
        &self.footer.schema
    }

    pub fn write_batch(&mut self, batch: &RecordBatch) -> Result<(), FileError> {
        let schema = &self.footer.schema;
        assert_eq!(
            batch.num_columns(),
            schema.len(),
            "batch does not match schema"
        );
        debug_assert!(batch.is_consistent());
        if batch.num_rows() == 0 {
            return Ok(());
        }

        let mut chunks = Vec::with_capacity(schema.len());
        for (i, (col, field)) in batch.columns().iter().zip(schema.fields()).enumerate() {
            assert_eq!(col.column_type(), field.ty, "column {i} type mismatch");

            let uncompressed_len = match col {
                Column::Int64(v) => v.len() * 8,
                Column::Varchar(v) => {
                    if v.data.len() > u32::MAX as usize {
                        return Err(FileError::ChunkTooLarge {
                            column: i,
                            bytes: v.data.len(),
                            max: u32::MAX as usize,
                        });
                    }
                    v.data.len()
                }
            };

            self.scratch.clear();
            encoding::encode_column(col, &mut self.scratch);
            self.out.write_all(&self.scratch)?;

            chunks.push(ChunkMeta {
                offset: self.pos,
                compressed_len: self.scratch.len() as u64,
                uncompressed_len: uncompressed_len as u64,
            });
            self.pos += self.scratch.len() as u64;
        }

        self.footer.row_groups.push(RowGroupMeta {
            rows: batch.num_rows() as u64,
            chunks,
        });
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), FileError> {
        self.scratch.clear();
        self.footer.encode(&mut self.scratch);
        self.out.write_all(&self.scratch)?;
        self.out
            .write_all(&(self.scratch.len() as u32).to_le_bytes())?;
        self.out.write_all(MAGIC)?;
        self.out.flush()?;
        self.out.get_ref().sync_all()?;
        Ok(())
    }
}
