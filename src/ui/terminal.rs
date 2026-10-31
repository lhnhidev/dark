use std::io::{self, Write};
#[cfg(windows)]
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

static INSTALL_HOOK: Once = Once::new();
static RAW_CHANGED: AtomicBool = AtomicBool::new(false);
static ALTERNATE: AtomicBool = AtomicBool::new(false);
static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static SAVED_MODES: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static INPUT_MODE: AtomicU32 = AtomicU32::new(0);
#[cfg(windows)]
static OUTPUT_MODE: AtomicU32 = AtomicU32::new(0);

pub struct TerminalSession;

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        // The hook also runs under panic=abort, unlike Drop.
        INSTALL_HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                previous(info);
            }));
        });
        let session = Self;
        #[cfg(windows)]
        save_console_modes()?;
        if !terminal::is_raw_mode_enabled()? {
            RAW_CHANGED.store(true, Ordering::SeqCst);
            terminal::enable_raw_mode()?;
        }
        ALTERNATE.store(true, Ordering::SeqCst);
        execute!(io::stdout(), EnterAlternateScreen)?;
        CURSOR_HIDDEN.store(true, Ordering::SeqCst);
        execute!(io::stdout(), Hide)?;
        Ok(session)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let cursor = CURSOR_HIDDEN.swap(false, Ordering::SeqCst);
    let alternate = ALTERNATE.swap(false, Ordering::SeqCst);
    if cursor || alternate {
        let mut out = io::stdout();
        if cursor {
            let _ = execute!(out, Show);
        }
        if alternate {
            let _ = execute!(out, LeaveAlternateScreen);
        }
        let _ = out.flush();
    }
    if RAW_CHANGED.swap(false, Ordering::SeqCst) {
        let _ = terminal::disable_raw_mode();
    }
    #[cfg(windows)]
    restore_console_modes();
}

#[cfg(windows)]
fn save_console_modes() -> io::Result<()> {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    let mut input = 0;
    let mut output = 0;
    // The handles belong to the process and remain owned by Windows.
    unsafe {
        if GetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), &mut input) == 0
            || GetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), &mut output) == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    INPUT_MODE.store(input, Ordering::SeqCst);
    OUTPUT_MODE.store(output, Ordering::SeqCst);
    SAVED_MODES.store(true, Ordering::SeqCst);
    // Crossterm caches ANSI support, so enable it again for later sessions.
    unsafe {
        SetConsoleMode(
            GetStdHandle(STD_OUTPUT_HANDLE),
            output | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        );
    }
    Ok(())
}

#[cfg(windows)]
fn restore_console_modes() {
    use windows_sys::Win32::System::Console::{
        GetStdHandle, SetConsoleMode, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    if SAVED_MODES.swap(false, Ordering::SeqCst) {
        // Restore exact modes after crossterm has finished its own cleanup.
        unsafe {
            SetConsoleMode(
                GetStdHandle(STD_INPUT_HANDLE),
                INPUT_MODE.load(Ordering::SeqCst),
            );
            SetConsoleMode(
                GetStdHandle(STD_OUTPUT_HANDLE),
                OUTPUT_MODE.load(Ordering::SeqCst),
            );
        }
    }
}
