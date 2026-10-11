use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;

mod hash;
use hash::stretch_md5;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 1024 * 1024;
const K: u64 = 1000;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run --release --bin pipeline filepath");
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
    let num_blocks = (total_size + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let mut buffer = vec![0u8; BLOCK_SIZE as usize];
    let mut result = vec![0u8; 16];
    for block_idx in 0..num_blocks {
        let offset = block_idx * BLOCK_SIZE;
        let bytes_to_read = std::cmp::min(BLOCK_SIZE, total_size - offset) as usize;

        // read
        let target_buf = &mut buffer[..bytes_to_read];
        let err = file.read_exact_at(target_buf, offset);
        if err.is_err() {
            eprintln!("Error reading file: {}", err.unwrap_err());
            return;
        }

        // compute
        let block_hash = stretch_md5(&target_buf, K);
        sum_block_hash(&mut result, &block_hash);
    }
    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");
}
