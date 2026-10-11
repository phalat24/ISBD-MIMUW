#!/usr/bin/env python3
import csv
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt


def load_csv(path: Path):
    rows = []
    with path.open("r", newline="", encoding="utf-8") as f:
        reader = csv.reader(f)
        header = next(reader, None)
        if header is None:
            return [], None
        for row in reader:
            if not row or not any(cell.strip() for cell in row):
                continue
            rows.append(row)
    return rows, header


def ns_to_ms(values):
    return [v / 1_000_000 for v in values]


def best_per_size(samples):
    best = {}
    for size, elapsed_ns in samples:
        if size not in best or elapsed_ns < best[size]:
            best[size] = elapsed_ns
    return sorted(best.items())


ZOOM_MAX_SIZE = 1_000_000_000


def load_series(csv_path: Path):
    rows, header = load_csv(csv_path)
    if not rows or not header:
        return None, {}

    param_col_name = header[2] if len(header) >= 3 else None
    data = {}
    for row in rows:
        try:
            size = int(row[0])
            elapsed_ns = int(row[1])
            key = int(row[2]) if param_col_name else None
        except (ValueError, IndexError):
            continue
        data.setdefault(key, []).append((size, elapsed_ns))

    series = {}
    for key in sorted(data, key=lambda k: (k is None, k)):
        label = f"{param_col_name}={key}" if param_col_name else None
        series[label] = best_per_size(data[key])
    return param_col_name, series


def render_plot(out_base: Path, title_name, series, has_legend, zoom: bool, styles=None):
    fig, ax = plt.subplots(figsize=(10, 6))

    for label, points in series.items():
        if zoom:
            points = [p for p in points if p[0] <= ZOOM_MAX_SIZE]
        sizes = [p[0] for p in points]
        times_ms = ns_to_ms([p[1] for p in points])
        style = (styles or {}).get(label, {})
        ax.plot(sizes, times_ms, marker="o", label=label, **style)

    if zoom:
        ax.set_title(f"{title_name}: Time vs File Size up to 1GB (min of repeats)")
        ax.grid(True, alpha=0.3)
        suffix = "zoom"
    else:
        ax.set_title(f"{title_name}: Time vs File Size, log scale (min of repeats)")
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.grid(True, which="both", alpha=0.3)
        suffix = "log"
    ax.set_xlabel("File size [B]")
    ax.set_ylabel("Time [ms]")

    if has_legend:
        ax.legend()

    fig.tight_layout()
    out_path = out_base.with_name(f"{out_base.name}_plot_{suffix}.png")
    fig.savefig(out_path, dpi=200)
    plt.close(fig)
    print(f"Plot saved: {out_path}")


def plot_single_csv(csv_path: Path):
    param_col_name, series = load_series(csv_path)
    if not series:
        print(f"No valid data in {csv_path}")
        return

    title_name = csv_path.stem.replace("_", " ").title()
    for zoom in (True, False):
        render_plot(csv_path.with_suffix(""), title_name, series,
                    param_col_name is not None, zoom)


THROUGHPUT_COMPARISON = {
    "mmap warm": ("throughput_warm.csv", {"color": "tab:blue", "linestyle": "-"}),
    "mmap cold": ("throughput_cold.csv", {"color": "tab:blue", "linestyle": "--"}),
    "read warm": ("throughput_read_warm.csv", {"color": "tab:orange", "linestyle": "-"}),
    "read cold": ("throughput_read_cold.csv", {"color": "tab:orange", "linestyle": "--"}),
}


def plot_throughput_comparison(base: Path):
    series, styles = {}, {}
    for label, (file_name, style) in THROUGHPUT_COMPARISON.items():
        csv_path = base / file_name
        if not csv_path.exists():
            continue
        _, file_series = load_series(csv_path)
        if file_series:
            series[label] = file_series[None]
            styles[label] = style

    if len(series) < 2:
        print("Not enough throughput CSVs for the read vs mmap comparison.")
        return

    for zoom in (True, False):
        render_plot(base / "throughput_comparison", "Throughput: read vs mmap", series,
                    True, zoom, styles)


def main():
    base = Path(__file__).resolve().parent
    csv_files = sorted(p for p in base.glob("*.csv") if not p.name.startswith("k_opt_"))

    if not csv_files:
        print("No CSV files found.")
        return

    for csv_path in csv_files:
        plot_single_csv(csv_path)

    plot_throughput_comparison(base)


if __name__ == "__main__":
    main()