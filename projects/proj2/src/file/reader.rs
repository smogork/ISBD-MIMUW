use std::fs::File;
use std::path::Path;

use memmap2::{Advice, Mmap};

use super::{FileError, FileFooter, HEADER_LEN, MAGIC, TRAILER_LEN, VERSION};
use crate::data::{ColumnType, RecordBatch, Schema};
use crate::encoding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    cols: Vec<(usize, ColumnType)>,
}

impl Projection {
    pub fn len(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.cols.iter().map(|&(i, _)| i)
    }
}

pub struct FileReader {
    mmap: Mmap,
    footer: FileFooter,
}

impl FileReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FileError> {
        let file = File::open(path)?;
        // SAFETY: files are immutable once written; nothing truncates or
        // modifies them while mapped. Unlinking is fine, the mapping keeps the inode alive.
        let mmap = unsafe { Mmap::map(&file)? };
        let _ = mmap.advise(Advice::Sequential);

        let len = mmap.len();
        if len < HEADER_LEN + TRAILER_LEN || &mmap[..4] != MAGIC || &mmap[len - 4..] != MAGIC {
            return Err(FileError::BadMagic);
        }
        let version = u16::from_le_bytes([mmap[4], mmap[5]]);
        if version != VERSION {
            return Err(FileError::BadVersion(version));
        }

        let footer_len = u32::from_le_bytes(mmap[len - 8..len - 4].try_into().unwrap()) as usize;
        let footer_end = len - TRAILER_LEN;
        let footer_start = footer_end
            .checked_sub(footer_len)
            .filter(|&s| s >= HEADER_LEN)
            .ok_or_else(|| FileError::Corrupt("footer length out of range".into()))?;
        let footer = FileFooter::decode(&mmap[footer_start..footer_end])?;

        for (gi, g) in footer.row_groups.iter().enumerate() {
            for (ci, c) in g.chunks.iter().enumerate() {
                let end = c.offset.checked_add(c.compressed_len);
                if c.offset < HEADER_LEN as u64 || end.is_none_or(|e| e > footer_start as u64) {
                    return Err(FileError::Corrupt(format!(
                        "chunk (row group {gi}, column {ci}) out of file bounds"
                    )));
                }
            }
        }

        Ok(Self { mmap, footer })
    }

    pub fn schema(&self) -> &Schema {
        &self.footer.schema
    }

    pub fn footer(&self) -> &FileFooter {
        &self.footer
    }

    pub fn num_row_groups(&self) -> usize {
        self.footer.row_groups.len()
    }

    pub fn row_group_rows(&self, idx: usize) -> usize {
        self.footer.row_groups[idx].rows as usize
    }

    pub fn total_rows(&self) -> u64 {
        self.footer.total_rows()
    }

    pub fn projection(&self, indices: &[usize]) -> Result<Projection, FileError> {
        let schema = &self.footer.schema;
        let cols = indices
            .iter()
            .map(|&i| {
                if i < schema.len() {
                    Ok((i, schema.field(i).ty))
                } else {
                    Err(FileError::BadProjection {
                        index: i,
                        columns: schema.len(),
                    })
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Projection { cols })
    }

    pub fn full_projection(&self) -> Projection {
        let cols = self.footer.schema.column_types().enumerate().collect();
        Projection { cols }
    }

    pub fn new_batch(&self, proj: &Projection) -> RecordBatch {
        let indices: Vec<usize> = proj.indices().collect();
        RecordBatch::new(&self.footer.schema.project(&indices))
    }

    pub fn read_row_group(
        &self,
        idx: usize,
        proj: &Projection,
        out: &mut RecordBatch,
    ) -> Result<(), FileError> {
        let group = &self.footer.row_groups[idx];
        debug_assert_eq!(
            out.num_columns(),
            proj.len(),
            "batch does not match projection"
        );
        let rows = group.rows as usize;

        for (j, &(col_idx, ty)) in proj.cols.iter().enumerate() {
            let meta = &group.chunks[col_idx];
            let start = meta.offset as usize;
            let src = &self.mmap[start..start + meta.compressed_len as usize];
            encoding::decode_column(ty, src, rows, out.column_mut(j))?;
        }
        debug_assert!(out.is_consistent());
        Ok(())
    }
}
