mod common;

use common::*;
use isbd::data::{Column, RecordBatch};
use isbd::encoding::{encode_column, varint};
use isbd::file::FileWriter;

#[test]
fn varint_bytes() {
    let mut out = Vec::new();
    varint::put(&mut out, 0);
    varint::put(&mut out, 127);
    varint::put(&mut out, 128);
    varint::put(&mut out, 300);
    assert_eq!(out, vec![0, 127, 0x80, 0x01, 0xAC, 0x02]);
}

#[test]
fn zigzag_values() {
    assert_eq!(varint::zigzag(0), 0);
    assert_eq!(varint::zigzag(-1), 1);
    assert_eq!(varint::zigzag(1), 2);
    assert_eq!(varint::zigzag(-64), 127);
    assert_eq!(varint::zigzag(64), 128);
}

#[test]
fn int64_chunk_bytes() {
    let mut out = Vec::new();
    encode_column(&Column::Int64(vec![10, 12, 11]), &mut out);
    assert_eq!(out, vec![20, 4, 1]);

    out.clear();
    encode_column(&Column::Int64(vec![1000, 1001, 1002]), &mut out);
    assert_eq!(out, vec![0xD0, 0x0F, 2, 2]);

    out.clear();
    encode_column(&Column::Int64(vec![0; 5]), &mut out);
    assert_eq!(out, vec![0; 5]);

    out.clear();
    encode_column(&Column::Int64(vec![i64::MIN]), &mut out);
    assert_eq!(
        out,
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]
    );

    out.clear();
    encode_column(&Column::Int64(vec![]), &mut out);
    assert!(out.is_empty());
}

#[test]
fn varchar_chunk_bytes() {
    let mut out = Vec::new();
    encode_column(&Column::Varchar(varchar(&[b"ab", b"", b"c"])), &mut out);
    assert_eq!(out, vec![3, 2, 0, 1, 0x30, b'a', b'b', b'c']);

    out.clear();
    encode_column(&Column::Varchar(varchar(&[])), &mut out);
    assert_eq!(out, vec![0]);

    out.clear();
    encode_column(&Column::Varchar(varchar(&[b"", b"", b""])), &mut out);
    assert_eq!(out, vec![0, 0, 0, 0]);

    out.clear();
    let long = vec![b'x'; 200];
    encode_column(&Column::Varchar(varchar(&[&long])), &mut out);
    assert_eq!(&out[..4], &[0xC8, 0x01, 0xC8, 0x01]);
    assert!(out.len() < 4 + 200);
}

#[test]
fn whole_file_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.isbd");

    let mut w = FileWriter::create(&path, &schema2()).unwrap();
    w.write_batch(&golden_batch()).unwrap();
    w.finish().unwrap();

    let actual = std::fs::read(&path).unwrap();
    assert_eq!(actual, golden_file_bytes());
}

#[test]
fn empty_file_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.isbd");
    FileWriter::create(&path, &schema2())
        .unwrap()
        .finish()
        .unwrap();

    let mut expected = Vec::new();
    expected.extend_from_slice(b"ISBD");
    expected.extend_from_slice(&1u16.to_le_bytes());
    let footer: Vec<u8> = vec![1, 0, 2, 0, 2, b'i', b'd', 1, 4, b'n', b'a', b'm', b'e', 0];
    expected.extend_from_slice(&footer);
    expected.extend_from_slice(&(footer.len() as u32).to_le_bytes());
    expected.extend_from_slice(b"ISBD");

    assert_eq!(std::fs::read(&path).unwrap(), expected);
}

#[test]
fn two_row_groups_have_independent_deltas() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rg.isbd");
    let mut w = FileWriter::create(&path, &schema2()).unwrap();
    w.write_batch(&RecordBatch::from_columns(vec![
        Column::Int64(vec![100]),
        Column::Varchar(varchar(&[b""])),
    ]))
    .unwrap();
    w.write_batch(&RecordBatch::from_columns(vec![
        Column::Int64(vec![100]),
        Column::Varchar(varchar(&[b""])),
    ]))
    .unwrap();
    w.finish().unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[6..8], &[0xC8, 0x01]);
    assert_eq!(&bytes[8..10], &[0, 0]);
    assert_eq!(&bytes[10..12], &[0xC8, 0x01]);
    assert_eq!(&bytes[12..14], &[0, 0]);
}
