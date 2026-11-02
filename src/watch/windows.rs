use super::{git_common_dir, git_dir, metadata_path, worktree_path};
use std::ffi::c_void;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

type Handle = *mut c_void;
#[repr(C)]
#[derive(Default)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const c_void,
        creation: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn CreateEventW(
        attributes: *const c_void,
        manual: i32,
        initial: i32,
        name: *const u16,
    ) -> Handle;
    fn ReadDirectoryChangesW(
        handle: Handle,
        buffer: *mut c_void,
        size: u32,
        subtree: i32,
        filter: u32,
        returned: *mut u32,
        overlapped: *mut Overlapped,
        completion: *const c_void,
    ) -> i32;
    fn GetOverlappedResult(
        handle: Handle,
        overlapped: *mut Overlapped,
        returned: *mut u32,
        wait: i32,
    ) -> i32;
    fn CancelIoEx(handle: Handle, overlapped: *mut Overlapped) -> i32;
    fn WaitForSingleObject(handle: Handle, timeout: u32) -> u32;
    fn ResetEvent(handle: Handle) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

struct Directory {
    handle: Handle,
    overlapped: Box<Overlapped>,
    // Both addresses must remain stable while an asynchronous read is pending.
    buffer: Box<[u32; 4096]>,
    metadata: bool,
    pending: bool,
}

impl Directory {
    fn new(path: &Path, metadata: bool) -> io::Result<Self> {
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe { CreateFileW(name.as_ptr(), 1, 7, null(), 3, 0x42000000, null_mut()) };
        if handle == -1isize as Handle {
            return Err(io::Error::last_os_error());
        }
        let event = unsafe { CreateEventW(null(), 1, 0, null()) };
        if event.is_null() {
            unsafe {
                CloseHandle(handle);
            }
            return Err(io::Error::last_os_error());
        }
        let mut result = Self {
            handle,
            overlapped: Box::new(Overlapped {
                event,
                ..Overlapped::default()
            }),
            buffer: Box::new([0; 4096]),
            metadata,
            pending: false,
        };
        result.arm()?;
        Ok(result)
    }

    fn arm(&mut self) -> io::Result<()> {
        unsafe {
            ResetEvent(self.overlapped.event);
        }
        let ok = unsafe {
            ReadDirectoryChangesW(
                self.handle,
                self.buffer.as_mut_ptr().cast(),
                16384,
                1,
                0x1b,
                null_mut(),
                &mut *self.overlapped,
                null(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        self.pending = true;
        Ok(())
    }

    fn take_changed(&mut self) -> io::Result<bool> {
        match unsafe { WaitForSingleObject(self.overlapped.event, 0) } {
            258 => return Ok(false),
            0 => {}
            _ => return Err(io::Error::last_os_error()),
        }
        let mut count = 0;
        let ok = unsafe { GetOverlappedResult(self.handle, &mut *self.overlapped, &mut count, 0) };
        self.pending = false;
        if ok == 0 {
            let error = io::Error::last_os_error();
            // ERROR_NOTIFY_ENUM_DIR means the OS queue overflowed: refresh once.
            if error.raw_os_error() != Some(1022) {
                return Err(error);
            }
            self.arm()?;
            return Ok(true);
        }
        let bytes = unsafe {
            std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), count as usize)
        };
        let mut offset = 0;
        let mut changed = count == 0;
        while offset + 12 <= bytes.len() {
            let next = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            let length =
                u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
            if !length.is_multiple_of(2) || offset + 12 + length > bytes.len() {
                changed = true;
                break;
            }
            let name: Vec<u16> = bytes[offset + 12..offset + 12 + length]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            let path = PathBuf::from(std::ffi::OsString::from_wide(&name));
            changed |= if self.metadata {
                metadata_path(&path)
            } else {
                worktree_path(&path)
            };
            if next == 0 {
                break;
            }
            if next < 12 || next > bytes.len() - offset {
                changed = true;
                break;
            }
            offset += next;
        }
        // Re-arm before the caller reloads Git, so writes during refresh survive.
        self.arm()?;
        Ok(changed)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        unsafe {
            if self.pending {
                CancelIoEx(self.handle, &mut *self.overlapped);
                let mut returned = 0;
                GetOverlappedResult(self.handle, &mut *self.overlapped, &mut returned, 1);
            }
            CloseHandle(self.handle);
            CloseHandle(self.overlapped.event);
        }
    }
}

pub struct Watcher {
    tree: Directory,
    git: Directory,
    common: Option<Directory>,
}
impl Watcher {
    pub fn new(root: &Path) -> io::Result<Self> {
        let git = git_dir(root)?;
        let common = git_common_dir(root)?;
        Ok(Self {
            tree: Directory::new(root, false)?,
            git: Directory::new(&git, true)?,
            common: if common != git {
                Some(Directory::new(&common, true)?)
            } else {
                None
            },
        })
    }
    pub fn take_changed(&mut self) -> io::Result<bool> {
        let tree = self.tree.take_changed()?;
        let git = self.git.take_changed()?;
        let common = match &mut self.common {
            Some(common) => common.take_changed()?,
            None => false,
        };
        Ok(tree || git || common)
    }
}
