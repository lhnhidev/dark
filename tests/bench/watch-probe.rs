//! Isolated change-detection probe; not part of Dark or its RAM acceptance tests.
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::{self, Write},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn fingerprint(root: &Path, state: &mut DefaultHasher) -> io::Result<()> {
    // No retained per-file cache; include names, lengths and timestamps.
    // The fixture has no symlinks or ignored directories.
    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        entry.file_name().hash(state);
        let meta = entry.metadata()?;
        meta.len().hash(state);
        meta.modified()?.hash(state);
        if meta.is_dir() {
            fingerprint(&entry.path(), state)?;
        }
    }
    Ok(())
}
fn scan(root: &Path) -> io::Result<u64> {
    let mut state = DefaultHasher::new();
    fingerprint(root, &mut state)?;
    Ok(state.finish())
}

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use std::ffi::CString;
    use std::{
        os::raw::{c_char, c_int, c_short, c_void},
        os::unix::ffi::OsStrExt,
    };
    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: c_short,
        revents: c_short,
    }
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    extern "C" {
        fn inotify_init1(flags: c_int) -> c_int;
        fn inotify_add_watch(fd: c_int, name: *const c_char, mask: u32) -> c_int;
        fn poll(fds: *mut PollFd, count: usize, timeout: c_int) -> c_int;
        fn read(fd: c_int, buf: *mut c_void, len: usize) -> isize;
        fn close(fd: c_int) -> c_int;
        fn clock_gettime(clock: c_int, time: *mut Timespec) -> c_int;
    }
    pub struct Watch(c_int);
    impl Watch {
        pub fn new(root: &Path) -> io::Result<Self> {
            let fd = unsafe { inotify_init1(0x800 | 0x80000) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let watch = Self(fd);
            watch.add(root)?;
            Ok(watch)
        }
        fn add(&self, root: &Path) -> io::Result<()> {
            let name = CString::new(root.as_os_str().as_bytes())?;
            // close-write, moved-from/to, create, delete, attributes.
            if unsafe {
                inotify_add_watch(
                    self.0,
                    name.as_ptr(),
                    0x8 | 0x40 | 0x80 | 0x100 | 0x200 | 0x4,
                )
            } < 0
            {
                return Err(io::Error::last_os_error());
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    self.add(&entry.path())?;
                }
            }
            Ok(())
        }
        pub fn wait(&self, timeout: Duration) -> io::Result<bool> {
            let mut fd = PollFd {
                fd: self.0,
                events: 1,
                revents: 0,
            };
            let result =
                unsafe { poll(&mut fd, 1, timeout.as_millis().min(i32::MAX as u128) as i32) };
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            if result == 0 {
                return Ok(false);
            }
            let mut buffer = [0u8; 4096];
            if unsafe { read(self.0, buffer.as_mut_ptr().cast(), buffer.len()) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(true)
        }
    }
    impl Drop for Watch {
        fn drop(&mut self) {
            unsafe {
                close(self.0);
            }
        }
    }
    pub fn cpu_ns() -> io::Result<u128> {
        let mut time = Timespec { sec: 0, nsec: 0 };
        if unsafe { clock_gettime(2, &mut time) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(time.sec as u128 * 1_000_000_000 + time.nsec as u128)
    }
    pub fn memory() -> io::Result<u64> {
        let text = fs::read_to_string("/proc/self/status")?;
        let value = text
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        Ok(value.parse::<u64>().unwrap() * 1024)
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    type Handle = *mut c_void;
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct Memory {
        cb: u32,
        faults: u32,
        peak_ws: usize,
        ws: usize,
        peak_paged: usize,
        paged: usize,
        peak_nonpaged: usize,
        nonpaged: usize,
        pagefile: usize,
        peak_pagefile: usize,
        private_usage: usize,
        private_ws: usize,
        shared_commit: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn FindFirstChangeNotificationW(path: *const u16, subtree: i32, filter: u32) -> Handle;
        fn FindNextChangeNotification(handle: Handle) -> i32;
        fn FindCloseChangeNotification(handle: Handle) -> i32;
        fn WaitForSingleObject(handle: Handle, ms: u32) -> u32;
        fn GetCurrentProcess() -> Handle;
        fn GetProcessTimes(
            handle: Handle,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn K32GetProcessMemoryInfo(handle: Handle, counters: *mut Memory, size: u32) -> i32;
    }
    pub struct Watch(Handle);
    impl Watch {
        pub fn new(root: &Path) -> io::Result<Self> {
            let name: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
            let handle =
                unsafe { FindFirstChangeNotificationW(name.as_ptr(), 1, 1 | 2 | 4 | 8 | 16) };
            if handle as isize == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(handle))
        }
        pub fn wait(&self, timeout: Duration) -> io::Result<bool> {
            match unsafe {
                WaitForSingleObject(self.0, timeout.as_millis().min(u32::MAX as u128 - 1) as u32)
            } {
                0 => {
                    if unsafe { FindNextChangeNotification(self.0) } == 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(true)
                }
                258 => Ok(false),
                _ => Err(io::Error::last_os_error()),
            }
        }
    }
    impl Drop for Watch {
        fn drop(&mut self) {
            unsafe {
                FindCloseChangeNotification(self.0);
            }
        }
    }
    pub fn cpu_ns() -> io::Result<u128> {
        let mut times: [FileTime; 4] = unsafe { std::mem::zeroed() };
        let [creation, exit, kernel, user] = &mut times;
        if unsafe { GetProcessTimes(GetCurrentProcess(), creation, exit, kernel, user) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let ns = |time: &FileTime| ((u128::from(time.high) << 32) | u128::from(time.low)) * 100;
        Ok(ns(kernel) + ns(user))
    }
    pub fn memory() -> io::Result<u64> {
        let mut counters: Memory = unsafe { std::mem::zeroed() };
        counters.cb = std::mem::size_of::<Memory>() as u32;
        if unsafe {
            K32GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                std::mem::size_of::<Memory>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if counters.private_ws == 0 {
            return Err(io::Error::other("EX2 private working set unavailable"));
        }
        Ok(counters.private_ws as u64)
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args[1] == "write" {
        let started = stamp();
        fs::write(&args[2], format!("change {}\n", stamp()))?;
        println!("{started}");
        return Ok(());
    }
    let mode = &args[1];
    let root = Path::new(&args[2]);
    let seconds: u64 = args[3].parse().unwrap();
    let watcher = if mode == "notify" {
        Some(native::Watch::new(root)?)
    } else {
        None
    };
    let mut previous = if mode == "poll" { scan(root)? } else { 0 };
    let memory_start = native::memory()?;
    let cpu_start = native::cpu_ns()?;
    let started = Instant::now();
    println!("READY {}", stamp());
    io::stdout().flush()?;
    let duration = Duration::from_secs(seconds);
    let mut scans = 0;
    while started.elapsed() < duration {
        let remaining = duration.saturating_sub(started.elapsed());
        let changed = if let Some(watcher) = &watcher {
            watcher.wait(remaining)?
        } else if mode == "poll" {
            std::thread::sleep(remaining.min(Duration::from_secs(1)));
            scans += 1;
            let current = scan(root)?;
            let changed = current != previous;
            previous = current;
            changed
        } else {
            std::thread::sleep(remaining);
            false
        };
        if changed {
            println!("EVENT {}", stamp());
            io::stdout().flush()?;
        }
    }
    let cpu = native::cpu_ns()? - cpu_start;
    let memory_end = native::memory()?;
    println!(
        "RESULT memory_start={memory_start} memory_end={memory_end} cpu_ns={cpu} scans={scans}"
    );
    Ok(())
}
