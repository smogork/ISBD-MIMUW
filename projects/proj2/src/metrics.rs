use crate::data::{Column, ColumnType, RecordBatch};
use crate::file::{FileError, FileReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnMetric {
    Int64Avg { sum: i128, count: u64 },
    AsciiCount { count: u64 },
}

impl ColumnMetric {
    pub fn new(ty: ColumnType) -> Self {
        match ty {
            ColumnType::Int64 => Self::Int64Avg { sum: 0, count: 0 },
            ColumnType::Varchar => Self::AsciiCount { count: 0 },
        }
    }

    pub fn update(&mut self, col: &Column) {
        match (self, col) {
            (Self::Int64Avg { sum, count }, Column::Int64(v)) => {
                *sum += v.iter().map(|&x| x as i128).sum::<i128>();
                *count += v.len() as u64;
            }
            (Self::AsciiCount { count }, Column::Varchar(v)) => {
                *count += v.data.iter().filter(|&&b| b < 128).count() as u64;
            }
            _ => panic!("metric/column type mismatch"),
        }
    }

    pub fn average(&self) -> Option<f64> {
        match self {
            Self::Int64Avg { sum, count } if *count > 0 => {
                let c = *count as i128;
                Some((sum / c) as f64 + (sum % c) as f64 / c as f64)
            }
            _ => None,
        }
    }
}

pub fn compute(reader: &FileReader) -> Result<(Vec<ColumnMetric>, u64), FileError> {
    let schema = reader.schema();
    let mut metrics: Vec<ColumnMetric> = schema.column_types().map(ColumnMetric::new).collect();
    let projection = reader.full_projection();
    let mut batch: RecordBatch = reader.new_batch(&projection);
    let mut rows = 0u64;

    for g in 0..reader.num_row_groups() {
        reader.read_row_group(g, &projection, &mut batch)?;
        rows += batch.num_rows() as u64;
        for (m, c) in metrics.iter_mut().zip(batch.columns()) {
            m.update(c);
        }
    }
    Ok((metrics, rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::VarcharColumn;

    #[test]
    fn int_average_handles_extremes() {
        let mut m = ColumnMetric::new(ColumnType::Int64);
        m.update(&Column::Int64(vec![i64::MAX, i64::MAX]));
        m.update(&Column::Int64(vec![i64::MAX]));
        assert_eq!(
            m,
            ColumnMetric::Int64Avg {
                sum: 3 * i64::MAX as i128,
                count: 3
            }
        );
        assert_eq!(m.average(), Some(i64::MAX as f64));

        let mut m = ColumnMetric::new(ColumnType::Int64);
        m.update(&Column::Int64(vec![1, 2, 3, 4]));
        assert_eq!(m.average(), Some(2.5));
        assert_eq!(ColumnMetric::new(ColumnType::Int64).average(), None);
    }

    #[test]
    fn ascii_count_ignores_high_bytes() {
        let mut c = VarcharColumn::new();
        c.push(b"abc");
        c.push(b"");
        c.push("zażółć".as_bytes());
        let mut m = ColumnMetric::new(ColumnType::Varchar);
        m.update(&Column::Varchar(c));
        assert_eq!(m, ColumnMetric::AsciiCount { count: 3 + 2 });
        assert_eq!(m.average(), None);
    }
}
