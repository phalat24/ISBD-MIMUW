#!/usr/bin/env python3
"""
Usage:
  ./run_with_constrained_memory.sh 2G ./find_k_opt.py --file-size 1000000000
  ./run_with_constrained_memory.sh 2G ./find_k_opt.py --file-size 4000000000
"""
import argparse
import csv
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent
BINARY = ROOT_DIR / "target" / "release" / "pipeline_fixed"
FIT_FRACTION = 0.9


def cgroup_memory_limit():
    try:
        cgroup_path = Path("/proc/self/cgroup").read_text().strip().split("::", 1)[1]
        value = Path(f"/sys/fs/cgroup{cgroup_path}/memory.max").read_text().strip()
        if value != "max":
            return int(value)
    except (OSError, IndexError, ValueError):
        pass
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("Cannot determine memory limit")


def generate_random_file(path: Path, size: int):
    chunk = 1024 * 1024
    with path.open("wb") as f:
        remaining = size
        while remaining > 0:
            n = min(chunk, remaining)
            f.write(os.urandom(n))
            remaining -= n
    os.sync()


def drop_file_cache(path: Path, drop_mode: str):
    if drop_mode == "global":
        os.sync()
        subprocess.run(["sysctl", "-w", "vm.drop_caches=3"], check=True,
                       stdout=subprocess.DEVNULL)
        return
    os.sync()
    fd = os.open(path, os.O_RDONLY)
    try:
        os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
    finally:
        os.close(fd)


def warm_file_cache(path: Path):
    with path.open("rb") as f:
        while f.read(16 * 1024 * 1024):
            pass


class Searcher:
    def __init__(self, path, cache_mode, drop_mode, repeats, log_writer):
        self.path = path
        self.cache_mode = cache_mode
        self.drop_mode = drop_mode
        self.repeats = repeats
        self.log_writer = log_writer
        self.file_size = path.stat().st_size
        self.memo = {}

    def run_once(self, k: int) -> int:
        drop_file_cache(self.path, self.drop_mode)
        if self.cache_mode == "warm":
            warm_file_cache(self.path)
        start = time.perf_counter_ns()
        subprocess.run([str(BINARY), str(self.path), str(k), "--no-write-csv"],
                       check=True, stdout=subprocess.DEVNULL)
        return time.perf_counter_ns() - start

    def measure(self, k: int) -> int:
        if k not in self.memo:
            samples = []
            for repeat in range(self.repeats):
                elapsed_ns = self.run_once(k)
                samples.append(elapsed_ns)
                self.log_writer.writerow([self.file_size, elapsed_ns, k, repeat])
            median = int(statistics.median(samples))
            self.memo[k] = median
            print(f"  K={k:>10}  median={median / 1e6:10.1f} ms  "
                  f"samples=[{', '.join(f'{s / 1e6:.1f}' for s in samples)}] ms",
                  flush=True)
        return self.memo[k]


def find_k_opt(searcher: Searcher, threshold: float, rel_precision: float):
    t0 = searcher.measure(0)
    limit = t0 * (1 + threshold)
    print(f"  baseline T(0)={t0 / 1e6:.1f} ms, limit={limit / 1e6:.1f} ms "
          f"(+{threshold:.0%})", flush=True)

    if searcher.measure(1) > limit:
        return None

    lo, hi = 1, 4
    while searcher.measure(hi) <= limit:
        lo, hi = hi, hi * 4

    while hi - lo > max(1, int(lo * rel_precision)):
        mid = (lo + hi) // 2
        if searcher.measure(mid) <= limit:
            lo = mid
        else:
            hi = mid
    return lo


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--file-size", type=int,
                        help="rozmiar generowanego losowego pliku testowego [B]")
    source.add_argument("--file", type=Path, help="uzyj istniejacego pliku")
    parser.add_argument("--threshold", type=float, default=0.05,
                        help="dopuszczalny wzrost czasu wzgledem T(0) (default: 0.05)")
    parser.add_argument("--repeats", type=int, default=3,
                        help="przebiegi na kazde K, brana mediana (default: 3)")
    parser.add_argument("--rel-precision", type=float, default=0.01,
                        help="koniec wyszukiwania binarnego, gdy (hi-lo) <= lo*precision "
                             "(default: 0.01)")
    parser.add_argument("--cache-mode", choices=["cold", "warm", "auto"], default="cold",
                        help="cold (default); auto: warm, jesli plik miesci sie w limicie pamieci")
    parser.add_argument("--drop-mode", choices=["fadvise", "global"], default="fadvise",
                        help="fadvise: POSIX_FADV_DONTNEED na pliku; global: vm.drop_caches=3")
    parser.add_argument("--out", type=Path, default=None,
                        help="CSV z kazdym przebiegiem, dopisywany "
                             "(default: k_opt_<cache-mode>.csv)")
    args = parser.parse_args()

    if not BINARY.exists():
        sys.exit(f"{BINARY} not found - run 'cargo build --release --bins' first")

    generated = args.file is None
    path = ROOT_DIR / ".k_opt_tmp.bin" if generated else args.file

    try:
        if generated:
            print(f"Generating {args.file_size} B test file...", flush=True)
            generate_random_file(path, args.file_size)
        file_size = path.stat().st_size

        memory_limit = cgroup_memory_limit()
        cache_mode = args.cache_mode
        if cache_mode == "auto":
            cache_mode = "warm" if file_size <= memory_limit * FIT_FRACTION else "cold"
        print(f"file_size={file_size} memory_limit={memory_limit} cache_mode={cache_mode} "
              f"threshold={args.threshold} repeats={args.repeats}", flush=True)

        out_path = args.out or ROOT_DIR / f"k_opt_{cache_mode}.csv"
        write_header = not out_path.exists() or out_path.stat().st_size == 0
        with out_path.open("a", newline="") as out:
            writer = csv.writer(out)
            if write_header:
                writer.writerow(["file_size", "elapsed_ns", "k", "repeat"])
            searcher = Searcher(path, cache_mode, args.drop_mode, args.repeats, writer)
            k_opt = find_k_opt(searcher, args.threshold, args.rel_precision)
    finally:
        if generated:
            path.unlink(missing_ok=True)

    if k_opt is None:
        print(f"RESULT file_size={file_size} cache_mode={cache_mode}: already K=1 exceeds "
              f"T(0) by more than {args.threshold:.0%} - compute-bound, no K_opt on this "
              f"hardware")
    else:
        print(f"RESULT file_size={file_size} cache_mode={cache_mode}: K_opt={k_opt}")


if __name__ == "__main__":
    main()
