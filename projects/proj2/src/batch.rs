use crate::layout::ColumnBuffer;

// a single row group in the file.
pub const BATCH_SIZE: usize = 1024;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Batch {
    pub rows: usize,
    pub columns: Vec<ColumnBuffer>,
}

impl Batch {
    pub fn row(&self, index: usize) -> Row<'_> {
        Row {
            columns: &self.columns,
            index,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Row<'b> {
    pub columns: &'b [ColumnBuffer],
    pub index: usize,
}

impl<'b> Row<'b> {
    pub fn int64(&self, column: usize) -> i64 {
        match &self.columns[column] {
            ColumnBuffer::Int64(values) => values[self.index],
            other => panic!("column {column} is {:?}, not Int64", other.data_type()),
        }
    }

    pub fn boolean(&self, column: usize) -> bool {
        match &self.columns[column] {
            ColumnBuffer::Boolean(values) => values[self.index],
            other => panic!("column {column} is {:?}, not Boolean", other.data_type()),
        }
    }

    pub fn varchar(&self, column: usize) -> &'b str {
        match &self.columns[column] {
            ColumnBuffer::Varchar(values) => {
                let start = values.offsets[self.index] as usize;
                let end = values.offsets[self.index + 1] as usize;
                &values.data[start..end]
            }
            other => panic!("column {column} is {:?}, not Varchar", other.data_type()),
        }
    }
}
