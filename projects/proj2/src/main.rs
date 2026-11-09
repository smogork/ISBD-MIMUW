use std::{env, fs, io};

use proj2::{BATCH_SIZE, Column, ColumnBuffer, DataType, Field, FileReader, FileWriter, Schema, StrBuffer};

const GROUP_COUNT: usize = 4;
const ROWS_PER_GROUP: usize = BATCH_SIZE;
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
        Field::new("is_pirate", DataType::Boolean),
        Field::new("name", DataType::Varchar),
        Field::new("comment", DataType::Varchar),
    ]);
    let mut writer = FileWriter::create(path, schema)?;

    let mut random = fastrand::Rng::with_seed(1);
    let mut ids: Vec<i64> = Vec::new();
    let mut pirates: Vec<bool> = Vec::new();
    let mut names = StrBuffer::default();
    let mut comments = StrBuffer::default();

    for group in 0..GROUP_COUNT {
        ids.clear();
        pirates.clear();
        names.data.clear();
        names.offsets.clear();
        names.offsets.push(0);
        comments.data.clear();
        comments.offsets.clear();
        comments.offsets.push(0);

        for row in 0..ROWS_PER_GROUP {
            let id = (group * ROWS_PER_GROUP + row) as i64 + 1;
            ids.push(id);
            pirates.push(random.u8(..4) == 0);
            names.data.push_str(NAMES[row % NAMES.len()]);
            names.offsets.push(names.data.len() as u32);
            comments.data.push_str(COMMENTS[row % COMMENTS.len()]);
            comments.offsets.push(comments.data.len() as u32);
        }

        writer.write_in_chunks(&[
            Column::Int64(&ids),
            Column::Boolean(&pirates),
            Column::Varchar(names.view()),
            Column::Varchar(comments.view()),
        ])?;
    }
    writer.finish()?;
    println!("wrote {} rows to {path}", GROUP_COUNT * ROWS_PER_GROUP);
    Ok(())
}

fn print_metrics(path: &str) -> io::Result<()> {
    let file = fs::File::open(path)?;
    // SAFETY: we are the sole reader of the file.
    let bytes = unsafe { memmap2::Mmap::map(&file)? };
    let reader = FileReader::new(&bytes)?;
    let fields = &reader.footer.schema.fields;

    let mut buffers: Vec<ColumnBuffer> = Vec::new();
    for field in fields {
        buffers.push(ColumnBuffer::new(field.data_type));
    }
    let mut sums: Vec<i128> = vec![0; fields.len()];
    let mut true_counts: Vec<u64> = vec![0; fields.len()];
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
                ColumnBuffer::Boolean(values) => {
                    for &value in values {
                        true_counts[column] += u64::from(value);
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
            DataType::Boolean => {
                if row_count == 0 {
                    println!("{} has no rows", field.name);
                } else {
                    let p = true_counts[column] as f64 / row_count as f64;
                    println!("{} BOOLEAN: binary entropy = {}", field.name, binary_entropy(p));
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

fn binary_entropy(p: f64) -> f64 {
    let term = |x: f64| if x == 0.0 { 0.0 } else { -x * x.log2() };
    term(p) + term(1.0 - p)
}
