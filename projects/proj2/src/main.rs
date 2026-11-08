use std::{env, fs, io};

use proj2::{Column, ColumnBuffer, DataType, Field, FileReader, FileWriter, Schema, StrBuffer};

const GROUP_COUNT: usize = 4;
const ROWS_PER_GROUP: usize = 1000;
const NAMES: [&str; 6] = [
    "Nami ナミ",
    "Usopp ウソップ",
    "Franky フランキー",
    "Garp ガープ",
    "Oda 尾田",
    "Eiichirō 栄一郎",
];
const COMMENTS: [&str; 4] = ["IMU イム様", "LOKI ロキ", "", "NÍÐHÖGGR"];

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 && args[1] == "generate" {
        generate(&args[2])
    } else if args.len() == 2 {
        print_metrics(&args[1])
    } else {
        eprintln!("usage: proj2 <file>  |  proj2 generate <file>");
        Ok(())
    }
}

fn generate(path: &str) -> io::Result<()> {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64),
        Field::new("amount", DataType::Int64),
        Field::new("name", DataType::Varchar),
        Field::new("comment", DataType::Varchar),
    ]);
    let mut writer = FileWriter::create(path, schema)?;

    let mut random = fastrand::Rng::with_seed(1);
    let mut ids: Vec<i64> = Vec::new();
    let mut amounts: Vec<i64> = Vec::new();
    let mut names = StrBuffer::default();
    let mut comments = StrBuffer::default();

    for group in 0..GROUP_COUNT {
        ids.clear();
        amounts.clear();
        names.data.clear();
        names.offsets.clear();
        names.offsets.push(0);
        comments.data.clear();
        comments.offsets.clear();
        comments.offsets.push(0);

        for row in 0..ROWS_PER_GROUP {
            let id = (group * ROWS_PER_GROUP + row) as i64 + 1;
            ids.push(id);
            amounts.push(random.i64(-61830..=915629));
            names.data.push_str(NAMES[row % NAMES.len()]);
            names.offsets.push(names.data.len() as u32);
            comments.data.push_str(COMMENTS[row % COMMENTS.len()]);
            comments.offsets.push(comments.data.len() as u32);
        }

        writer.write_in_chunks(&[
            Column::Int64(&ids),
            Column::Int64(&amounts),
            Column::Varchar(names.view()),
            Column::Varchar(comments.view()),
        ])?;
    }
    writer.finish()?;
    println!("wrote {} rows to {path}", GROUP_COUNT * ROWS_PER_GROUP);
    Ok(())
}

fn print_metrics(path: &str) -> io::Result<()> {
    let bytes = fs::read(path)?;
    let reader = FileReader::new(&bytes)?;
    let fields = &reader.footer.schema.fields;

    let mut buffers: Vec<ColumnBuffer> = Vec::new();
    for field in fields {
        buffers.push(ColumnBuffer::new(field.data_type));
    }
    let mut sums: Vec<i128> = vec![0; fields.len()];
    let mut ascii_counts: Vec<u64> = vec![0; fields.len()];
    let mut row_count: u64 = 0;

    for group in 0..reader.footer.group_rows.len() {
        row_count += reader.footer.group_rows[group];
        for column in 0..fields.len() {
            reader.read_column(group, column, &mut buffers[column])?;
            match &buffers[column] {
                ColumnBuffer::Int64(values) => {
                    for &value in values {
                        sums[column] += i128::from(value);
                    }
                }
                ColumnBuffer::Varchar(strings) => {
                    for byte in strings.data.bytes() {
                        if byte.is_ascii() {
                            ascii_counts[column] += 1;
                        }
                    }
                }
            }
        }
    }

    println!("{}: {} rows, {} columns", path, row_count, fields.len());
    for column in 0..fields.len() {
        let field = &fields[column];
        match field.data_type {
            DataType::Int64 => {
                if row_count == 0 {
                    println!("{} has no rows", field.name);
                } else {
                    let mean = sums[column] as f64 / row_count as f64;
                    println!("{} INT64: mean = {}", field.name, mean);
                }
            }
            DataType::Varchar => {
                println!(
                    "{} VARCHAR: ascii characters = {}",
                    field.name, ascii_counts[column]
                );
            }
        }
    }
    Ok(())
}
