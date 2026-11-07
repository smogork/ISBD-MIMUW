use proj2::{Column, DataType, Field, FileWriter, Schema, StrColumn};

fn main() {
    let schema = Schema::new(vec![
        Field::new("a", DataType::Int64),
        Field::new("b", DataType::Varchar),
    ]);
    let mut writer = FileWriter::new(Vec::new(), schema).unwrap();
    let a: Vec<i64> = vec![1, 2, 3];
    let data: &str = "xyz";
    let offsets: &[u32] = &[0, 1, 2, 3];
    let b = StrColumn::new(data, offsets);

    writer
        .write_in_chunks(&[Column::Int64(&a), Column::Varchar(b)])
        .unwrap();

    let bytes: Vec<u8> = writer.finish().unwrap();
    print!("{:?}", bytes);
}
