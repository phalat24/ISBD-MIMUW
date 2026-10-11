#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

MAX_SIZE="${MAX_SIZE:-10000000000}"
CACHE_MODE="${CACHE_MODE:-warm}"
# PROJECT.md: "warto skupic sie na minimum z wielu pomiarow" - kazda konfiguracja
# jest mierzona REPEATS razy, a plot_measurements.py bierze minimum.
REPEATS="${REPEATS:-3}"
# Powyzej LARGE_SIZE pojedynczy przebieg trwa minuty (10GB / 1 watek / cold to
# ~3.5 min samego latency_fixed), wiec powtorzen jest mniej. Szum systemu i tak
# jest wzgledenie mniejszy przy dlugich pomiarach.
LARGE_REPEATS="${LARGE_REPEATS:-1}"
LARGE_SIZE="${LARGE_SIZE:-1500000000}"
# fadvise - POSIX_FADV_DONTNEED tylko na pliku testowym (bez roota, nie rusza
#           reszty systemu). global - sudo sysctl vm.drop_caches=3.
DROP_MODE="${DROP_MODE:-fadvise}"
# Nazwy = prefiksy plikow CSV: latency, throughput (mmap), throughput_read
# (read), pipeline. Kasowane sa tylko CSV wybranych benchmarkow.
BENCHMARKS="${BENCHMARKS:-latency,throughput,throughput_read,pipeline}"

usage() {
    cat <<'EOF'
Usage: run_measurements.sh [options]
  --cache-mode warm|cold   czy plik ma byc w page cache przed pomiarem (default: warm)
  --max-size BYTES         pomijaj rozmiary wieksze niz podany (default: 10000000000)
  --repeats N              ile powtorzen kazdej konfiguracji (default: 3)
  --large-repeats N        powtorzenia dla plikow > --large-size (default: 1)
  --large-size BYTES       prog "duzego pliku" (default: 1500000000)
  --drop-mode fadvise|global
                           fadvise: posix_fadvise(DONTNEED) na pliku testowym, bez roota
                           global:  sudo sysctl -w vm.drop_caches=3 (czysci cache calego systemu)
  --benchmarks LIST        lista po przecinku z: latency,throughput,throughput_read,pipeline
                           (default: wszystkie)
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cache-mode)
            shift
            CACHE_MODE="${1:-warm}"
            if [[ "$CACHE_MODE" != "warm" && "$CACHE_MODE" != "cold" ]]; then
                echo "Unsupported cache mode: $CACHE_MODE" >&2
                exit 1
            fi
            ;;
        --max-size)
            shift
            MAX_SIZE="${1:-}"
            ;;
        --repeats)
            shift
            REPEATS="${1:-3}"
            ;;
        --large-repeats)
            shift
            LARGE_REPEATS="${1:-1}"
            ;;
        --large-size)
            shift
            LARGE_SIZE="${1:-1500000000}"
            ;;
        --benchmarks)
            shift
            BENCHMARKS="${1:-}"
            ;;
        --drop-mode)
            shift
            DROP_MODE="${1:-fadvise}"
            if [[ "$DROP_MODE" != "fadvise" && "$DROP_MODE" != "global" ]]; then
                echo "Unsupported drop mode: $DROP_MODE" >&2
                exit 1
            fi
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 1
            ;;
    esac
    shift
done

n_threads=(1 2 4 8 16)
K=(1 10 50 100 500 1000 5000 20000 100000)
# Posortowane rosnaco - petla nizej uzywa "continue", wiec zly porzadek nie urwie
# juz reszty pomiarow, ale posortowana lista i tak daje czytelniejszy przebieg.
SIZES=(
    512 1024 2048 4096 8192 16384 32768 65536 131072 262144 524288
    1048576 2097152 4194304 8388608 16777216 33554432 67108864
    400000000 500000000 600000000 700000000 800000000 900000000
    1000000000 1100000000 1200000000
    # Powyzej MemoryMax=2G page cache nie pomiesci juz calego pliku - od tego
    # miejsca tryb "warm" degraduje sie do cold i widac presje pamieciowa (LRU).
    2000000000 3000000000 4000000000 6000000000 8000000000 10000000000
)

tmp_file="$ROOT_DIR/.measurement_tmp.bin"
trap 'rm -f "$tmp_file"' EXIT

drop_file_cache() {
    local path="$1"

    if [[ "$DROP_MODE" == "global" ]]; then
        sudo sync
        sudo sysctl -w vm.drop_caches=3 >/dev/null
        return
    fi

    sync
    python3 - "$path" <<'PY'
import os, sys
fd = os.open(sys.argv[1], os.O_RDONLY)
try:
    os.posix_fadvise(fd, 0, os.fstat(fd).st_size, os.POSIX_FADV_DONTNEED)
finally:
    os.close(fd)
PY
}

