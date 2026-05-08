#!/usr/bin/env python3
"""
Run the syscall criterion benches, save raw + parsed output keyed by the
arkworks version detected in Cargo.lock, and rewrite the four data tables
in syscalls/benches/README.md from the saved JSON files.

Usage:
    syscalls/benches/update_bench_results.py
        # detect ark version, run `cargo bench -p solana-syscalls`,
        # save to syscalls/benches/results/ark-{ver}.{txt,json},
        # regenerate README tables

    syscalls/benches/update_bench_results.py --no-run
        # skip running benches; regenerate README from existing JSON files

    syscalls/benches/update_bench_results.py --from-file results/ark-0.5.txt --ark 0.5
        # parse a specific text file, save JSON, regenerate README

    syscalls/benches/update_bench_results.py --filter "BN254 prepared pairing"
        # run only benches whose name matches the criterion filter; merge into
        # existing ark-{ver}.json (other categories preserved); regenerate README

The README tables sit between HTML comment markers; the script replaces
between them. Per-ark JSON files stay on disk so historical ark 0.4
columns survive even when we only re-bench against the current ark.
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CARGO_LOCK = ROOT / "Cargo.lock"
RESULTS_DIR = HERE / "results"
README = HERE / "README.md"

NS_PER_CU = 33.0

# Historical reference data (units: ns).
ZEN3_2022_ALT_BN128 = {  # c6a.2xlarge, alt_bn128 PR (solana-labs/solana#27961)
    "G1 add": 4_155.0,
    "G1 mul": 126_680.0,
    "Pairing n=2": 1_372_000.0,
    "Pairing n=3": 1_682_000.0,
    "Pairing n=4": 1_998_000.0,
    "Pairing n=5": 2_323_000.0,
}
ZEN3_2023_COMPRESSION = {  # c6a.2xlarge, compression PR (#32870)
    "g1_compress": 1_004.9,
    "g1_decompress": 13_154.0,
    "g2_compress": 2_854.3,
    "g2_decompress": 449_130.0,
}
ZEN4_LIGHT_POSEIDON = {  # Ryzen 9 7945HX, from light-poseidon README
    1: 12_735.0,
    2: 18_963.0,
    4: 38_513.0,
    8: 105_490.0,
    12: 210_810.0,
}

# Mainnet CU formulas / values.
MAINNET_ALT_BN128 = {
    "G1 add": 334,
    "G1 mul": 3_840,
    "G2 add": 535,
    "G2 mul": 15_670,
}
MAINNET_COMPRESSION = {
    "g1_compress": 30,
    "g1_decompress": 398,
    "g2_compress": 86,
    "g2_decompress": 13_610,
}


def mainnet_pairing(n):
    return 36_364 + (n - 1) * 12_121


def mainnet_poseidon(n):
    return 61 * n * n + 542


# Bench id -> (op_key, endianness)
ALT_BN128_BENCHES = {
    "BN254 G1 random/Addition/BE": ("G1 add", "BE"),
    "BN254 G1 random/Addition/LE": ("G1 add", "LE"),
    "BN254 G1 random/Multiplication/BE": ("G1 mul", "BE"),
    "BN254 G1 random/Multiplication/LE": ("G1 mul", "LE"),
    "BN254 G2 random/Addition/BE": ("G2 add", "BE"),
    "BN254 G2 random/Addition/LE": ("G2 add", "LE"),
    "BN254 G2 random/Multiplication/BE": ("G2 mul", "BE"),
    "BN254 G2 random/Multiplication/LE": ("G2 mul", "LE"),
    **{
        f"BN254 Pairing random/{e}/{n}": (f"Pairing n={n}", e)
        for n in (2, 3, 4, 8, 16)
        for e in ("BE", "LE")
    },
}

PREPARED_PAIRING_NS = (2, 3, 4, 8, 16)
PREPARED_PAIRING_BENCHES = {
    f"BN254 prepared pairing/LE/{n}": (f"prepared n={n}", "LE")
    for n in PREPARED_PAIRING_NS
}

POSEIDON_BENCHES = {
    f"Poseidon Bn254X5/{e}/{n}": (f"poseidon n={n}", e)
    for n in (1, 2, 4, 8, 12)
    for e in ("BE", "LE")
}

COMPRESSION_BENCHES = {
    f"BN254 G{g} compression/{op}/{e}": (f"g{g}_{op.lower()}", e)
    for g in (1, 2)
    for op in ("Compress", "Decompress")
    for e in ("BE", "LE")
}

UNIT_NS = {"ns": 1.0, "µs": 1_000.0, "us": 1_000.0, "ms": 1_000_000.0, "s": 1e9}
TIME_RE = re.compile(
    r"time:\s*\[\s*"
    r"(?P<lo>[\d.]+)\s+(?P<lo_u>ns|µs|us|ms|s)\s+"
    r"(?P<mean>[\d.]+)\s+(?P<mean_u>ns|µs|us|ms|s)\s+"
    r"(?P<hi>[\d.]+)\s+(?P<hi_u>ns|µs|us|ms|s)\s*\]"
)


def detect_ark_version():
    """Return e.g. '0.5' for the ark-bn254 dep of solana-bn254 in Cargo.lock."""
    text = CARGO_LOCK.read_text()
    m = re.search(
        r'\[\[package\]\]\nname = "solana-bn254"\n.*?\ndependencies = \[\n(.*?)\n\]',
        text,
        re.DOTALL,
    )
    if not m:
        sys.exit("could not locate solana-bn254 in Cargo.lock")
    am = re.search(r'"ark-bn254 (\d+\.\d+)\.\d+"', m.group(1))
    if not am:
        sys.exit("could not locate ark-bn254 dep of solana-bn254 in Cargo.lock")
    return am.group(1)


def run_benches(filter_pattern=None):
    cmd = ["cargo", "bench", "-p", "solana-syscalls"]
    if filter_pattern:
        cmd += ["--", filter_pattern]
    proc = subprocess.run(
        cmd,
        cwd=ROOT,
        text=True,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return proc.stdout


def parse_criterion(text):
    """{'BN254 G1 random/Addition/BE': 3248.2, ...} — upper bounds in ns."""
    results = {}
    lines = text.splitlines()
    for i, line in enumerate(lines):
        m = TIME_RE.search(line)
        if not m:
            continue
        prefix = line[: m.start()].strip()
        bench_id = prefix or (lines[i - 1].strip() if i > 0 else "")
        bench_id = re.sub(r"^Benchmarking\s+", "", bench_id).rstrip(":")
        results[bench_id] = float(m.group("hi")) * UNIT_NS[m.group("hi_u")]
    return results


def to_structured(parsed):
    """Group parsed criterion results into {category: {(op, endianness): ns}}."""
    out = {"alt_bn128": {}, "poseidon": {}, "compression": {}, "prepared_pairing": {}}
    for bench_id, ns in parsed.items():
        if bench_id in ALT_BN128_BENCHES:
            out["alt_bn128"][ALT_BN128_BENCHES[bench_id]] = ns
        elif bench_id in PREPARED_PAIRING_BENCHES:
            out["prepared_pairing"][PREPARED_PAIRING_BENCHES[bench_id]] = ns
        elif bench_id in POSEIDON_BENCHES:
            out["poseidon"][POSEIDON_BENCHES[bench_id]] = ns
        elif bench_id in COMPRESSION_BENCHES:
            out["compression"][COMPRESSION_BENCHES[bench_id]] = ns
    return out


def fmt_time(ns):
    if ns is None:
        return "—"
    if ns < 1_000:
        return f"{ns:.5g} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.5g} µs"
    return f"{ns / 1_000_000:.5g} ms"


def fmt_int(x):
    return "—" if x is None else f"{int(round(x)):,}"


def cu(time_ns):
    return None if time_ns is None else int(round(time_ns / NS_PER_CU))


def load_results():
    """Load every results/ark-X.json into {ark_version: structured_data}."""
    if not RESULTS_DIR.exists():
        return {}
    out = {}
    for path in sorted(RESULTS_DIR.glob("ark-*.json")):
        ark = path.stem.split("-", 1)[1]
        data = json.loads(path.read_text())
        # JSON keys are strings; rehydrate (op, endian) tuples.
        out[ark] = {
            cat: {tuple(k.split("|")): v for k, v in d.items()}
            for cat, d in data.items()
        }
    return out


def save_results(ark, structured, merge=False):
    RESULTS_DIR.mkdir(exist_ok=True)
    path = RESULTS_DIR / f"ark-{ark}.json"
    if merge and path.exists():
        existing = json.loads(path.read_text())
        existing_struct = {
            cat: {tuple(k.split("|")): v for k, v in d.items()}
            for cat, d in existing.items()
        }
        for cat, d in structured.items():
            existing_struct.setdefault(cat, {}).update(d)
        structured = existing_struct
    serializable = {
        cat: {f"{op}|{end}": v for (op, end), v in d.items()}
        for cat, d in structured.items()
    }
    path.write_text(json.dumps(serializable, indent=2) + "\n")


def arks_with_data(per_ark, category):
    return sorted(a for a, d in per_ark.items() if d.get(category))


def render_alt_bn128(per_ark):
    arks = arks_with_data(per_ark, "alt_bn128")
    cu_ark = arks[-1] if arks else "?"
    headers = (
        ["op", "2022 c6a.2xlarge (Zen 3)"]
        + [f"M5 Pro (ark {a})" for a in arks]
        + [f"CU @ 33 ns (ark {cu_ark})", "mainnet CU"]
    )
    rows = []
    base_ops = ["G1 add", "G1 mul", "G2 add", "G2 mul"]
    pair_ns = [2, 3, 4, 5, 8, 16]
    for op in base_ops:
        zen3 = ZEN3_2022_ALT_BN128.get(op)
        ark_times = [per_ark.get(a, {}).get("alt_bn128", {}).get((op, "BE")) for a in arks]
        cu_t = ark_times[-1] if ark_times else None
        rows.append(
            [op, fmt_time(zen3)]
            + [fmt_time(t) for t in ark_times]
            + [fmt_int(cu(cu_t)), fmt_int(MAINNET_ALT_BN128[op])]
        )
    for n in pair_ns:
        op = f"Pairing n={n}"
        zen3 = ZEN3_2022_ALT_BN128.get(op)
        ark_times = [per_ark.get(a, {}).get("alt_bn128", {}).get((op, "BE")) for a in arks]
        cu_t = ark_times[-1] if ark_times else None
        rows.append(
            [op, fmt_time(zen3)]
            + [fmt_time(t) for t in ark_times]
            + [fmt_int(cu(cu_t)), fmt_int(mainnet_pairing(n))]
        )
    return render_table(headers, rows)


def render_poseidon(per_ark):
    arks = arks_with_data(per_ark, "poseidon")
    cu_ark = arks[-1] if arks else "?"
    headers = (
        ["n inputs", "light-poseidon Ryzen 9 7945HX (Zen 4)"]
        + [f"M5 Pro (ark {a})" for a in arks]
        + [f"CU @ 33 ns (ark {cu_ark})", "mainnet CU (61n² + 542)"]
    )
    rows = []
    for n in (1, 2, 4, 8, 12):
        zen4 = ZEN4_LIGHT_POSEIDON.get(n)
        ark_times = [
            per_ark.get(a, {}).get("poseidon", {}).get((f"poseidon n={n}", "BE"))
            for a in arks
        ]
        cu_t = ark_times[-1] if ark_times else None
        rows.append(
            [str(n), fmt_time(zen4)]
            + [fmt_time(t) for t in ark_times]
            + [fmt_int(cu(cu_t)), fmt_int(mainnet_poseidon(n))]
        )
    return render_table(headers, rows)


def render_prepared_pairing(per_ark):
    arks = arks_with_data(per_ark, "prepared_pairing")
    cu_ark = arks[-1] if arks else "?"
    headers = (
        ["n pairs"]
        + [f"M5 Pro (ark {a})" for a in arks]
        + [f"CU @ 33 ns (ark {cu_ark})"]
    )
    rows = []
    for n in PREPARED_PAIRING_NS:
        op_key = (f"prepared n={n}", "LE")
        ark_times = [
            per_ark.get(a, {}).get("prepared_pairing", {}).get(op_key) for a in arks
        ]
        cu_t = ark_times[-1] if ark_times else None
        rows.append(
            [str(n)]
            + [fmt_time(t) for t in ark_times]
            + [fmt_int(cu(cu_t))]
        )
    return render_table(headers, rows)


def fit_prepared_pair_costs(per_ark):
    """Least-squares fit y = base + per_pair * n over the prepared-pairing samples.

    Returns (base_cu, per_pair_cu) for the latest ark, or (None, None) if insufficient.
    """
    arks = arks_with_data(per_ark, "prepared_pairing")
    if not arks:
        return None, None
    d = per_ark[arks[-1]].get("prepared_pairing", {})
    pts = []
    for n in PREPARED_PAIRING_NS:
        t = d.get((f"prepared n={n}", "LE"))
        if t is not None:
            pts.append((n, t / NS_PER_CU))
    if len(pts) < 2:
        return None, None
    xs = [n for n, _ in pts]
    ys = [y for _, y in pts]
    base, per_pair = least_squares(xs, ys)
    return int(round(base)), int(round(per_pair))


def render_compression(per_ark):
    arks = arks_with_data(per_ark, "compression")
    cu_ark = arks[-1] if arks else "?"
    ops = ("g1_compress", "g1_decompress", "g2_compress", "g2_decompress")

    def has_endian(ark, end):
        d = per_ark.get(ark, {}).get("compression", {})
        return any(d.get((op, end)) is not None for op in ops)

    columns = []  # list of (header, ark, endianness)
    for a in arks:
        if has_endian(a, "BE"):
            columns.append((f"M5 Pro ark {a} (BE)", a, "BE"))
        if has_endian(a, "LE"):
            columns.append((f"M5 Pro ark {a} (LE)", a, "LE"))

    headers = ["op", "2023 c6a.2xlarge (BE)"] + [c[0] for c in columns]
    headers.extend([f"CU @ 33 ns (ark {cu_ark}, BE)", "mainnet CU"])
    rows = []
    for op in ops:
        zen3 = ZEN3_2023_COMPRESSION.get(op)
        cells = [op, fmt_time(zen3)]
        for _, a, end in columns:
            cells.append(fmt_time(per_ark.get(a, {}).get("compression", {}).get((op, end))))
        be_latest = per_ark.get(arks[-1], {}).get("compression", {}).get((op, "BE")) if arks else None
        cells.append(fmt_int(cu(be_latest)))
        cells.append(fmt_int(MAINNET_COMPRESSION[op]))
        rows.append(cells)
    return render_table(headers, rows)


def render_table(headers, rows):
    aligns = ["---"] + [" ---: " for _ in headers[1:]]
    out = []
    out.append("| " + " | ".join(headers) + " |")
    out.append("| " + " | ".join(a.strip() for a in aligns) + " |")
    for row in rows:
        out.append("| " + " | ".join(row) + " |")
    return "\n".join(out) + "\n"


def render_proposed_cu(per_ark):
    arks = sorted(per_ark.keys())
    if not arks:
        return "(no results — run benches first)\n"
    latest = arks[-1]
    d = per_ark[latest]

    rows = []

    def get(cat, key):
        return d.get(cat, {}).get(key)

    for op in ("G1 add", "G1 mul", "G2 add", "G2 mul"):
        t = get("alt_bn128", (op, "BE"))
        rows.append((op, MAINNET_ALT_BN128[op], cu(t)))

    pair_pts = [(n, get("alt_bn128", (f"Pairing n={n}", "BE"))) for n in (2, 4, 8, 16)]
    pair_pts = [(n, t) for n, t in pair_pts if t is not None]
    if len(pair_pts) >= 2:
        xs = [n - 1 for n, _ in pair_pts]
        ys = [t / NS_PER_CU for _, t in pair_pts]
        first, other = least_squares(xs, ys)
        rows.append(("pairing first", 36_364, int(round(first))))
        rows.append(("pairing other", 12_121, int(round(other))))

    base_cu, per_pair_cu = fit_prepared_pair_costs(per_ark)
    if base_cu is not None and per_pair_cu is not None:
        rows.append(("prepared pairing base", 0, base_cu))
        rows.append(("prepared pairing per pair", 0, per_pair_cu))

    for op in ("g1_compress", "g1_decompress", "g2_compress", "g2_decompress"):
        t = get("compression", (op, "BE"))
        rows.append((op, MAINNET_COMPRESSION[op], cu(t)))

    pose_pts = [(n, get("poseidon", (f"poseidon n={n}", "BE"))) for n in (1, 2, 4, 8, 12)]
    pose_pts = [(n, t) for n, t in pose_pts if t is not None]
    if len(pose_pts) >= 2:
        xs = [n * n for n, _ in pose_pts]
        ys = [t / NS_PER_CU for _, t in pose_pts]
        c_int, a_slope = least_squares(xs, ys)
        rows.append(("poseidon coefficient `a` (per `n²`)", 61, int(round(a_slope))))
        rows.append(("poseidon coefficient `c`", 542, int(round(c_int))))

    def pct(cur, new):
        if new is None or cur in (0, None):
            return "—"
        delta = (new - cur) / cur * 100
        sign = "+" if delta > 0 else ""
        return f"{sign}{delta:.1f}%"

    out = [
        f"# CU based on current Benchmark(ark {latest}, M5 Pro)",
        "",
        f"Derived: criterion upper-bound time / {NS_PER_CU:.0f} ns/CU.",
        "",
        "| op | mainnet CU | updated CU | change |",
        "| --- | ---: | ---: | ---: |",
    ]
    for op, cur, new in rows:
        new_cell = fmt_int(new) if new is not None else "—"
        out.append(f"| {op} | {cur:,} | {new_cell} | {pct(cur, new)} |")
    return "\n".join(out) + "\n\n"


def least_squares(xs, ys):
    """Return (intercept, slope) for y = intercept + slope * x."""
    n = len(xs)
    sx, sy = sum(xs), sum(ys)
    sxx = sum(x * x for x in xs)
    sxy = sum(x * y for x, y in zip(xs, ys))
    slope = (n * sxy - sx * sy) / (n * sxx - sx * sx)
    intercept = (sy - slope * sx) / n
    return intercept, slope


def replace_between_markers(text, name, body):
    pattern = re.compile(
        rf"(<!-- TABLE-BEGIN: {re.escape(name)} -->)(.*?)(<!-- TABLE-END: {re.escape(name)} -->)",
        re.DOTALL,
    )
    if not pattern.search(text):
        sys.exit(f"missing markers for table '{name}' in README — add `<!-- TABLE-BEGIN: {name} -->` and `<!-- TABLE-END: {name} -->` lines")
    return pattern.sub(rf"\1\n{body}\3", text)


def replace_proposed_cu_section(text, body):
    return replace_between_markers(text, "proposed-cu", body)


def update_readme(per_ark):
    text = README.read_text()
    text = replace_between_markers(text, "alt_bn128", render_alt_bn128(per_ark))
    text = replace_between_markers(
        text, "prepared-pairing", render_prepared_pairing(per_ark)
    )
    text = replace_between_markers(text, "poseidon", render_poseidon(per_ark))
    text = replace_between_markers(text, "compression", render_compression(per_ark))
    text = replace_proposed_cu_section(text, render_proposed_cu(per_ark))
    README.write_text(text)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-run", action="store_true", help="skip running benches; only regenerate README from existing results/")
    ap.add_argument("--from-file", help="parse a saved criterion text file instead of running benches")
    ap.add_argument("--ark", help="override ark version detection")
    ap.add_argument(
        "--filter",
        help="run only benches matching the given criterion filter (e.g. 'BN254 prepared pairing'); "
        "merges results into existing JSON instead of overwriting",
    )
    args = ap.parse_args()

    ark = args.ark or detect_ark_version()

    if not args.no_run:
        if args.from_file:
            text = Path(args.from_file).read_text()
            merge = False
        else:
            if args.filter:
                print(f"running cargo bench against ark {ark} (filter: {args.filter!r})…", file=sys.stderr)
            else:
                print(f"running cargo bench against ark {ark}…", file=sys.stderr)
            text = run_benches(args.filter)
            merge = bool(args.filter)
        RESULTS_DIR.mkdir(exist_ok=True)
        if not args.filter:
            (RESULTS_DIR / f"ark-{ark}.txt").write_text(text)
        save_results(ark, to_structured(parse_criterion(text)), merge=merge)

    per_ark = load_results()
    if not per_ark:
        sys.exit("no results to render — run without --no-run first")
    update_readme(per_ark)
    print(f"updated {README}", file=sys.stderr)


if __name__ == "__main__":
    main()
