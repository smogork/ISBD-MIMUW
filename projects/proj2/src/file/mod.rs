pub mod footer;
pub mod reader;
pub mod writer;

pub use footer::{ChunkMeta, FileFooter, RowGroupMeta};
pub use reader::{FileReader, Projection};
pub use writer::FileWriter;

pub const MAGIC: &[u8; 4] = b"ISBD";
pub const VERSION: u16 = 1;
pub const HEADER_LEN: usize = 4 + 2;
pub const TRAILER_LEN: usize = 4 + 4;

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encoding error: {0}")]
    Encoding(#[from] crate::encoding::EncodingError),
    #[error("not an ISBD file (bad magic)")]
    BadMagic,
    #[error("unsupported file version {0}")]
    BadVersion(u16),
    #[error("corrupt file: {0}")]
    Corrupt(String),
    #[error("projection refers to column {index}, file has {columns} columns")]
    BadProjection { index: usize, columns: usize },
    #[error("chunk too large: column {column} has {bytes} bytes, max {max}")]
    ChunkTooLarge {
        column: usize,
        bytes: usize,
        max: usize,
    },
}
