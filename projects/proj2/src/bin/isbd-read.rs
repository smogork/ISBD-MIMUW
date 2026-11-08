use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use isbd::data::ColumnType;
use isbd::file::FileReader;
use isbd::metrics::{self, ColumnMetric};

#[derive(Parser)]
#[command(about = "Deserialize an ISBD file and compute column metrics")]
struct Args {
    /// Input ISBD file
    input: String,
    /// Also print the row group index from the footer
    #[arg(long)]
    verbose: bool,
}

fn run(args: &Args) -> Result<(), String> {
    let start = Instant::now();
    let reader = FileReader::open(&args.input).map_err(|e| e.to_string())?;
    let open_time = start.elapsed();

    let schema = reader.schema();
    println!("file: {}", args.input);
    println!(
        "columns: {}, row groups: {}, rows: {}",
        schema.len(),
        reader.num_row_groups(),
        reader.total_rows()
    );
    for (i, f) in schema.fields().iter().enumerate() {
        println!("  [{i}] {} {:?}", f.name, f.ty);
    }
    if args.verbose {
        for (g, rg) in reader.footer().row_groups.iter().enumerate() {
            println!("row group {g}: {} rows", rg.rows);
            for (c, ch) in rg.chunks.iter().enumerate() {
                println!(
                    "  col {c}: offset {} compressed {} B uncompressed {} B",
                    ch.offset, ch.compressed_len, ch.uncompressed_len
                );
            }
        }
    }

    let t = Instant::now();
    let (results, rows) = metrics::compute(&reader).map_err(|e| e.to_string())?;
    let compute_time = t.elapsed();

    println!("metrics ({rows} rows):");
    for (f, m) in schema.fields().iter().zip(&results) {
        match (f.ty, m) {
            (ColumnType::Int64, ColumnMetric::Int64Avg { sum, count }) => match m.average() {
                Some(avg) => println!("  {}: avg = {avg} (sum {sum}, count {count})", f.name),
                None => println!("  {}: avg = n/a (no rows)", f.name),
            },
            (ColumnType::Varchar, ColumnMetric::AsciiCount { count }) => {
                println!("  {}: ascii chars = {count}", f.name)
            }
            _ => unreachable!(),
        }
    }
    println!(
        "time: open {:.3?}, decode+metrics {:.3?}",
        open_time, compute_time
    );
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
