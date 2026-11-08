mod common;

use common::*;
use isbd::data::{Column, ColumnType};
use isbd::encoding::{EncodingError, decode_column};
use isbd::file::{FileError, FileReader};

fn write_temp(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("in.isbd");
    std::fs::write(&path, bytes).unwrap();
    (dir, path)
}

#[test]
fn int64_chunk_from_bytes() {
    let mut out = Column::new(ColumnType::Int64);
    decode_column(ColumnType::Int64, &[20, 4, 1], 3, &mut out).unwrap();
    assert_eq!(out, Column::Int64(vec![10, 12, 11]));

    decode_column(ColumnType::Int64, &[1, 1, 1], 3, &mut out).unwrap();
    assert_eq!(out, Column::Int64(vec![-1, -2, -3]));

    decode_column(ColumnType::Int64, &[0xD0, 0x0F], 1, &mut out).unwrap();
    assert_eq!(out, Column::Int64(vec![1000]));

    let mut bytes = vec![0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
    bytes.push(2);
    decode_column(ColumnType::Int64, &bytes, 2, &mut out).unwrap();
    assert_eq!(out, Column::Int64(vec![i64::MAX, i64::MIN]));
}

#[test]
fn int64_chunk_rejects_bad_bytes() {
    let mut out = Column::new(ColumnType::Int64);
    assert!(matches!(
        decode_column(ColumnType::Int64, &[20, 4], 3, &mut out),
        Err(EncodingError::Truncated)
    ));
    assert!(matches!(
        decode_column(ColumnType::Int64, &[20, 4, 1, 7], 3, &mut out),
        Err(EncodingError::TrailingBytes(1))
    ));
    assert!(matches!(
        decode_column(ColumnType::Int64, &[0x80, 0x80], 1, &mut out),
        Err(EncodingError::Truncated)
    ));
}

#[test]
fn varchar_chunk_from_bytes() {
    let mut out = Column::new(ColumnType::Varchar);
    decode_column(
        ColumnType::Varchar,
        &[3, 2, 0, 1, 0x30, b'a', b'b', b'c'],
        3,
        &mut out,
    )
    .unwrap();
    assert_eq!(out, Column::Varchar(varchar(&[b"ab", b"", b"c"])));

    decode_column(ColumnType::Varchar, &[0], 0, &mut out).unwrap();
    assert_eq!(out, Column::Varchar(varchar(&[])));

    decode_column(ColumnType::Varchar, &[0, 0, 0], 2, &mut out).unwrap();
    assert_eq!(out, Column::Varchar(varchar(&[b"", b""])));

    let lz4 = [0x16, b'a', 1, 0, 0x50, b'a', b'a', b'a', b'a', b'a'];
    let mut bytes = vec![16, 16];
    bytes.extend_from_slice(&lz4);
    decode_column(ColumnType::Varchar, &bytes, 1, &mut out).unwrap();
    assert_eq!(out, Column::Varchar(varchar(&[&[b'a'; 16]])));
}

#[test]
fn varchar_chunk_rejects_bad_bytes() {
    let mut out = Column::new(ColumnType::Varchar);
    assert!(matches!(
        decode_column(
            ColumnType::Varchar,
            &[4, 2, 0, 1, 0x30, b'a', b'b', b'c'],
            3,
            &mut out
        ),
        Err(EncodingError::SizeMismatch {
            expected: 4,
            got: 3
        })
    ));
    assert!(
        decode_column(
            ColumnType::Varchar,
            &[3, 2, 0, 1, 0x30, b'a', b'b'],
            3,
            &mut out
        )
        .is_err()
    );
    assert!(decode_column(ColumnType::Varchar, &[3, 2, 0, 1], 3, &mut out).is_err());
}

#[test]
fn golden_file_decodes_to_expected_batch() {
    let (_dir, path) = write_temp(&golden_file_bytes());
    let r = FileReader::open(&path).unwrap();

    assert_eq!(r.schema(), &schema2());
    assert_eq!(r.num_row_groups(), 1);
    assert_eq!(r.total_rows(), 3);
    assert_eq!(r.row_group_rows(0), 3);

    let proj = r.full_projection();
    let mut out = r.new_batch(&proj);
    r.read_row_group(0, &proj, &mut out).unwrap();
    assert_eq!(out, golden_batch());

    let proj = r.projection(&[1]).unwrap();
    let mut out = r.new_batch(&proj);
    r.read_row_group(0, &proj, &mut out).unwrap();
    assert_eq!(strs(&out, 0), vec![&b"ab"[..], &b""[..], &b"c"[..]]);
}

#[test]
fn hand_built_two_row_groups() {
    let mut f = Vec::new();
    f.extend_from_slice(b"ISBD\x01\x00");
    f.extend_from_slice(&[10]);
    f.extend_from_slice(&[2, 2, 0x20, b'x', b'y']);
    f.extend_from_slice(&[9, 1]);
    f.extend_from_slice(&[1, 0, 1, 0x10, b'z']);
    let footer: Vec<u8> = vec![
        1, 0, 2, 0, 2, b'i', b'd', 1, 4, b'n', b'a', b'm', b'e', 2, 1, 6, 1, 8, 7, 5, 2, 2, 12, 2,
        16, 14, 5, 1,
    ];
    f.extend_from_slice(&footer);
    f.extend_from_slice(&(footer.len() as u32).to_le_bytes());
    f.extend_from_slice(b"ISBD");

    let (_dir, path) = write_temp(&f);
    let r = FileReader::open(&path).unwrap();
    assert_eq!(r.num_row_groups(), 2);
    assert_eq!(r.total_rows(), 3);

    let proj = r.full_projection();
    let mut out = r.new_batch(&proj);
    r.read_row_group(0, &proj, &mut out).unwrap();
    assert_eq!(ints(&out, 0), &[5]);
    assert_eq!(strs(&out, 1), vec![&b"xy"[..]]);
    r.read_row_group(1, &proj, &mut out).unwrap();
    assert_eq!(ints(&out, 0), &[-5, -6]);
    assert_eq!(strs(&out, 1), vec![&b""[..], &b"z"[..]]);
}

#[test]
fn rejects_malformed_files() {
    let good = golden_file_bytes();

    let (_d, p) = write_temp(b"not an isbd file at all");
    assert!(matches!(FileReader::open(&p), Err(FileError::BadMagic)));

    let (_d, p) = write_temp(b"ISBD");
    assert!(matches!(FileReader::open(&p), Err(FileError::BadMagic)));

    let (_d, p) = write_temp(&good[..good.len() - 4]);
    assert!(matches!(FileReader::open(&p), Err(FileError::BadMagic)));

    let mut bad = good.clone();
    bad[4] = 9;
    let (_d, p) = write_temp(&bad);
    assert!(matches!(
        FileReader::open(&p),
        Err(FileError::BadVersion(9))
    ));

    let mut bad = good.clone();
    let n = bad.len();
    bad[n - 8..n - 4].copy_from_slice(&1000u32.to_le_bytes());
    let (_d, p) = write_temp(&bad);
    assert!(matches!(FileReader::open(&p), Err(FileError::Corrupt(_))));

    let mut bad = good.clone();
    let pos = bad.windows(3).position(|w| w == [6, 3, 24]).unwrap();
    bad[pos] = 200;
    let (_d, p) = write_temp(&bad);
    assert!(matches!(FileReader::open(&p), Err(FileError::Corrupt(_))));

    let mut bad = good.clone();
    bad[6] = 0x80 | 20;
    let (_d, p) = write_temp(&bad);
    let r = FileReader::open(&p).unwrap();
    let proj = r.full_projection();
    let mut out = r.new_batch(&proj);
    assert!(r.read_row_group(0, &proj, &mut out).is_err());
}
