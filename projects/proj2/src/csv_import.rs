use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::data::{Column, ColumnType, RecordBatch, Schema};

#[derive(Debug, Clone)]
pub struct CsvOptions {
    pub has_header: bool,
    pub delimiter: u8,
    pub max_rows: usize,
    pub max_bytes: usize,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            has_header: false,
            delimiter: b',',
            max_rows: 64 * 1024,
            max_bytes: 1 << 20,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("line {line}: expected {expected} fields, got {got}")]
    ColumnCount {
        line: u64,
        expected: usize,
        got: usize,
    },
    #[error("line {line}, column {column}: cannot parse {value:?} as INT64")]
    ParseInt {
        line: u64,
        column: usize,
        value: String,
    },
}

pub struct CsvBatchReader<R: Read> {
    reader: csv::Reader<R>,
    record: csv::ByteRecord,
    types: Vec<ColumnType>,
    opts: CsvOptions,
}

impl CsvBatchReader<File> {
    pub fn open(
        path: impl AsRef<Path>,
        schema: &Schema,
        opts: CsvOptions,
    ) -> Result<Self, CsvError> {
        let file = File::open(path)?;
        Ok(Self::new(file, schema, opts))
    }
}

impl<R: Read> CsvBatchReader<R> {
    pub fn new(input: R, schema: &Schema, opts: CsvOptions) -> Self {
        let reader = csv::ReaderBuilder::new()
            .has_headers(opts.has_header)
            .delimiter(opts.delimiter)
            .flexible(true)
            .from_reader(input);
        Self {
            reader,
            record: csv::ByteRecord::new(),
            types: schema.column_types().collect(),
            opts,
        }
    }

    pub fn next_batch(&mut self, batch: &mut RecordBatch) -> Result<bool, CsvError> {
        debug_assert_eq!(batch.num_columns(), self.types.len());
        batch.clear();
        let mut bytes = 0usize;

        while batch.num_rows() < self.opts.max_rows && bytes < self.opts.max_bytes {
            if !self.reader.read_byte_record(&mut self.record)? {
                break;
            }
            let line = self.record.position().map_or(0, |p| p.line());

            if self.record.len() != self.types.len() {
                return Err(CsvError::ColumnCount {
                    line,
                    expected: self.types.len(),
                    got: self.record.len(),
                });
            }

            for (i, (col, field)) in batch
                .columns_mut()
                .iter_mut()
                .zip(self.record.iter())
                .enumerate()
            {
                match col {
                    Column::Int64(v) => {
                        let value = parse_i64(field).ok_or_else(|| CsvError::ParseInt {
                            line,
                            column: i,
                            value: String::from_utf8_lossy(field).into_owned(),
                        })?;
                        v.push(value);
                        bytes += 8;
                    }
                    Column::Varchar(v) => {
                        v.push(field);
                        bytes += field.len() + 4;
                    }
                }
            }
        }

        debug_assert!(batch.is_consistent());
        Ok(batch.num_rows() > 0)
    }
}

