#!/usr/bin/env python3
"""Capture real dark PTY frames. Usage: python3 scripts/capture-review.py /absolute/path/to/dark"""
import fcntl
from collections import Counter
import html
import importlib.util
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unicodedata

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("review_fixture", ROOT / "tests/fixtures/review.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
CSI = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])")


def rss(pid):
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except (OSError, ValueError):
        pass
    return 0


def child_rss(pid):
    total = 0
    pending = [pid]
    seen = {pid}
    while pending:
        parent = pending.pop()
        try:
            children = Path(f"/proc/{parent}/task/{parent}/children").read_text().split()
        except OSError:
            continue
        for child in map(int, children):
            if child not in seen:
                seen.add(child)
                total += rss(child)
                pending.append(child)
    return total


def capture(binary, cols, rows, repo):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    def setup():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
    env = os.environ.copy()
    env.pop("NO_COLOR", None)
    env["TERM"] = "xterm-256color"
    env["COLORTERM"] = "truecolor"
    proc = subprocess.Popen([str(binary)], cwd=repo, stdin=slave, stdout=slave, stderr=slave,
                            env=env, preexec_fn=setup, close_fds=True)
    output = bytearray()
    app_peak = child_peak = 0
    try:
        deadline = time.monotonic() + 45
        last_data = time.monotonic()
        complete = f"\x1b[{rows};1H".encode()
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], 0.1)
            if ready:
                output.extend(os.read(master, 65536))
                last_data = time.monotonic()
            app_peak = max(app_peak, rss(proc.pid))
            child_peak = max(child_peak, child_rss(proc.pid))
            if proc.poll() is not None:
                raise RuntimeError(f"dark exited before capture: {bytes(output)!r}")
            if complete in output and b"Changes" in output and b"session.ts" in output and time.monotonic()-last_data > 0.5:
                break
        else:
            raise RuntimeError(f"timed out waiting for review frame: {bytes(output)!r}")
        os.write(master, b"\x1bq")
        proc.wait(timeout=10)
        if proc.returncode != 0:
            raise RuntimeError(f"dark exited with {proc.returncode}")
        return bytes(output), app_peak, child_peak
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def screen(data, cols, rows):
    """Interpret cursor and SGR commands into cells; only captured bytes determine content."""
    cells = [[(" ", None, None, False, False) for _ in range(cols)] for _ in range(rows)]
    row = col = 0
    fg = bg = None
    bold = reverse = False
    text = data.decode("utf-8", "replace")
    i = 0
    while i < len(text):
        if text[i] == "\x1b":
            if text.startswith("\x1b]", i):
                bel = text.find("\x07", i + 2)
                st = text.find("\x1b\\", i + 2)
                ends = [(position, length) for position, length in ((bel, 1), (st, 2)) if position >= 0]
                if not ends:
                    break
                position, length = min(ends)
                i = position + length
                continue
            match = CSI.match(text, i)
            if not match:
                i += 2
                continue
            arg, op = match.groups()
            i = match.end()
            if arg.startswith("?"):
                continue
            nums = [int(x) if x else 0 for x in arg.split(";")]
            if op in ("H", "f"):
                row = max(0, (nums[0] or 1)-1)
                col = max(0, (nums[1] if len(nums)>1 and nums[1] else 1)-1)
            elif op == "K" and row < rows:
                for x in range(cols):
                    cells[row][x] = (" ", None, None, False, False)
            elif op == "m":
                j = 0
                while j < len(nums):
                    n = nums[j]
                    if n == 0: fg = bg = None; bold = reverse = False
                    elif n == 1: bold = True
                    elif n == 22: bold = False
                    elif n == 7: reverse = True
                    elif n == 27: reverse = False
                    elif n in (38,48) and j+4 < len(nums) and nums[j+1] == 2:
                        value = "#%02x%02x%02x" % tuple(nums[j+2:j+5]); j += 4
                        if n == 38: fg = value
                        else: bg = value
                    j += 1
            continue
        ch = text[i]
        i += 1
        if ch in "\r\n":
            if ch == "\n": row += 1
            else: col = 0
            continue
        if ord(ch) < 32 or ord(ch) == 127:
            continue
        w = 0 if unicodedata.combining(ch) else (2 if unicodedata.east_asian_width(ch) in "WF" else 1)
        if w == 0 and 0 <= row < rows and col > 0:
            old = cells[row][col-1]
            cells[row][col-1] = (old[0] + ch, *old[1:])
        elif 0 <= row < rows and col < cols:
            cells[row][col] = (ch, fg, bg, bold, reverse)
            if w == 2 and col+1 < cols:
                cells[row][col+1] = ("", fg, bg, bold, reverse)
            col += w
    return cells


def write_html(path, cells, cols, rows):
    lines = []
    for row in cells:
        spans = []
        current = None
        chunk = []
        for ch, fg, bg, bold, reverse in row:
            style = (fg, bg, bold, reverse)
            if style != current and chunk:
                spans.append(render_chunk(chunk, current)); chunk = []
            current = style
            chunk.append(ch)
        if chunk: spans.append(render_chunk(chunk, current))
        lines.append("".join(spans))
    title = f"dark Review capture {cols}×{rows}"
    backgrounds = Counter(cell[2] for row in cells for cell in row if cell[2])
    background = backgrounds.most_common(1)[0][0] if backgrounds else "transparent"
    path.write_text("<!doctype html><meta charset='utf-8'><title>" + title + "</title>"
                    "<style>body{margin:0;background:" + background + "}pre{margin:0;line-height:1.15;"
                    "font:14px/1.15 Consolas,'DejaVu Sans Mono',monospace;white-space:pre}</style>"
                    "<pre>" + "\n".join(lines) + "</pre>", encoding="utf-8")


def render_chunk(chunk, style):
    fg, bg, bold, reverse = style
    if reverse: fg, bg = bg, fg
    attrs = (f"color:{fg};" if fg else "") + (f"background:{bg};" if bg else "") + ("font-weight:bold;" if bold else "")
    return f'<span style="{attrs}">{html.escape("".join(chunk))}</span>'


def main():
    if not sys.platform.startswith("linux") or len(sys.argv) != 2:
        raise SystemExit(__doc__)
    binary = Path(sys.argv[1]).resolve(strict=True)
    destination = ROOT / "docs/ui/m1"
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="dark-capture-") as temp:
        repo = Path(temp) / "shop-api"
        fixture.create(repo)
        for cols, rows in ((160,44), (80,24)):
            data, app_peak, child_peak = capture(binary, cols, rows, repo)
            name = f"{cols}x{rows}"
            (destination / (name + ".ansi")).write_bytes(data)
            cells = screen(data, cols, rows)
            (destination / (name + ".txt")).write_text("\n".join("".join(c[0] for c in row).rstrip() for row in cells) + "\n", encoding="utf-8")
            write_html(destination / (name + ".html"), cells, cols, rows)
            print(f"Captured {name} ({len(data)} bytes); app_peak_sampled_rss_bytes={app_peak} child_tree_peak_sampled_rss_bytes={child_peak}")


if __name__ == "__main__":
    main()
