# Change-detection probe

This is an isolated benchmark, not a change to Dark. It compares a passive
baseline, recursive metadata polling every second, and native notifications
(`FindFirstChangeNotificationW` with a subtree on Windows, recursive `inotify`
watches on Linux). The Windows strategy reports that something changed, not
which file changed. Both strategies would still need Git/diff refresh in Dark.

No new dependencies. Build with the existing release settings:

```powershell
New-Item -ItemType Directory -Force .dark/verification/watch-probe
rustc --edition=2021 tests/bench/watch-probe.rs -D warnings -C opt-level=z -C lto=fat -C codegen-units=1 -C panic=abort -C strip=symbols -o .dark/verification/watch-probe/watch-probe.exe
```

Then in WSL (use the existing musl toolchain):

```bash
cd /mnt/d/workspace/dark
export PATH="$HOME/.cargo/bin:$PATH"
rustc --edition=2021 tests/bench/watch-probe.rs -D warnings --target x86_64-unknown-linux-musl -C opt-level=z -C lto=fat -C codegen-units=1 -C panic=abort -C strip=symbols -o .dark/verification/watch-probe/watch-probe-linux
python3 tests/bench/compare-watch.py \
  --linux "$PWD/.dark/verification/watch-probe/watch-probe-linux" \
  --windows "$PWD/.dark/verification/watch-probe/watch-probe.exe" \
  --output "$PWD/.dark/verification/watch-probe"
```

The runner requires Windows executable interop and `wslpath`. It uses a temporary
Linux fixture and a generated drive fixture under the output directory. Each has
30 files / 3,000 lines in three subdirectories. Only fixture files are written.
There are three runs per mode and environment, each seven seconds, with three
writes spaced more than one second apart. Baseline runs do not write. There is
no excluded warm-up run; setup occurs before READY and CPU measurement, apart
from a common final memory sample excluded from CPU timing.

Memory is the greater of two resident samples per run, summarized by median:
Windows EX2 private working set; Linux VmRSS. This is **not peak memory**. CPU
is process user + kernel time over the interval, excluding startup/setup and
writer processes. Writer processes run separately; their RAM is not measured.
Windows CPU counters can report zero over this short interval due to their
coarse accounting resolution; this does not mean that no CPU work occurred.
Kernel allocations for watch registrations are not included in the process RAM.
Neither Git subprocesses nor rendering run here. Do not apply Dark's RAM budget
to these numbers, or add probe RAM to Dark's RAM to predict its eventual total.

Latency is from the start of the fixture write to notification detection,
excluding writer startup but including file write/close time. Sub-ms values are
approximate. This measures detection, not when a refreshed diff reaches screen.
Latency is omitted for a Windows writer and WSL detector because their wall
clocks may differ. Detection counts use events, capped at the number of writes;
this fixed workload separates writes by more than one polling interval.
Polling and native Windows/Linux cases must detect every write; drive-notify
cases are observations, because WSL interop may not supply events. CPU includes
the shared cost of printing notifications. Polling checks metadata, not content;
same-size edits preserving timestamps are outside this probe.

Limits: fixed fixture only; no symlinks, ignored directories, new directories,
watch overflow recovery, atomic-save variants, Git/index changes, terminal input
integration, debounce, or production error handling. The Linux recursive setup
allocates watches in the kernel per directory; costs can grow with repo size.
Production behavior and the full Dark release require separate tests and RAM
measurements before any implementation is accepted.

Raw JSONL, summary JSON and binaries are generated under `.dark/` only.

## Measured on 2026-11-02

Rust 1.99.1 stable; Windows x86_64 MSVC and Ubuntu 22.04 WSL2, kernel
6.6.114.1-microsoft-standard-WSL2, x86_64 musl. Drive D is mounted via 9p.
Three seven-second runs per mode/environment, nine writes per active mode.
All numbers below are medians. RAM is bytes, CPU is milliseconds per seven-second
run; latency is milliseconds from write start. The two RAM metrics are different
and must not be used for cross-platform ranking.

| Detector / writer / filesystem | Mode | Process RAM | CPU ms / 7s | Writes detected | Latency ms |
|---|---|---:|---:|---:|---:|
| Linux / Linux / ext4 | Baseline | 364,544 | 0.137 | — | — |
| Linux / Linux / ext4 | Poll | 368,640 | 2.295 | 9/9 | 769.427 |
| Linux / Linux / ext4 | Notify | 376,832 | 0.285 | 9/9 | 0.298 |
| WSL / Linux / D: | Baseline | 368,640 | 0.163 | — | — |
| WSL / Linux / D: | Poll | 372,736 | 28.529 | 9/9 | 610.396 |
| WSL / Linux / D: | Notify | 348,160 | 0.145 | 0/9 | — |
| WSL / Windows / D: | Baseline | 368,640 | 0.115 | — | — |
| WSL / Windows / D: | Poll | 352,256 | 29.659 | 9/9 | — |
| WSL / Windows / D: | Notify | 368,640 | 0.152 | 0/9 | — |
| Windows / Windows / D: | Baseline | 372,736 | < accounting resolution | — | — |
| Windows / Windows / D: | Poll | 557,056 | < accounting resolution | 9/9 | 712.912 |
| Windows / Windows / D: | Notify | 372,736 | < accounting resolution | 9/9 | 0.242 |

Linux RAM is VmRSS; Windows RAM is EX2 private working set. An individual Windows
notify run recorded 15.625 ms CPU; the median was zero. Windows emitted two events
per write, so a production implementation needs to coalesce them. Polling emitted
one per write in this fixture. Windows probe binary: 155,136 bytes; musl probe:
414,384 bytes (one binary supports all modes, so no per-mode binary-size claim).

Process RAM differs by only a few resident pages on Linux; baseline variation
is comparable, so this does not establish a meaningful Linux RAM winner. Native
notification used less process RAM than this scanning implementation on Windows.
It also detected changes much faster on native filesystems. On this WSL D: mount,
notification failed for both writers, so polling is the working option. These are
small-fixture observations, not general bounds or Dark acceptance results.

Evidence: the first 18 rows of `.dark/verification/watch-probe/final/results.jsonl`
contain the complete Linux and WSL/Linux-writer runs; all 18 rows and summary in
`.dark/verification/watch-probe/final-windows/` contain the rerun Windows-writer
and native Windows cases. An earlier cross-OS latency attempt was discarded due
to clock differences; that case was rerun with latency omitted. All completed
poll runs detected every write. Native ext4/Windows notification runs detected
every write; WSL D: notification detected none. Probe builds pass `-D warnings`
on Windows and musl; standalone rustfmt check passes. Dark itself is unchanged;
the full application verification suite was not rerun for this isolated probe.