# Zwraca liczbe stron pliku rezydujacych w page cache - sanity check, ze tryb
# cold faktycznie jest cold, a warm faktycznie warm.
cached_pages() {
    python3 - "$1" <<'PY'
import ctypes, mmap, os, sys
path = sys.argv[1]
fd = os.open(path, os.O_RDWR)
size = os.fstat(fd).st_size
if size == 0:
    print("0 0")
    sys.exit()
m = mmap.mmap(fd, size, prot=mmap.PROT_READ | mmap.PROT_WRITE)
npages = (size + 4095) // 4096
vec = (ctypes.c_ubyte * npages)()
addr = ctypes.addressof(ctypes.c_char.from_buffer(m))
ctypes.CDLL("libc.so.6").mincore(ctypes.c_void_p(addr), ctypes.c_size_t(size), vec)
print(f"{sum(v & 1 for v in vec)} {npages}")
del vec
m.close()
os.close(fd)
PY
}

generate_random_file() {
    local size="$1"
    local path="$2"

    rm -f "$path"

    local avail
    avail=$(df --output=avail -B1 "$ROOT_DIR" | tail -1)
    if (( avail < size + 104857600 )); then
        echo "Not enough free space for a ${size}B file (${avail}B available, need ~$((size + 104857600))B)" >&2
        return 1
    fi

    if (( size > 0 )); then
        local block_count=$(( (size + 1048575) / 1048576 ))
        dd if=/dev/urandom of="$path" bs=1048576 count="$block_count" status=none
    else
        : > "$path"
    fi

    truncate -s "$size" "$path"
}

# Pojedynczy pomiar na JUZ wygenerowanym pliku. Plik nie jest odtwarzany miedzy
# powtorzeniami: cache i tak jest zrzucany, a staly uklad ekstentow na dysku
# czyni kolejne pomiary porownywalnymi (i oszczedza minuty na dd z /dev/urandom).
run_single_measurement() {
    drop_file_cache "$tmp_file"

    if [[ "$CACHE_MODE" == "warm" ]]; then
        # UWAGA: throughput_fixed przyjmuje DOKLADNIE jeden argument pozycyjny.
        # Poprzednia wersja wolala go z dodatkowym "4", przez co program
        # wypisywal usage i konczyl sie bez czytania pliku - tryb warm byl
        # w rzeczywistosci drugim przebiegiem cold.
        cat "$tmp_file" > /dev/null
    fi

    "$@" >/dev/null
}

verify_cache_state() {
    local size="$1"
    read -r resident total <<<"$(cached_pages "$tmp_file")"
    (( total == 0 )) && return 0

    local pct=$(( 100 * resident / total ))
    if [[ "$CACHE_MODE" == "cold" && $pct -gt 5 ]]; then
        echo "  WARNING: size=$size cold mode but ${pct}% of the file is still cached" >&2
    elif [[ "$CACHE_MODE" == "warm" && $pct -lt 95 ]]; then
        echo "  WARNING: size=$size warm mode but only ${pct}% of the file is cached" \
             "(memory limit too low for this size?)" >&2
    fi
}

selected() {
    [[ ",$BENCHMARKS," == *",$1,"* ]]
}

for bench in ${BENCHMARKS//,/ }; do
    case "$bench" in
        latency|throughput|throughput_read|pipeline)
            rm -f "./${bench}_${CACHE_MODE}.csv"
            ;;
        *)
            echo "Unknown benchmark: $bench" >&2
            exit 1
            ;;
    esac
done

echo "benchmarks=$BENCHMARKS cache-mode=$CACHE_MODE drop-mode=$DROP_MODE repeats=$REPEATS max-size=$MAX_SIZE"

for size in "${SIZES[@]}"; do
    (( size > MAX_SIZE )) && continue

    size_repeats="$REPEATS"
    (( size > LARGE_SIZE )) && size_repeats="$LARGE_REPEATS"

    size_started=$SECONDS
    echo "size=$size repeats=$size_repeats"
    generate_random_file "$size" "$tmp_file" || continue

    # Jeden przebieg kontrolny, zeby zglosic rozjazd miedzy zadanym
    # a faktycznym stanem cache (np. warm pod zbyt malym MemoryMax).
    drop_file_cache "$tmp_file"
    [[ "$CACHE_MODE" == "warm" ]] && cat "$tmp_file" > /dev/null
    verify_cache_state "$size"

    for (( rep = 0; rep < size_repeats; rep++ )); do
        if selected latency; then
            for threads in "${n_threads[@]}"; do
                run_single_measurement ./target/release/latency_fixed \
                    "$tmp_file" "$threads" --write-csv --cache-mode "$CACHE_MODE"
            done
        fi

        if selected throughput; then
            run_single_measurement ./target/release/throughput_fixed \
                "$tmp_file" --write-csv --cache-mode "$CACHE_MODE"
        fi

        if selected throughput_read; then
            run_single_measurement ./target/release/throughput \
                "$tmp_file" --write-csv --cache-mode "$CACHE_MODE"
        fi

        if selected pipeline; then
            for k in "${K[@]}"; do
                run_single_measurement ./target/release/pipeline_fixed \
                    "$tmp_file" "$k" --write-csv --cache-mode "$CACHE_MODE"
            done
        fi
    done

    rm -f "$tmp_file"
    echo "  done in $(( SECONDS - size_started ))s"
done

echo "Results in:"
ls -lh ./*_"${CACHE_MODE}".csv 2>/dev/null || true
