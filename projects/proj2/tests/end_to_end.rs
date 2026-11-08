use proj2::{
    Column, ColumnBuffer, DataType, Field, FileReader, FileWriter, Schema, StrBuffer, StrColumn,
};

#[test]
fn writes_and_reads_back_the_same_data() {
    let schema = Schema::new(vec![
        Field::new("int", DataType::Int64),
        Field::new("str", DataType::Varchar),
    ]);
    let mut writer = FileWriter::new(Vec::new(), schema).unwrap();

    let numbers_0: Vec<i64> = vec![1, -5, i64::MIN, i64::MAX, 0];
    let text_0 = "Namiナミ日本";
    // Kanji are 3 bytes in utf8
    let offsets_0: Vec<u32> = vec![0, 0, 4, 10, 16, 16];
    writer
        .write_in_chunks(&[
            Column::Int64(&numbers_0),
            Column::Varchar(StrColumn::new(text_0, &offsets_0)),
        ])
        .unwrap();

    let numbers_1: Vec<i64> = vec![7, 8];
    let text_1 = "One Piece is real";
    let offsets_1: Vec<u32> = vec![0, 9, 17];
    writer
        .write_in_chunks(&[
            Column::Int64(&numbers_1),
            Column::Varchar(StrColumn::new(text_1, &offsets_1)),
        ])
        .unwrap();

    let bytes = writer.finish().unwrap();

    let reader = FileReader::new(&bytes).unwrap();
    assert_eq!(reader.footer.group_rows, vec![5, 2]);
    assert_eq!(reader.footer.schema.fields[0].name, "int");
    assert_eq!(reader.footer.schema.fields[1].name, "str");

    let mut numbers = ColumnBuffer::new(DataType::Int64);
    let mut text = ColumnBuffer::new(DataType::Varchar);

    reader.read_column(0, 0, &mut numbers).unwrap();
    reader.read_column(0, 1, &mut text).unwrap();
    assert_eq!(numbers, ColumnBuffer::Int64(numbers_0));
    assert_eq!(
        text,
        ColumnBuffer::Varchar(StrBuffer {
            data: text_0.to_string(),
            offsets: offsets_0,
        })
    );

    reader.read_column(1, 0, &mut numbers).unwrap();
    reader.read_column(1, 1, &mut text).unwrap();
    assert_eq!(numbers, ColumnBuffer::Int64(numbers_1));
    assert_eq!(
        text,
        ColumnBuffer::Varchar(StrBuffer {
            data: text_1.to_string(),
            offsets: offsets_1,
        })
    );
}
