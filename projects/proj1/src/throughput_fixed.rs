use memmap2::Mmap;
use std::env;
use std::fs::File;
use std::time::Instant;

mod hash;
mod utils;
use hash::sum_block_hash;

const BLOCK_SIZE: usize = 512;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let (should_write_csv, cache_mode) = utils::parse_runtime_flags(&mut args);

    if args.len() != 2 {
        println!(
            "Usage: cargo run --release --bin throughput_fixed filepath [--write-csv|--no-write-csv] [--cache-mode warm|cold]"
        );
        return;
    }

    let file_res = File::open(args[1].clone());
    if file_res.is_err() {
        eprintln!("File not found: {}", args[1]);
        return;
    }
    let file = file_res.unwrap();
    let mmap = unsafe { Mmap::map(&file) }.unwrap();

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }
    let total_size = metadata.unwrap().len() as usize;

    let mut result = [0u8; 16];
    let start = Instant::now();
    let mut offset: usize = 0;
    while offset < total_size {
        let block_hash =
            md5::compute(&mmap[offset..std::cmp::min(offset + BLOCK_SIZE, total_size)]);

        sum_block_hash(&mut result, &block_hash);
        offset += BLOCK_SIZE;
    }
    let elapsed = start.elapsed();

    if should_write_csv {
        utils::append_csv(
            "throughput",
            &cache_mode,
            &["file_size", "elapsed_ns"],
            vec![total_size.to_string(), elapsed.as_nanos().to_string()],
        );
    }

    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");
}
