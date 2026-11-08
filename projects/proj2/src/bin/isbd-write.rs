use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use isbd::csv_import::{CsvBatchReader, CsvOptions};
use isbd::data::{ColumnType, Field, RecordBatch, Schema};
use isbd::file::FileWriter;
use isbd::pipeline;

#[derive(Parser)]
#[command(about = "Serialize a CSV file into the ISBD columnar format")]
struct Args {
    /// Input CSV file
    input: String,
    /// Output ISBD file
    output: String,
    /// Schema as `name:TYPE,...`, e.g. `id:INT64,name:VARCHAR`
    #[arg(short, long, conflicts_with = "schema_file")]
    schema: Option<String>,
    /// Schema file with one `name TYPE` per line
    #[arg(long)]
    schema_file: Option<String>,
    /// Skip the first CSV row (header)
    #[arg(long)]
    header: bool,
    /// Maximum rows per row group
    #[arg(long, default_value_t = 64 * 1024)]
    max_rows: usize,
    /// Approximate maximum bytes per row group
    #[arg(long, default_value_t = 1 << 20)]
    max_bytes: usize,
    /// Batches in flight between parser and writer (0 = single thread)
    #[arg(long, default_value_t = 2)]
    depth: usize,
}

fn parse_type(s: &str) -> Result<ColumnType, String> {
    match s.trim().to_ascii_uppercase().as_str() {
        "INT64" => Ok(ColumnType::Int64),
        "VARCHAR" => Ok(ColumnType::Varchar),
        other => Err(format!(
            "unknown column type {other:?} (expected INT64 or VARCHAR)"
        )),
    }
}

fn parse_schema(args: &Args) -> Result<Schema, String> {
    let mut fields = Vec::new();
    if let Some(spec) = &args.schema {
        for item in spec.split(',') {
            let (name, ty) = item
                .split_once(':')
                .ok_or_else(|| format!("bad schema item {item:?}, expected name:TYPE"))?;
            fields.push(Field::new(name.trim(), parse_type(ty)?));
        }
    } else if let Some(path) = &args.schema_file {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut it = line.split_whitespace();
            let name = it.next().unwrap();
            let ty = it
                .next()
                .ok_or_else(|| format!("bad schema line {line:?}"))?;
            fields.push(Field::new(name, parse_type(ty)?));
        }
    } else {
        return Err("schema required: --schema or --schema-file".into());
    }
    if fields.is_empty() {
        return Err("schema has no columns".into());
    }
    Ok(Schema::new(fields))
}

fn run(args: &Args) -> Result<(), String> {
    let schema = parse_schema(args)?;
    let opts = CsvOptions {
        has_header: args.header,
        max_rows: args.max_rows,
        max_bytes: args.max_bytes,
        ..Default::default()
    };

    let start = Instant::now();
    let mut src = CsvBatchReader::open(&args.input, &schema, opts).map_err(|e| e.to_string())?;
    let mut writer = FileWriter::create(&args.output, &schema).map_err(|e| e.to_string())?;
    let (rows, groups) = if args.depth == 0 {
        let mut batch = RecordBatch::new(&schema);
        let (mut rows, mut groups) = (0u64, 0u64);
        while src.next_batch(&mut batch).map_err(|e| e.to_string())? {
            writer.write_batch(&batch).map_err(|e| e.to_string())?;
            rows += batch.num_rows() as u64;
            groups += 1;
        }
        (rows, groups)
    } else {
        let stats = pipeline::run(
            &schema,
            args.depth,
            |batch| src.next_batch(batch).map_err(|e| e.to_string()),
            |batch| writer.write_batch(batch).map_err(|e| e.to_string()),
        )?;
        (stats.rows, stats.batches)
    };
    writer.finish().map_err(|e| e.to_string())?;

    let elapsed = start.elapsed();
    let in_size = std::fs::metadata(&args.input).map(|m| m.len()).unwrap_or(0);
    let out_size = std::fs::metadata(&args.output)
        .map(|m| m.len())
        .unwrap_or(0);
    println!("rows: {rows}, row groups: {groups}");
    println!(
        "input: {in_size} B, output: {out_size} B, ratio: {:.2}",
        out_size as f64 / in_size.max(1) as f64
    );
    println!("time: {:.3?}", elapsed);
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
