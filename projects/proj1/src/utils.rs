use std::fs::OpenOptions;
use std::io::Write;

pub fn parse_runtime_flags(args: &mut Vec<String>) -> (bool, String) {
    let mut should_write_csv = true;
    let mut cache_mode = "warm".to_string();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--write-csv" => {
                should_write_csv = true;
                args.remove(i);
            }
            "--no-write-csv" => {
                should_write_csv = false;
                args.remove(i);
            }
            "--cache-mode" => {
                if i + 1 >= args.len() {
                    panic!("Missing value after --cache-mode");
                }
                let chosen_mode = args[i + 1].trim().to_ascii_lowercase();
                if chosen_mode == "warm" || chosen_mode == "cold" {
                    cache_mode = chosen_mode;
                } else {
                    panic!("Unsupported cache mode: {chosen_mode}; expected warm or cold");
                }
                args.remove(i);
                args.remove(i);
            }
            "--warm-cache" => {
                cache_mode = "warm".to_string();
                args.remove(i);
            }
            "--cold-cache" => {
                cache_mode = "cold".to_string();
                args.remove(i);
            }
            _ => i += 1,
        }
    }

    (should_write_csv, cache_mode)
}

pub fn append_csv(task_name: &str, cache_mode: &str, header: &[&str], row: Vec<String>) {
    let mode = cache_mode.trim().to_ascii_lowercase();
    let file_path = if mode.is_empty() || mode == "default" {
        format!("{task_name}.csv")
    } else {
        format!("{task_name}_{mode}.csv")
    };

    let mut csv = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .unwrap_or_else(|err| panic!("Failed to open {file_path}: {err}"));

    if csv.metadata().unwrap().len() == 0 {
        writeln!(csv, "{}", header.join(",")).unwrap();
    }

    writeln!(csv, "{}", row.join(",")).unwrap();
}
