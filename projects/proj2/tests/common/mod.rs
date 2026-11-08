#![allow(dead_code)]

use isbd::data::{Column, ColumnType, Field, RecordBatch, Schema, VarcharColumn};

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

pub fn schema4() -> Schema {
    Schema::new(vec![
        Field::new("id", ColumnType::Int64),
        Field::new("name", ColumnType::Varchar),
        Field::new("surname", ColumnType::Varchar),
        Field::new("age", ColumnType::Int64),
    ])
}

pub fn schema2() -> Schema {
    Schema::new(vec![
        Field::new("id", ColumnType::Int64),
        Field::new("name", ColumnType::Varchar),
    ])
}

pub fn varchar(strs: &[&[u8]]) -> VarcharColumn {
    let mut c = VarcharColumn::new();
    for s in strs {
        c.push(s);
    }
    c
}

pub fn random_batch4(rng: &mut Rng, rows: usize, base: i64) -> RecordBatch {
    let mut id = Vec::with_capacity(rows);
    let mut name = VarcharColumn::new();
    let mut surname = VarcharColumn::new();
    let mut age = Vec::with_capacity(rows);
    for i in 0..rows {
        id.push(base + i as i64);
        name.push(format!("name{}", rng.below(50)).as_bytes());
        let len = rng.below(30) as usize;
        surname.push(&vec![b'a' + (rng.below(26) as u8); len]);
        age.push(rng.next_i64() % 120);
    }
    RecordBatch::from_columns(vec![
        Column::Int64(id),
        Column::Varchar(name),
        Column::Varchar(surname),
        Column::Int64(age),
    ])
}

pub fn ints(b: &RecordBatch, i: usize) -> &[i64] {
    match b.column(i) {
        Column::Int64(v) => v,
        _ => panic!("column {i} is not INT64"),
    }
}

pub fn strs(b: &RecordBatch, i: usize) -> Vec<&[u8]> {
    match b.column(i) {
        Column::Varchar(v) => v.iter().collect(),
        _ => panic!("column {i} is not VARCHAR"),
    }
}

pub fn golden_file_bytes() -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(b"ISBD");
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&[20, 4, 1]);
    f.extend_from_slice(&[3, 2, 0, 1, 0x30, b'a', b'b', b'c']);
    let footer: Vec<u8> = vec![
        1, 0, 2, 0, 2, b'i', b'd', 1, 4, b'n', b'a', b'm', b'e', 1, 3, 6, 3, 24, 9, 8, 3,
    ];
    f.extend_from_slice(&footer);
    f.extend_from_slice(&(footer.len() as u32).to_le_bytes());
    f.extend_from_slice(b"ISBD");
    f
}

pub fn golden_batch() -> RecordBatch {
    RecordBatch::from_columns(vec![
        Column::Int64(vec![10, 12, 11]),
        Column::Varchar(varchar(&[b"ab", b"", b"c"])),
    ])
}

pub fn sample_csv() -> &'static std::path::Path {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.csv"));
    assert!(
        path.exists(),
        "{} is missing; run `make examples/sample.csv` (decompresses examples/sample.csv.gz)",
        path.display()
    );
    path
}
