mod common;

use common::*;
use isbd::csv_import::{CsvBatchReader, CsvOptions};
use isbd::data::{Column, RecordBatch};
use isbd::file::{FileReader, FileWriter, HEADER_LEN, TRAILER_LEN};
use isbd::metrics;
use isbd::pipeline;

#[test]
fn roundtrip_multiple_row_groups() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    let schema = schema4();
    let mut rng = Rng::new(42);
    let batches: Vec<RecordBatch> = [1000, 1, 777]
        .iter()
        .enumerate()
        .map(|(i, &n)| random_batch4(&mut rng, n, i as i64 * 10_000))
        .collect();

    let mut w = FileWriter::create(&path, &schema).unwrap();
    for b in &batches {
        w.write_batch(b).unwrap();
    }
    w.finish().unwrap();

    let r = FileReader::open(&path).unwrap();
    assert_eq!(r.schema(), &schema);
    assert_eq!(r.num_row_groups(), 3);
    assert_eq!(r.total_rows(), 1778);

    let all = r.full_projection();
    let mut out = r.new_batch(&all);
    for (i, expected) in batches.iter().enumerate() {
        r.read_row_group(i, &all, &mut out).unwrap();
        assert_eq!(&out, expected, "row group {i}");
    }
}

#[test]
fn roundtrip_extreme_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    let big = vec![b'q'; 3_000_000];
    let mut rng = Rng::new(3);
    let mut random_bytes = Vec::new();
    for _ in 0..1000 {
        random_bytes.push(rng.next_u64() as u8);
    }
    let batch = RecordBatch::from_columns(vec![
        Column::Int64(vec![i64::MIN, i64::MAX, 0, -1, 1, i64::MAX, i64::MIN]),
        Column::Varchar(varchar(&[
            b"",
            &big,
            b"",
            &random_bytes,
            b"\n,\"",
            b"\xff\xfe",
            b"",
        ])),
        Column::Varchar(varchar(&[&b""[..]; 7])),
        Column::Int64(vec![0; 7]),
    ]);

    let mut w = FileWriter::create(&path, &schema4()).unwrap();
    w.write_batch(&batch).unwrap();
    w.finish().unwrap();

    let r = FileReader::open(&path).unwrap();
    let all = r.full_projection();
    let mut out = r.new_batch(&all);
    r.read_row_group(0, &all, &mut out).unwrap();
    assert_eq!(out, batch);
}

#[test]
fn projection_and_reorder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    let mut rng = Rng::new(1);
    let batch = random_batch4(&mut rng, 100, 0);

    let mut w = FileWriter::create(&path, &schema4()).unwrap();
    w.write_batch(&batch).unwrap();
    w.finish().unwrap();

    let r = FileReader::open(&path).unwrap();
    let proj = r.projection(&[3, 1, 3]).unwrap();
    let mut out = r.new_batch(&proj);
    r.read_row_group(0, &proj, &mut out).unwrap();
    assert_eq!(out.num_columns(), 3);
    assert_eq!(out.column(0), batch.column(3));
    assert_eq!(out.column(1), batch.column(1));
    assert_eq!(out.column(2), batch.column(3));
}

#[test]
fn empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    FileWriter::create(&path, &schema4())
        .unwrap()
        .finish()
        .unwrap();

    let r = FileReader::open(&path).unwrap();
    assert_eq!(r.schema(), &schema4());
    assert_eq!(r.num_row_groups(), 0);
    assert_eq!(r.total_rows(), 0);
    assert_eq!(
        std::fs::metadata(&path).unwrap().len() as usize,
        HEADER_LEN + r.footer().encoded_len() + TRAILER_LEN
    );
}

#[test]
fn empty_batches_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    let mut w = FileWriter::create(&path, &schema4()).unwrap();
    w.write_batch(&RecordBatch::new(&schema4())).unwrap();
    w.finish().unwrap();
    assert_eq!(FileReader::open(&path).unwrap().num_row_groups(), 0);
}

#[test]
fn reader_reuses_batch_buffers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.isbd");
    let mut rng = Rng::new(5);
    let mut w = FileWriter::create(&path, &schema4()).unwrap();
    for _ in 0..5 {
        w.write_batch(&random_batch4(&mut rng, 500, 0)).unwrap();
    }
    w.finish().unwrap();

    let r = FileReader::open(&path).unwrap();
    let all = r.full_projection();
    let mut out = r.new_batch(&all);
    r.read_row_group(0, &all, &mut out).unwrap();
    r.read_row_group(1, &all, &mut out).unwrap();
    let caps = |b: &RecordBatch| -> Vec<usize> {
        b.columns()
            .iter()
            .map(|c| match c {
                Column::Int64(v) => v.capacity(),
                Column::Varchar(v) => v.data.capacity(),
            })
            .collect()
    };
    let before = caps(&out);
    for i in 2..5 {
        r.read_row_group(i, &all, &mut out).unwrap();
    }
    assert_eq!(caps(&out), before);
}

#[test]
fn csv_to_file_to_batch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("people.isbd");
    let schema = schema4();
    let csv = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pit/tables/people/data.csv"
    );

    let opts = CsvOptions {
        has_header: true,
        max_rows: 2,
        ..Default::default()
    };
    let mut src = CsvBatchReader::open(csv, &schema, opts).unwrap();
    let mut w = FileWriter::create(&path, &schema).unwrap();
    let mut batch = RecordBatch::new(&schema);
    while src.next_batch(&mut batch).unwrap() {
        w.write_batch(&batch).unwrap();
    }
    w.finish().unwrap();

    let r = FileReader::open(&path).unwrap();
    assert_eq!(r.num_row_groups(), 3);
    assert_eq!(r.total_rows(), 5);
    let proj = r.projection(&[2]).unwrap();
    let mut out = r.new_batch(&proj);
    r.read_row_group(1, &proj, &mut out).unwrap();
    assert_eq!(strs(&out, 0)[1], b"Pilsudski");
}

#[test]
fn csv_through_pipeline_equals_sequential() {
    let dir = tempfile::tempdir().unwrap();
    let schema = schema4();
    let csv = sample_csv();
    let opts = || CsvOptions {
        has_header: true,
        max_rows: 700,
        ..Default::default()
    };

    let seq = dir.path().join("seq.isbd");
    {
        let mut src = CsvBatchReader::open(csv, &schema, opts()).unwrap();
        let mut w = FileWriter::create(&seq, &schema).unwrap();
        let mut batch = RecordBatch::new(&schema);
        while src.next_batch(&mut batch).unwrap() {
            w.write_batch(&batch).unwrap();
        }
        w.finish().unwrap();
    }

    let par = dir.path().join("par.isbd");
    {
        let mut src = CsvBatchReader::open(csv, &schema, opts()).unwrap();
        let mut w = FileWriter::create(&par, &schema).unwrap();
        let stats = pipeline::run(
            &schema,
            3,
            |b| src.next_batch(b).map_err(|e| e.to_string()),
            |b| w.write_batch(b).map_err(|e| e.to_string()),
        )
        .unwrap();
        w.finish().unwrap();
        assert_eq!(stats.rows, 5000);
        assert_eq!(stats.batches, 8);
    }

    assert_eq!(std::fs::read(&seq).unwrap(), std::fs::read(&par).unwrap());

    let r = FileReader::open(&par).unwrap();
    let (m, rows) = metrics::compute(&r).unwrap();
    assert_eq!(rows, 5000);
    assert_eq!(m[0].average(), Some(2500.5));
}
