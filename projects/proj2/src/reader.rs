use std::io;

use crate::{
    batch::Batch,
    encoding::decode,
    layout::{ColumnBuffer, Footer},
};

#[derive(Debug)]
pub struct FileReader<'a> {
    source: &'a [u8],
    pub footer: Footer<'a>,
}

impl<'a> FileReader<'a> {
    pub fn new(source: &'a [u8]) -> io::Result<Self> {
        let footer = Footer::parse(source)?;
        Ok(Self { source, footer })
    }

    pub fn read_column(
        &self,
        group: usize,
        column: usize,
        destination: &mut ColumnBuffer,
    ) -> io::Result<()> {
        let data_type = self.footer.schema.fields[column].data_type;
        if destination.data_type() != data_type {
            *destination = ColumnBuffer::new(data_type);
        }
        let chunk = &self.footer.chunks[group * self.footer.schema.fields.len() + column];
        let start = chunk.offset as usize;
        let chunk_bytes = &self.source[start..start + chunk.len as usize];
        decode(chunk_bytes, self.footer.group_rows[group] as usize, destination)
    }

    pub fn read_batch(&self, group: usize, destination: &mut Batch) -> io::Result<()> {
        let fields = &self.footer.schema.fields;
        destination.rows = self.footer.group_rows[group] as usize;
        destination.columns.truncate(fields.len());
        for field in &fields[destination.columns.len()..] {
            destination.columns.push(ColumnBuffer::new(field.data_type));
        }
        for column in 0..fields.len() {
            self.read_column(group, column, &mut destination.columns[column])?;
        }
        Ok(())
    }
}
