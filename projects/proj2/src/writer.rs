use std::{fs::File, io::Write, path::Path};

use crate::{
    batch::BATCH_SIZE,
    encoding::encode,
    layout::{Chunk, Column, DataType, Footer, HEADER_LEN, Schema, header},
};
use std::io;

#[derive(Debug)]
pub struct FileWriter<'a, W: Write> {
    io_sink: W,
    footer: Footer<'a>,
    position: u64,
    buffer: Vec<u8>,
}

impl<'s> FileWriter<'s, File> {
    pub fn create(path: impl AsRef<Path>, schema: Schema<'s>) -> io::Result<Self> {
        Self::new(File::create(path)?, schema)
    }
}

impl<'s, W: Write> FileWriter<'s, W> {
    pub fn new(mut io_sink: W, schema: Schema<'s>) -> io::Result<Self> {
        io_sink.write_all(&header())?;
        Ok(Self {
            io_sink,
            footer: Footer {
                schema,
                group_rows: Vec::new(),
                chunks: Vec::new(),
            },
            position: HEADER_LEN as u64,
            buffer: Vec::new(),
        })
    }

    pub fn write_in_chunks(&mut self, columns: &[Column<'_>]) -> io::Result<()> {
        let fields = &self.footer.schema.fields;
        if columns.len() != fields.len() {
            return Err(io::Error::other("column count does not match the schema"));
        }
        for (column, field) in columns.iter().zip(fields) {
            let same_type = matches!(
                (column, field.data_type),
                (Column::Int64(_), DataType::Int64)
                    | (Column::Varchar(_), DataType::Varchar)
                    | (Column::Boolean(_), DataType::Boolean)
            );
            if !same_type {
                return Err(io::Error::other("column type does not match the schema"));
            }
            if let Column::Varchar(strings) = column {
                let offsets = strings.offsets;
                if offsets.first() != Some(&0)
                    || offsets.last() != Some(&(strings.data.len() as u32))
                {
                    return Err(io::Error::other("invalid varchar offsets"));
                }
                for i in 1..offsets.len() {
                    if offsets[i] < offsets[i - 1]
                        || !strings.data.is_char_boundary(offsets[i] as usize)
                    {
                        return Err(io::Error::other("invalid varchar offsets"));
                    }
                }
            }
        }
        let rows = columns.first().map_or(0, Column::len);
        if rows > BATCH_SIZE {
            return Err(io::Error::other("rows exceed BATCH_SIZE"));
        }
        if columns.iter().any(|column| column.len() != rows) {
            return Err(io::Error::other("columns have different lengths"));
        }
        for column in columns {
            self.buffer.clear();
            encode(column, &mut self.buffer);
            self.io_sink.write_all(&self.buffer)?;
            let len = self.buffer.len() as u64;
            self.footer.chunks.push(Chunk {
                offset: self.position,
                len,
            });
            self.position += len;
        }
        self.footer.group_rows.push(rows as u64);
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<W> {
        self.buffer.clear();
        self.footer.encode(&mut self.buffer).map_err(io::Error::other)?;
        self.io_sink.write_all(&self.buffer)?;
        self.io_sink.flush()?;
        Ok(self.io_sink)
    }
}
