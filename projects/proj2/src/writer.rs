use std::{fs::File, io::Write, path::Path};

use crate::{
    encoding::encode,
    layout::{Chunk, Column, Footer, HEADER_LEN, Schema, header},
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
        // FIXME: validate
        let rows = columns.first().map_or(0, Column::len);
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
