use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::time::Instant;
use memmap::MmapOptions;

mod hash;
use hash::{env_u64, sum_block_hash};

const BLOCK_SIZE: u64 = 512;

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: [M=512] [MODE=mmap|read] cargo run --release --bin throughput filepath");
        return;
    }

    let file_res = File::open(args[1].clone());
    if file_res.is_err() {
        eprintln!("File not found: {}", args[1]);
        return;
    }
    let file = file_res.unwrap();

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }
    let total_size = metadata.unwrap().len();
    let block_size = env_u64("M", BLOCK_SIZE);
    let use_read = env::var("MODE").is_ok_and(|m| m == "read");
    let num_blocks = (total_size + block_size - 1) / block_size;

    let mut buffer = vec![0u8; block_size as usize];
    let mut result = vec![0u8; 16];
    let start = Instant::now();
    if use_read {
        for block_idx in 0..num_blocks
        {
            let offset = block_idx * block_size;
            let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;

            // read
            let target_buf = &mut buffer[..bytes_to_read];
            let err = file.read_exact_at(target_buf, offset);
            if err.is_err() {
                eprintln!("Error reading file: {}", err.unwrap_err());
                return;
            }

            // compute
            let block_hash = md5::compute(target_buf);
            sum_block_hash(&mut result, &block_hash);
        }
    } else { 
        let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to mmap file") };
        for block in mmap.chunks(block_size as usize) {
            let block_hash = md5::compute(block);
            sum_block_hash(&mut result, &block_hash);
        }
    }
    let total = start.elapsed().as_secs_f64();
    eprintln!(
        "prog=throughput mode={} M={} size={} total_s={:.6} mib_s={:.1}",
        if use_read { "read" } else { "mmap" },
        block_size, total_size, total, total_size as f64 / 1048576.0 / total
    );
    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}
