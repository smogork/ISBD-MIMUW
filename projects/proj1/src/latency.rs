use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Instant;
use rand::{rng, Rng, RngExt, SeedableRng};
use rand::rngs::StdRng;

mod hash;
use hash::{env_u64, sum_block_hash};

pub struct RandomOrderIterator {
    elements: Vec<u64>,
    rng: Box<dyn Rng>
}

impl RandomOrderIterator {
    pub fn with_seed(size: u64, seed: u64) -> Self {
        Self { elements: (0 as u64..size).collect() , rng: Box::new(StdRng::seed_from_u64(seed)), }
    }

    pub fn new(size: u64) -> Self {
        Self {
            elements: (0..size).collect(),
            rng: Box::new(rng()),
        }
    }
}

impl Iterator for RandomOrderIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements.is_empty() {
            None
        } else {
            let rand_idx = self.rng.random_range(0..self.elements.len());
            Some(self.elements.swap_remove(rand_idx))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.elements.len(), Some(self.elements.len()))
    }
}

const BLOCK_SIZE: u64 = 512;
const RAND_SEED: u64 = 1234;
const N_THREADS: u64 = 4;

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: [M=512] [THREADS=4 (0 = sync baseline)] cargo run --release --bin latency filepath");
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
    let n_threads = env_u64("THREADS", N_THREADS);
    let num_blocks = (total_size + block_size - 1) / block_size;
    let random_iter = RandomOrderIterator::with_seed(num_blocks, RAND_SEED);

    let mut buffer = vec![0u8; block_size as usize];
    let mut result = vec![0u8; 16];
    // per-read latency in ns
    let (mut read_sum, mut read_min, mut read_max) = (0u64, u64::MAX, 0u64);
    let start = Instant::now();
    if n_threads == 0 {
        for block_idx in random_iter
        {
            let offset = block_idx * block_size;
            let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;

            // read
            let target_buf = &mut buffer[..bytes_to_read];
            let t = Instant::now();
            let err = file.read_exact_at(target_buf, offset);
            let ns = t.elapsed().as_nanos() as u64;
            read_sum += ns;
            read_min = read_min.min(ns);
            read_max = read_max.max(ns);
            if err.is_err() {
                eprintln!("Error reading file: {}", err.unwrap_err());
                return;
            }

            // compute
            let block_hash = md5::compute(target_buf);
            sum_block_hash(&mut result, &block_hash);
        }
    } else {
        let (sender, receiver) = sync_channel(1);
        let f = &file;
        let idxs: Vec<_> = random_iter.collect();
        thread::scope(|s| {
            let mut handles = Vec::new();
            for idx_range in idxs.chunks(idxs.len().div_ceil(n_threads as usize).max(1)) {
                let se = sender.clone();
                handles.push(s.spawn(move || {
                    let (mut read_sum, mut read_min, mut read_max) = (0u64, u64::MAX, 0u64);
                    for block_idx in idx_range {
                        let offset = block_idx * block_size;
                        let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;
                        let mut buf = vec![0u8; block_size as usize];
                        let target_buf = &mut buf[..bytes_to_read];
                        let t = Instant::now();
                        f.read_exact_at(target_buf, offset)
                            .expect("Error reading file");
                        let ns = t.elapsed().as_nanos() as u64;
                        read_sum += ns;
                        read_min = read_min.min(ns);
                        read_max = read_max.max(ns);
                        se.send((buf, bytes_to_read))
                            .expect("Failed to send through the channel");
                    }
                    (read_sum, read_min, read_max)
                }));
            }
            drop(sender);
            for (buf, len) in receiver {
                let block_hash = md5::compute(&buf[..len]);
                sum_block_hash(&mut result, &block_hash);
            }
            for h in handles {
                let (sum, min, max) = h.join().expect("Join failed");
                read_sum += sum;
                read_min = read_min.min(min);
                read_max = read_max.max(max);
            }
        });
    }
    let total = start.elapsed().as_secs_f64();
    eprintln!(
        "prog=latency M={} THREADS={} blocks={} total_s={:.6} read_avg_ns={} read_min_ns={} read_max_ns={}",
        block_size, n_threads, num_blocks, total,
        read_sum / num_blocks.max(1),
        if num_blocks == 0 { 0 } else { read_min },
        read_max
    );
    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}
