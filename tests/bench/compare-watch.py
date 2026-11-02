#!/usr/bin/env python3
"""Run isolated release probes from WSL; no application RAM-budget claims.

Uses fixed 30-file / 3,000-line fixtures in three directories. Measures passive
baseline, 1s metadata polling and native notifications. Windows memory is EX2
private working set, Linux memory is VmRSS. Kernel watch memory is not included.
No Git/diff loading, rendering, new-directory registration or overflow recovery
is benchmarked. Do not use this probe as a production watcher implementation.
"""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def fixture(root):
    for group in range(3):
        directory = root / f"group-{group}"
        directory.mkdir(parents=True, exist_ok=True)
        for index in range(10):
            (directory / f"file-{index}.rs").write_text("fn sample() {}\n" * 100)


def windows_path(path):
    return subprocess.check_output(["wslpath", "-w", str(path)], text=True).strip()


def measure(binary, root, mode, writer, seconds):
    command_root = windows_path(root) if str(binary).endswith(".exe") else str(root)
    process = subprocess.Popen([str(binary), mode, command_root, str(seconds)],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    ready = process.stdout.readline().strip()
    if not ready.startswith("READY "):
        raise RuntimeError((ready, process.communicate()))
    writes = []
    # Three writes >1s apart; vary their phase relative to the polling clock.
    # First 2.23s are idle. Writer startup is excluded from notification latency.
    started = time.monotonic()
    if mode != "baseline":
        for index, offset in enumerate((2.23, 3.67, 5.11)):
            time.sleep(max(0, started + offset - time.monotonic()))
            target = root / "group-1" / f"file-{index}.rs"
            if writer:
                timestamp = int(subprocess.check_output(
                    [str(writer), "write", windows_path(target)], text=True).strip())
            else:
                timestamp = time.time_ns()
                target.write_text(f"changed {timestamp}\n")
            writes.append(timestamp)
    output, errors = process.communicate(timeout=seconds + 15)
    if process.returncode:
        raise RuntimeError(errors)
    events = [int(line.split()[1]) for line in output.splitlines() if line.startswith("EVENT ")]
    result_line = next(line for line in output.splitlines() if line.startswith("RESULT "))
    result = dict(item.split("=") for item in result_line.split()[1:])
    result = {key: int(value) for key, value in result.items()}
    latencies = []
    for index, timestamp in enumerate(writes):
        end = writes[index + 1] if index + 1 < len(writes) else timestamp + 2_000_000_000
        match = next((event for event in events if timestamp <= event < end), None)
        latencies.append(None if match is None else max(0, match - timestamp) / 1_000_000)
    if writer and not str(binary).endswith(".exe"):
        # Windows and WSL wall clocks are not guaranteed to be synchronized.
        latencies = []
    result.update(latency_ms=latencies, events=len(events), writes=len(writes))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--linux", type=Path, required=True)
    parser.add_argument("--windows", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--case", action="append", choices=("linux-ext4", "wsl-drive-linux-write",
                        "wsl-drive-windows-write", "windows-drive"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="dark-watch-") as temporary:
        linux_root = Path(temporary)
        drive_root = args.output / "fixture"
        fixture(linux_root)
        fixture(drive_root)
        cases = [("linux-ext4", args.linux, linux_root, None),
                 ("wsl-drive-linux-write", args.linux, drive_root, None),
                 ("wsl-drive-windows-write", args.linux, drive_root, args.windows),
                 ("windows-drive", args.windows, drive_root, args.windows)]
        if args.case:
            cases = [case for case in cases if case[0] in args.case]
        results = []
        with (args.output / "results.jsonl").open("w") as log:
            for case, binary, root, writer in cases:
                for run in range(args.runs):
                    for mode in ("baseline", "poll", "notify"):
                        row = dict(case=case, mode=mode, run=run + 1,
                                   **measure(binary, root, mode, writer, 7))
                        if mode == "poll" and row["events"] != row["writes"]:
                            raise RuntimeError(f"Polling missed a fixture write: {row}")
                        if mode == "notify" and case in ("linux-ext4", "windows-drive"):
                            if row["events"] < row["writes"] or any(value is None for value in row["latency_ms"]):
                                raise RuntimeError(f"Native watcher missed a fixture write: {row}")
                        results.append(row)
                        line = json.dumps(row)
                        print(line, flush=True)
                        log.write(line + "\n")
                        log.flush()
        summaries = []
        for case, *_ in cases:
            for mode in ("baseline", "poll", "notify"):
                rows = [r for r in results if r["case"] == case and r["mode"] == mode]
                latencies = [value for r in rows for value in r["latency_ms"] if value is not None]
                summaries.append(dict(case=case, mode=mode,
                    memory_median=statistics.median(max(r["memory_start"], r["memory_end"]) for r in rows),
                    cpu_ms_median=statistics.median(r["cpu_ns"] / 1_000_000 for r in rows),
                    detected=sum(min(r["events"], r["writes"]) for r in rows), writes=sum(r["writes"] for r in rows),
                    latency_ms_median=statistics.median(latencies) if latencies else None,
                    latency_ms_max=max(latencies) if latencies else None))
        (args.output / "summary.json").write_text(json.dumps(summaries, indent=2) + "\n")
        print(json.dumps(summaries, indent=2))


if __name__ == "__main__":
    main()
