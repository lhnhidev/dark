use std::io;

#[cfg(windows)]
pub const METRIC: &str = "private_working_set";
#[cfg(not(windows))]
pub const METRIC: &str = "rss";

#[cfg(windows)]
pub fn current_bytes() -> io::Result<u64> {
    use windows_sys::Win32::System::{
        ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
        },
        Threading::GetCurrentProcess,
    };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
    // EX2 extends the base structure; cb requests every field including private WS.
    let mut counters: PROCESS_MEMORY_COUNTERS_EX2 = unsafe { std::mem::zeroed() };
    counters.cb = size;
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    if counters.PrivateWorkingSetSize == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "private working set unavailable from Windows EX2 counters",
        ));
    }
    Ok(counters.PrivateWorkingSetSize as u64)
}

#[cfg(target_os = "linux")]
pub fn current_bytes() -> io::Result<u64> {
    use std::io::BufRead;
    let mut reader = io::BufReader::new(std::fs::File::open("/proc/self/status")?);
    let mut line = String::new();
    while reader.read_line(&mut line)? != 0 {
        if line.starts_with("VmRSS:") {
            return parse_rss(&line);
        }
        line.clear();
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "VmRSS missing"))
}

#[cfg(any(target_os = "linux", test))]
fn parse_rss(line: &str) -> io::Result<u64> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid VmRSS");
    let mut fields = line.split_whitespace();
    if fields.next() != Some("VmRSS:") {
        return Err(invalid());
    }
    let kb: u64 = fields
        .next()
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())?;
    if fields.next() != Some("kB") || fields.next().is_some() {
        return Err(invalid());
    }
    kb.checked_mul(1024).ok_or_else(invalid)
}

#[cfg(target_os = "macos")]
pub fn current_bytes() -> io::Result<u64> {
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDTASKINFO,
            0,
            (&mut info as *mut libc::proc_taskinfo).cast(),
            size,
        )
    };
    if read != size {
        return Err(if read <= 0 {
            io::Error::last_os_error()
        } else {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete process memory information",
            )
        });
    }
    Ok(info.pti_resident_size)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn current_bytes() -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "memory metric unavailable on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_kernel_kib_as_bytes() {
        assert_eq!(parse_rss("VmRSS:\t 1234 kB\n").unwrap(), 1_263_616);
    }
    #[test]
    fn rejects_missing_units_malformed_values_and_overflow() {
        for value in [
            "VmRSS: 1234",
            "VmRSS: no kB",
            "VmSize: 5 kB",
            "VmRSS: 18446744073709551615 kB",
            "VmRSS: 5 MB",
            "VmRSS: 5 kB extra",
        ] {
            assert!(parse_rss(value).is_err(), "{value}");
        }
    }
}
