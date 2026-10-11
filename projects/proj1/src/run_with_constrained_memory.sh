#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

usage() {
    cat <<'EOF'
Usage: run_with_constrained_memory.sh <memory-limit> <command> [args...]
  memory-limit  wartosc MemoryMax dla systemd-run, np. 512M, 2G
  command       program uruchamiany w ograniczonej cgroupie; wszystkie
                kolejne argumenty sa przekazywane do niego bez zmian

Przyklad:
  ./run_with_constrained_memory.sh 2G ./run_measurements.sh --cache-mode cold
EOF
}

if [[ $# -lt 2 ]]; then
    usage >&2
    exit 1
fi

MEMORY_LIMIT="$1"
shift

cargo build --release --bins

# MemoryMax ogranicza takze page cache cgroupy, wiec jest to jednoczesnie
# ustawienie "ile pliku moze byc cieple". Przy 2G pliki ponizej ~2GB mieszcza
# sie w cache w calosci, a wieksze - nie; tam tryb warm z zalozenia degraduje
# sie do cold i to wlasnie jest szukana presja pamieciowa.
echo "Running '$*' with MemoryMax=$MEMORY_LIMIT"
exec sudo systemd-run --scope --setenv=PATH="$PATH" -p MemoryMax="$MEMORY_LIMIT" -- "$@"
