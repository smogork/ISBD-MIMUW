use super::{Column, Schema};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordBatch {
    columns: Vec<Column>,
}

impl RecordBatch {
    pub fn new(schema: &Schema) -> Self {
        Self {
            columns: schema.column_types().map(Column::new).collect(),
        }
    }

    pub fn from_columns(columns: Vec<Column>) -> Self {
        if let Some(first) = columns.first() {
            let n = first.len();
            for (i, c) in columns.iter().enumerate() {
                assert_eq!(c.len(), n, "column {i} has {} rows, expected {n}", c.len());
            }
        }
        Self { columns }
    }

    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    pub fn num_rows(&self) -> usize {
        self.columns.first().map_or(0, Column::len)
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn column(&self, i: usize) -> &Column {
        &self.columns[i]
    }

    pub fn column_mut(&mut self, i: usize) -> &mut Column {
        &mut self.columns[i]
    }

    pub fn columns_mut(&mut self) -> &mut [Column] {
        &mut self.columns
    }

    pub fn clear(&mut self) {
        for c in &mut self.columns {
            c.clear();
        }
    }

    pub fn is_consistent(&self) -> bool {
        let n = self.num_rows();
        self.columns.iter().all(|c| c.len() == n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ColumnType, Field, VarcharColumn};

    fn schema() -> Schema {
        Schema::new(vec![
            Field::new("id", ColumnType::Int64),
            Field::new("name", ColumnType::Varchar),
        ])
    }

    #[test]
    fn new_from_schema_is_empty_with_right_types() {
        let b = RecordBatch::new(&schema());
        assert_eq!(b.num_columns(), 2);
        assert_eq!(b.num_rows(), 0);
        assert_eq!(b.column(0).column_type(), ColumnType::Int64);
        assert_eq!(b.column(1).column_type(), ColumnType::Varchar);
        assert!(b.is_consistent());
    }

    #[test]
    fn fill_and_clear() {
        let mut b = RecordBatch::new(&schema());
        {
            let cols = b.columns_mut();
            if let Column::Int64(v) = &mut cols[0] {
                v.extend([1, 2, 3]);
            }
            if let Column::Varchar(v) = &mut cols[1] {
                v.push(b"a");
                v.push(b"");
                v.push(b"ccc");
            }
        }
        assert_eq!(b.num_rows(), 3);
        assert!(b.is_consistent());

        b.clear();
        assert_eq!(b.num_rows(), 0);
        assert!(b.is_consistent());
    }

    #[test]
    fn from_columns_ok() {
        let mut s = VarcharColumn::new();
        s.push(b"x");
        s.push(b"y");
        let b = RecordBatch::from_columns(vec![Column::Int64(vec![7, 8]), Column::Varchar(s)]);
        assert_eq!(b.num_rows(), 2);
        assert!(b.is_consistent());
    }

    #[test]
    #[should_panic(expected = "column 1 has 1 rows, expected 2")]
    fn from_columns_mismatch_panics() {
        let mut s = VarcharColumn::new();
        s.push(b"x");
        RecordBatch::from_columns(vec![Column::Int64(vec![7, 8]), Column::Varchar(s)]);
    }

    #[test]
    fn inconsistent_detected() {
        let mut b = RecordBatch::new(&schema());
        if let Column::Int64(v) = b.column_mut(0) {
            v.push(1);
        }
        assert!(!b.is_consistent());
    }
}
