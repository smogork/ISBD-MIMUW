use lz4_flex::block;

use crate::{StrColumn, layout::Column};

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

