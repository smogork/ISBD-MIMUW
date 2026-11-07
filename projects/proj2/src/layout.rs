use std::num::TryFromIntError;

use crate::encoding::write_varint;

pub const NAME: [u8; 7] = *b"PARKIET";
pub const VERSION: u16 = 1;
pub const HEADER_LEN: usize = 9;

pub fn header() -> [u8; HEADER_LEN] {
    let [v0, v1] = VERSION.to_le_bytes();
    let [n0, n1, n2, n3, n4, n5, n6] = NAME;
    [n0, n1, n2, n3, n4, n5, n6, v0, v1]
}

#[derive(Debug)]
pub struct Footer<'a> {
    pub schema: Schema<'a>,
    // Rows of each row group
    pub group_rows: Vec<u64>,
    // `group_rows.len() * schema length` chunks
    pub chunks: Vec<Chunk>,
}

impl<'a> Footer<'a> {
    pub fn encode(&self, destination: &mut Vec<u8>) -> Result<(), TryFromIntError> {
        let start = destination.len();
        write_varint(destination, self.schema.fields.len() as u64);
        for field in &self.schema.fields {
            write_varint(destination, field.name.len() as u64);
            destination.extend_from_slice(field.name.as_bytes());
            destination.push(field.data_type.byte_repr());
        }
        write_varint(destination, self.group_rows.len() as u64);
        let chunks = self.chunks.chunks(self.schema.fields.len().max(1));
        for (&rows, group) in self.group_rows.iter().zip(chunks) {
            write_varint(destination, rows);
            for chunk in group {
                write_varint(destination, chunk.offset);
                write_varint(destination, chunk.len);
            }
        }
        let len = destination.len() - start;
        let u32_len = u32::try_from(len)?;
        destination.extend_from_slice(&u32_len.to_le_bytes());
        destination.extend_from_slice(&NAME);
        Ok(())
    }

}

#[derive(Debug)]
pub struct Chunk {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug)]
pub struct Schema<'a> {
    fields: Vec<Field<'a>>,
}

impl<'a> Schema<'a> {
    pub fn new(fields: Vec<Field<'a>>) -> Self {
        // FIXME: validate
        Self { fields }
    }
}

#[derive(Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub data_type: DataType,
}

impl<'a> Field<'a> {
    pub const fn new(name: &'a str, data_type: DataType) -> Self {
        Self { name, data_type }
    }
}

#[derive(Debug)]
pub enum DataType {
    Int64,
    Varchar,
}

impl DataType {
    pub fn byte_repr(&self) -> u8{
        match self {
            DataType::Int64 => 0u8,
            DataType::Varchar => 1u8,
        }
    }
}

// Borrowed column: a slice of i64 and StrColumn
#[derive(Debug)]
pub enum Column<'a> {
    Int64(&'a [i64]),
    Varchar(StrColumn<'a>),
}

impl<'a> Column<'a> {
    pub fn len(&self) -> usize {
        match self {
            Self::Int64(values) => values.len(),
            Self::Varchar(values) => values.len(),
        }
    }
}

// Varchar column is one string where offsets indicate rows.
#[derive(Debug)]
pub struct StrColumn<'a> {
    pub data: &'a str,
    pub offsets: &'a [u32],
}

impl<'a> StrColumn<'a> {
    pub fn new(data: &'a str, offsets: &'a [u32]) -> Self {
        // FIXME: validate
        Self { data, offsets }
    }
    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}
