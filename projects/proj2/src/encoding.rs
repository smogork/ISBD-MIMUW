use std::io;

use lz4_flex::block;

use crate::{
    StrColumn,
    layout::{Column, ColumnBuffer, StrBuffer},
};

pub fn encode<'c>(column: &Column<'c>, destination: &mut Vec<u8>) {
    match column {
        Column::Int64(values) => encode_int64(values, destination),
        Column::Varchar(values) => encode_varchar(values, destination),
    }
}

// Source for the encoding:
// https://protobuf.dev/programming-guides/encoding/
pub fn encode_int64(values: &[i64], destination: &mut Vec<u8>) {
    let mut previous = 0i64;
    for &value in values {
        let delta = value.wrapping_sub(previous);
        let zigzag = zigzag(delta);
        write_varint(destination, zigzag);
        previous = value;
    }
}

pub fn write_varint(destination: &mut Vec<u8>, mut value: u64) {
    while value >= (1 << 7) {
        destination.push(value as u8 | (1 << 7));
        value >>= 7;
    }
    destination.push(value as u8);
}

pub const fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

pub fn encode_varchar(values: &StrColumn<'_>, destination: &mut Vec<u8>) {
    for i in 1..values.offsets.len() {
        let size = values.offsets[i] - values.offsets[i-1];
        write_varint(destination, size as u64);
    }
    let data = values.data.as_bytes();
    let start = destination.len();
    destination.resize(start + block::get_maximum_output_size(data.len()), 0);
    let written = block::compress_into(data, &mut destination[start..]).expect("reserved enough space");
    destination.truncate(start + written);
}

pub fn decode(source: &[u8], rows: usize, destination: &mut ColumnBuffer) -> io::Result<()> {
    match destination {
        ColumnBuffer::Int64(values) => decode_int64(source, rows, values),
        ColumnBuffer::Varchar(values) => decode_varchar(source, rows, values)?,
    }
    Ok(())
}

pub fn decode_int64(mut source: &[u8], rows: usize, destination: &mut Vec<i64>) {
    destination.clear();
    let mut previous = 0i64;
    for _ in 0..rows {
        let delta = unzigzag(read_varint(&mut source));
        previous = previous.wrapping_add(delta);
        destination.push(previous);
    }
}

pub fn read_varint(source: &mut &[u8]) -> u64 {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        let byte = source[0];
        *source = &source[1..];
        value |= u64::from(byte & ((1 << 7) - 1)) << shift;
        if byte < (1 << 7) {
            return value;
        }
        shift += 7;
    }
}

pub const fn unzigzag(value: u64) -> i64 {
    (value >> 1) as i64 ^ -((value & 1) as i64)
}

pub fn decode_varchar(mut source: &[u8], rows: usize, destination: &mut StrBuffer) -> io::Result<()> {
    destination.offsets.clear();
    destination.offsets.push(0);
    let mut end = 0u32;
    for _ in 0..rows {
        end += read_varint(&mut source) as u32;
        destination.offsets.push(end);
    }
    let old_data = std::mem::take(&mut destination.data);
    let mut bytes = old_data.into_bytes();
    bytes.clear();
    bytes.resize(end as usize, 0);

    block::decompress_into(source, &mut bytes).map_err(io::Error::other)?;

    let data = String::from_utf8(bytes).map_err(io::Error::other)?;
    destination.data = data;
    Ok(())
}

