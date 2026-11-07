use std::io;
use std::num::TryFromIntError;

use crate::encoding::{read_varint, write_varint};

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

    pub fn parse(file: &'a [u8]) -> io::Result<Self> {
        let trailer_start = file.len() - NAME.len() - size_of::<u32>();
        let len_bytes = [
            file[trailer_start],
            file[trailer_start + 1],
            file[trailer_start + 2],
            file[trailer_start + 3],
        ];
        let footer_len = u32::from_le_bytes(len_bytes) as usize;
        let mut source = &file[trailer_start - footer_len..trailer_start];

        let column_count = read_varint(&mut source) as usize;
        let mut fields = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let name_len = read_varint(&mut source) as usize;
            let name = std::str::from_utf8(&source[..name_len]).map_err(io::Error::other)?;
            let data_type = DataType::from_byte_repr(source[name_len]);
            source = &source[name_len + 1..];
            fields.push(Field::new(name, data_type));
        }

        let group_count = read_varint(&mut source) as usize;
        let mut group_rows = Vec::with_capacity(group_count);
        let mut chunks = Vec::with_capacity(group_count * column_count);
        for _ in 0..group_count {
            group_rows.push(read_varint(&mut source));
            for _ in 0..column_count {
                let offset = read_varint(&mut source);
                let len = read_varint(&mut source);
                chunks.push(Chunk { offset, len });
            }
        }

        Ok(Self {
            schema: Schema::new(fields),
            group_rows,
            chunks,
        })
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug)]
pub struct Schema<'a> {
    pub fields: Vec<Field<'a>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_byte_repr(byte: u8) -> Self {
        match byte {
            0 => DataType::Int64,
            1 => DataType::Varchar,
            other => panic!("unknown data type byte {other}"),
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StrBuffer {
    pub data: String,
    pub offsets: Vec<u32>,
}

impl StrBuffer {
    pub fn view(&self) -> StrColumn<'_> {
        StrColumn::new(&self.data, &self.offsets)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ColumnBuffer {
    Int64(Vec<i64>),
    Varchar(StrBuffer),
}

impl ColumnBuffer {
    pub fn new(data_type: DataType) -> Self {
        match data_type {
            DataType::Int64 => Self::Int64(Vec::new()),
            DataType::Varchar => Self::Varchar(StrBuffer::default()),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Self::Int64(_) => DataType::Int64,
            Self::Varchar(_) => DataType::Varchar,
        }
    }

    pub fn view(&self) -> Column<'_> {
        match self {
            Self::Int64(values) => Column::Int64(values),
            Self::Varchar(values) => Column::Varchar(values.view()),
        }
    }
}
