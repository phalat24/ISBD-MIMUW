use md5;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::thread;
use std::time::Instant;

mod hash;
mod utils;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 512;
const RAND_SEED: u64 = 1234;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let (should_write_csv, cache_mode) = utils::parse_runtime_flags(&mut args);

    if args.len() != 3 {
        eprintln!(
            "Usage: cargo run --release --bin latency_fixed <filepath> <n_threads> [--write-csv|--no-write-csv] [--cache-mode warm|cold]"
        );
        return;
    }

    let filepath = &args[1];
    let file = File::open(filepath).unwrap_or_else(|e| {
        panic!("Error opening file {}: {}", filepath, e);
    });

    let total_size = file.metadata().unwrap().len();
    if total_size == 0 {
        println!("{} 0x00000000000000000000000000000000", filepath);
        return;
    }

    let num_blocks = ((total_size + BLOCK_SIZE - 1) / BLOCK_SIZE) as usize;

    let mut shuffled_blocks: Vec<u64> = (0..num_blocks as u64).collect();
    let mut rng = StdRng::seed_from_u64(RAND_SEED);
    shuffled_blocks.shuffle(&mut rng);

    let num_threads: u64 = args[2].parse().unwrap();
    let chunk_size = (num_blocks + num_threads as usize - 1) / num_threads as usize;

    let start = Instant::now();
    let final_hash = thread::scope(|s| {
        let mut handles = Vec::new();

        for chunk in shuffled_blocks.chunks(chunk_size) {
            let file_ref = &file;

            let handle = s.spawn(move || {
                let mut local_result = [0u8; 16];
                let mut buffer = vec![0u8; BLOCK_SIZE as usize];

                for &block_idx in chunk {
                    let offset = block_idx * BLOCK_SIZE;
                    let bytes_to_read = std::cmp::min(BLOCK_SIZE, total_size - offset) as usize;
                    let target_buf = &mut buffer[..bytes_to_read];

                    file_ref
                        .read_exact_at(target_buf, offset)
                        .unwrap_or_else(|e| {
                            panic!("Error reading at offset {}: {}", offset, e);
                        });

                    let block_hash = md5::compute(target_buf);
                    sum_block_hash(&mut local_result, &block_hash.0);
                }

                local_result
            });

            handles.push(handle);
        }

        let mut combined_result = [0u8; 16];
        for handle in handles {
            let thread_local_hash = handle.join().unwrap();
            sum_block_hash(&mut combined_result, &thread_local_hash);
        }

        combined_result
    });
    let elapsed = start.elapsed();

    if should_write_csv {
        utils::append_csv(
            "latency",
            &cache_mode,
            &["file_size", "elapsed_ns", "n_threads"],
            vec![
                total_size.to_string(),
                elapsed.as_nanos().to_string(),
                num_threads.to_string(),
            ],
        );
    }

    print!("{} 0x", filepath);
    for byte in &final_hash {
        print!("{:02x}", byte);
    }
    println!();
}
