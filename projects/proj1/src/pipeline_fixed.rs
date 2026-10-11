use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;
use std::time::Instant;

mod hash;
mod utils;
use hash::{stretch_md5, sum_block_hash};

const BLOCK_SIZE: usize = 1024 * 1024;
const K: u64 = 1000;

fn reader(file: File, sync_channel: SyncSender<Vec<u8>>, total_size: usize) {
    let mut offset: usize = 0;
    while offset < total_size {
        let bytes_to_read = std::cmp::min(BLOCK_SIZE, total_size - offset) as usize;

        let mut target_buf = vec![0u8; bytes_to_read];

        if let Err(err) = file.read_exact_at(&mut target_buf, offset as u64) {
            eprintln!("Error reading file at offset {}: {}", offset, err);
            return;
        }

        if sync_channel.send(target_buf).is_err() {
            break;
        }
        offset += bytes_to_read;
    }
}

fn compute(rx: Receiver<Vec<u8>>, k: u64) -> [u8; 16] {
    let mut result = [0u8; 16];

    if k == 0 {
        while rx.recv().is_ok() {}
        return result;
    }

    while let Ok(target_buf) = rx.recv() {
        let block_hash = stretch_md5(&target_buf, k);
        sum_block_hash(&mut result, &block_hash.0);
    }

    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    let (should_write_csv, cache_mode) = utils::parse_runtime_flags(&mut args);

    if args.len() < 2 || args.len() > 3 {
        eprintln!(
            "Usage: cargo run --release --bin pipeline <filepath> [k] [--write-csv|--no-write-csv] [--cache-mode warm|cold]"
        );
        return Ok(());
    }

    let filepath = &args[1];
    let k: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(K);

    let file = File::open(filepath)?;
    let total_size = file.metadata()?.len() as usize;

    if total_size == 0 {
        println!("{} 0x00000000000000000000000000000000", filepath);
        return Ok(());
    }

    let (tx, rx) = sync_channel::<Vec<u8>>(2);

    let start = Instant::now();
    let final_hash = thread::scope(|s| {
        s.spawn(move || {
            reader(file, tx, total_size);
        });

        let compute_handle = s.spawn(move || compute(rx, k));

        compute_handle.join().unwrap()
    });
    let elapsed = start.elapsed();

    if should_write_csv {
        utils::append_csv(
            "pipeline",
            &cache_mode,
            &["file_size", "elapsed_ns", "k"],
            vec![
                total_size.to_string(),
                elapsed.as_nanos().to_string(),
                k.to_string(),
            ],
        );
    }

    print!("{} 0x", filepath);
    for byte in &final_hash {
        print!("{:02x}", byte);
    }
    println!();

    Ok(())
}