fn parse_i64(field: &[u8]) -> Option<i64> {
    let s = std::str::from_utf8(field).ok()?;
    s.trim_ascii().parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Field;
    use std::io::Cursor;

    fn schema() -> Schema {
        Schema::new(vec![
            Field::new("id", ColumnType::Int64),
            Field::new("name", ColumnType::Varchar),
        ])
    }

    fn reader(input: &str, opts: CsvOptions) -> CsvBatchReader<Cursor<Vec<u8>>> {
        CsvBatchReader::new(Cursor::new(input.as_bytes().to_vec()), &schema(), opts)
    }

    fn ints(b: &RecordBatch, i: usize) -> Vec<i64> {
        match b.column(i) {
            Column::Int64(v) => v.clone(),
            _ => panic!("not int"),
        }
    }

    fn strs(b: &RecordBatch, i: usize) -> Vec<String> {
        match b.column(i) {
            Column::Varchar(v) => v
                .iter()
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect(),
            _ => panic!("not varchar"),
        }
    }

    #[test]
    fn header_and_batches_by_rows() {
        let opts = CsvOptions {
            has_header: true,
            max_rows: 2,
            ..Default::default()
        };
        let mut r = reader("id,name\n1,a\n2,b\n3,c\n", opts);
        let mut b = RecordBatch::new(&schema());

        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(ints(&b, 0), vec![1, 2]);
        assert_eq!(strs(&b, 1), vec!["a", "b"]);

        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(ints(&b, 0), vec![3]);
        assert_eq!(strs(&b, 1), vec!["c"]);

        assert!(!r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 0);
    }

    #[test]
    fn no_header_first_row_is_data() {
        let mut r = reader("7,x\n8,y\n", CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(ints(&b, 0), vec![7, 8]);
        assert_eq!(strs(&b, 1), vec!["x", "y"]);
    }

    #[test]
    fn quoting_commas_newlines_and_escaped_quotes() {
        let input = "1,\"a,b\"\n2,\"line1\nline2\"\n3,\"say \"\"hi\"\"\"\n";
        let mut r = reader(input, CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(strs(&b, 1), vec!["a,b", "line1\nline2", "say \"hi\""]);
    }

    #[test]
    fn empty_varchar_field_and_crlf() {
        let mut r = reader("1,\r\n2,b\r\n", CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(strs(&b, 1), vec!["", "b"]);
    }

    #[test]
    fn int_extremes_and_whitespace() {
        let input = format!("{},a\n{},b\n -5 ,c\n", i64::MIN, i64::MAX);
        let mut r = reader(&input, CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(ints(&b, 0), vec![i64::MIN, i64::MAX, -5]);
    }

    #[test]
    fn bad_int_reports_line_and_column() {
        let mut r = reader("1,a\nxyz,b\n", CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        let err = r.next_batch(&mut b).unwrap_err();
        match err {
            CsvError::ParseInt {
                line,
                column,
                value,
            } => {
                assert_eq!((line, column, value.as_str()), (2, 0, "xyz"));
            }
            other => panic!("unexpected: {other}"),
        }
    }

    #[test]
    fn empty_int_field_is_error() {
        let mut r = reader(",a\n", CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        assert!(matches!(
            r.next_batch(&mut b),
            Err(CsvError::ParseInt {
                line: 1,
                column: 0,
                ..
            })
        ));
    }

    #[test]
    fn wrong_field_count_reports_line() {
        let mut r = reader("1,a\n2,b,extra\n", CsvOptions::default());
        let mut b = RecordBatch::new(&schema());
        let err = r.next_batch(&mut b).unwrap_err();
        assert!(
            matches!(
                err,
                CsvError::ColumnCount {
                    line: 2,
                    expected: 2,
                    got: 3
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn empty_input_and_header_only() {
        let mut b = RecordBatch::new(&schema());

        let mut r = reader("", CsvOptions::default());
        assert!(!r.next_batch(&mut b).unwrap());

        let opts = CsvOptions {
            has_header: true,
            ..Default::default()
        };
        let mut r = reader("id,name\n", opts);
        assert!(!r.next_batch(&mut b).unwrap());
    }

    #[test]
    fn max_bytes_splits_but_never_empty() {
        let opts = CsvOptions {
            max_bytes: 30,
            ..Default::default()
        };
        let mut r = reader("1,aaaaaaaaaa\n2,bbbbbbbbbb\n3,cccccccccc\n", opts);
        let mut b = RecordBatch::new(&schema());

        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 2);
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 1);
        assert!(!r.next_batch(&mut b).unwrap());
    }

    #[test]
    fn huge_row_exceeding_limit_still_goes_through() {
        let big = "z".repeat(1000);
        let opts = CsvOptions {
            max_bytes: 16,
            ..Default::default()
        };
        let mut r = reader(&format!("1,{big}\n2,q\n"), opts);
        let mut b = RecordBatch::new(&schema());
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 1);
        assert_eq!(strs(&b, 1)[0].len(), 1000);
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 1);
    }

    #[test]
    fn buffers_are_reused_between_batches() {
        let mut rows = String::new();
        for i in 0..100 {
            rows.push_str(&format!("{i},name{i}\n"));
        }
        let opts = CsvOptions {
            max_rows: 50,
            ..Default::default()
        };
        let mut r = reader(&rows, opts);
        let mut b = RecordBatch::new(&schema());

        assert!(r.next_batch(&mut b).unwrap());
        let cap = match b.column(1) {
            Column::Varchar(v) => v.data.capacity(),
            _ => unreachable!(),
        };
        assert!(r.next_batch(&mut b).unwrap());
        let cap2 = match b.column(1) {
            Column::Varchar(v) => v.data.capacity(),
            _ => unreachable!(),
        };
        assert_eq!(cap, cap2);
    }

    #[test]
    fn reads_pit_people_table() {
        let schema = Schema::new(vec![
            Field::new("id", ColumnType::Int64),
            Field::new("name", ColumnType::Varchar),
            Field::new("surname", ColumnType::Varchar),
            Field::new("age", ColumnType::Int64),
        ]);
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../pit/tables/people/data.csv"
        );
        let opts = CsvOptions {
            has_header: true,
            ..Default::default()
        };
        let mut r = CsvBatchReader::open(path, &schema, opts).unwrap();
        let mut b = RecordBatch::new(&schema);
        assert!(r.next_batch(&mut b).unwrap());
        assert_eq!(b.num_rows(), 5);
        assert_eq!(ints(&b, 0), vec![1, 2, 3, 4, 5]);
        assert_eq!(strs(&b, 2)[3], "Pilsudski");
        assert!(!r.next_batch(&mut b).unwrap());
    }
}
