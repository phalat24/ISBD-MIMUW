# Laboratorium 1 - pomiary latencji i przepustowości odczytów

Treść zadania: [PROJECT.md](PROJECT.md), wnioski z pomiarów: [analysis.md](analysis.md).

Wszystkie polecenia poniżej uruchamiane są z katalogu `src/`.

## Programy

Przykładowe ręczne uruchomienie:

```bash
cargo build --release --bins
./target/release/latency_fixed <plik> <n_threads> [--no-write-csv] [--cache-mode warm|cold]
./target/release/throughput_fixed <plik> [--no-write-csv] [--cache-mode warm|cold]
./target/release/throughput <plik> [--no-write-csv] [--cache-mode warm|cold]
./target/release/pipeline_fixed <plik> [k] [--no-write-csv] [--cache-mode warm|cold]
```


### Pomiary

Pomiary uruchamiane są przez `run_with_constrained_memory.sh <limit> <polecenie> [argumenty...]`. **Uwaga skrypt używa sudo** ponieważ używa systemd-run z limitem MemoryMax

### Pełne uruchomienie

```bash
./run_with_constrained_memory.sh 2G ./run_measurements.sh --cache-mode warm
./run_with_constrained_memory.sh 2G ./run_measurements.sh --cache-mode cold
```

### Wyszukiwanie K_opt (pipeline)

`find_k_opt.py` mierzy czas samego odczytu T(0), a potem szuka największego K, dla którego czas nie przekracza T(0) o więcej niż próg 5%
(korzysta z binsearcha, nie trwa długo).

```bash
./run_with_constrained_memory.sh 2G ./find_k_opt.py --file-size 1000000000 
./run_with_constrained_memory.sh 2G ./find_k_opt.py --file-size 4000000000
```

### Wykresy

```bash
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
.venv/bin/python plot_measurements.py
```

### Disclaimer o AI
Kod napisany w ruście jest 100% przeze mnie, oraz sporawozdanie pochodzi ode mnie, Klaudiusz zaimplementował wrapper w pythonie do generowania wykresu + wrapper w bashu do uruchamiania kodów w wyczyszczonym cachem i ograniczonym ramem. 