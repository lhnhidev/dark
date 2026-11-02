#!/usr/bin/env python3
"""Exercise automatic refresh in a real Linux PTY, optionally on a WSL drive.

Usage: python3 scripts/test-watch-refresh.py /path/to/release/dark [fixture-parent]
The fixture parent can be on /mnt/d to exercise the Windows bridge. Reports
Linux app VmRSS; Windows bridge memory needs an independent Windows EX2 probe.
"""
import argparse
import base64
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import statistics
import subprocess
import tempfile
import termios
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("fixture_parent", nargs="?")
    parser.add_argument("--windows-writer", action="store_true")
    parser.add_argument("--kill-owner", action="store_true")
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    parent = args.fixture_parent
    with tempfile.TemporaryDirectory(prefix="dark-refresh-", dir=parent) as temp:
        root = Path(temp)
        def git(*args):
            subprocess.run(["git", *args], cwd=root, check=True, stdout=subprocess.DEVNULL)
        git("init", "-q")
        git("config", "user.name", "Test")
        git("config", "user.email", "test@example.invalid")
        tracked = root / "sample.txt"
        tracked.write_text("original\n", encoding="utf-8")
        git("add", "sample.txt")
        git("commit", "-qm", "initial")
        tracked.write_text("initialchange\n", encoding="utf-8")
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 44, 160, 0, 0))
        env = os.environ.copy()
        env["TERM"] = "xterm-256color"
        process = subprocess.Popen([binary], cwd=root, stdin=slave, stdout=slave,
                                   stderr=slave, env=env, start_new_session=True)
        output = bytearray()
        def wait_for(value, start, timeout=12):
            deadline = time.monotonic() + timeout
            while value not in output[start:]:
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"missing {value!r}; output tail: {bytes(output[-5000:])!r}")
                if select.select([master], [], [], 0.05)[0]:
                    output.extend(os.read(master, 65536))
        def send(key):
            os.write(master, key)
        def write_external(path, value, atomic=False):
            if not args.windows_writer:
                if atomic:
                    replacement = root / "replacement.tmp"
                    replacement.write_text(value, encoding="utf-8")
                    replacement.replace(path)
                else:
                    path.write_text(value, encoding="utf-8")
                return
            winpath = subprocess.check_output(["wslpath", "-w", str(path)], text=True).strip()
            path64 = base64.b64encode(winpath.encode("utf-8")).decode("ascii")
            value64 = base64.b64encode(value.encode("utf-8")).decode("ascii")
            command = (f"$p=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{path64}'));"
                       f"$v=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{value64}'));"
                       "$u=New-Object System.Text.UTF8Encoding($false);")
            if atomic:
                command += "$t=$p+'.tmp';$b=$p+'.bak';[IO.File]::WriteAllText($t,$v,$u);[IO.File]::Replace($t,$p,$b);[IO.File]::Delete($b)"
            else:
                command += "[IO.File]::WriteAllText($p,$v,$u)"
            encoded = base64.b64encode(command.encode("utf-16le")).decode("ascii")
            subprocess.run(["powershell.exe", "-NoProfile", "-EncodedCommand", encoded], check=True)
        try:
            wait_for(b"initialchange", 0)
            time.sleep(1)
            # Drain the initial frame and check that idle snapshot writes do not
            # continually trigger new frames. No keys or writes occur here.
            while select.select([master], [], [], 0)[0]:
                output.extend(os.read(master, 65536))
            idle_start = len(output)
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0]:
                    output.extend(os.read(master, 65536))
            if b"\x1b]0;" in output[idle_start:]:
                raise RuntimeError("unexpected redraw while idle")
            children_path = Path(f"/proc/{process.pid}/task/{process.pid}/children")
            children = [int(pid) for pid in children_path.read_text().split()]
            for child in children:
                child_status = Path(f"/proc/{child}/status").read_text()
                child_rss = next((line for line in child_status.splitlines() if line.startswith("VmRSS:")), "unavailable")
                print(f"Linux child {child} {child_rss}")
            if children and parent:
                # EX2 private working set is distinct from the Linux proxy VmRSS.
                probe = """
Add-Type -TypeDefinition @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
public static class BridgeMemory {
 [StructLayout(LayoutKind.Sequential)] struct MEM { public uint cb, faults; public UIntPtr peak, working, qpp, qp, qnp, qn, page, peakpage, privateUsage, privateWorking, shared; }
 [DllImport("psapi.dll",SetLastError=true)] static extern bool GetProcessMemoryInfo(IntPtr h,ref MEM m,uint size);
 public static ulong Read(int id) { using (var p=Process.GetProcessById(id)) { var m=new MEM(); m.cb=(uint)Marshal.SizeOf(typeof(MEM)); if(!GetProcessMemoryInfo(p.Handle,ref m,m.cb)) throw new Exception("GetProcessMemoryInfo EX2 failed"); return m.privateWorking.ToUInt64(); } }
}
'@
"""
                probe += ("$p=Get-CimInstance Win32_Process -Filter \"Name='dark.exe'\" | "
                          f"Where-Object {{$_.CommandLine -like '*{root.name}*' -and "
                          "$_.CommandLine -like '*--watch-bridge*'}; "
                          "if (@($p).Count -ne 1) {throw 'bridge process not uniquely identified'}; "
                          "1..5 | ForEach-Object {[BridgeMemory]::Read($p.ProcessId); Start-Sleep -Milliseconds 100}")
                encoded = base64.b64encode(probe.encode("utf-16le")).decode("ascii")
                try:
                    result = subprocess.check_output(["powershell.exe", "-NoProfile", "-EncodedCommand", encoded], text=True, stderr=subprocess.DEVNULL)
                    values = [int(line) for line in result.splitlines() if line.strip().isdigit()]
                    if len(values) != 5 or any(value <= 0 for value in values):
                        raise ValueError("invalid EX2 samples")
                    print(f"Windows bridge EX2 private working set median: {int(statistics.median(values))} bytes; samples: {values}")
                except (subprocess.CalledProcessError, ValueError) as error:
                    print(f"Windows bridge EX2 private working set unavailable: {error}")
            status = Path(f"/proc/{process.pid}/status").read_text()
            line = next(line for line in status.splitlines() if line.startswith("VmRSS:"))
            rss = int(line.split()[1]) * 1024
            print(f"app idle VmRSS: {rss} bytes")
            if rss <= 0 or rss >= 2_000_000:
                raise RuntimeError("app idle VmRSS exceeds budget or unavailable")
            if args.kill_owner:
                # Query the real Windows process, not only the WSL proxy PID.
                def windows_helpers():
                    cmd = ("Get-CimInstance Win32_Process -Filter \"Name='dark.exe'\" | "
                           f"Where-Object {{$_.CommandLine -like '*{root.name}*' -and "
                           "$_.CommandLine -like '*--watch-bridge*'} | "
                           "ForEach-Object {$_.ProcessId}")
                    encoded = base64.b64encode(cmd.encode("utf-16le")).decode("ascii")
                    result = subprocess.check_output(["powershell.exe", "-NoProfile", "-EncodedCommand", encoded], text=True)
                    return [int(line) for line in result.splitlines() if line.strip().isdigit()]
                helpers = windows_helpers()
                if not helpers:
                    raise RuntimeError("Windows bridge process not found")
                process.kill()
                process.wait(timeout=10)
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline and windows_helpers():
                    time.sleep(0.2)
                remaining = windows_helpers()
                if remaining:
                    raise RuntimeError(f"Windows bridge survives owner SIGKILL: {remaining}")
                print(f"PASS: Windows bridge {helpers} exited after owner SIGKILL")
                return
            start = len(output)
            write_external(tracked, "externalchange\n")
            wait_for(b"externalchange", start)
            start = len(output)
            write_external(tracked, "atomicchange\n", atomic=True)
            wait_for(b"atomicchange", start)
            start = len(output)
            write_external(root / "brandnew.txt", "new file\n")
            wait_for(b"brandnew.txt", start)
            start = len(output)
            send(b"u")
            wait_for("Khôi phục".encode(), start)
            tracked.write_text("changedafterprompt\n", encoding="utf-8")
            time.sleep(1)
            send(b"y")
            time.sleep(1)
            if tracked.read_text(encoding="utf-8") != "changedafterprompt\n":
                raise RuntimeError("confirmation overwrote an external change")
            send(b"q")
            process.wait(timeout=10)
            if process.returncode:
                raise RuntimeError(f"dark exited {process.returncode}")
            for child in children:
                if Path(f"/proc/{child}").exists():
                    raise RuntimeError(f"helper process {child} remains after exit")
            print("PASS: external write, atomic replacement, new file, confirmation safety, idle, q, Linux child shutdown")
            print(f"writer: {'Windows' if args.windows_writer else 'Linux'}; child PIDs: {children}")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
