mod app;
mod diff;
mod git;
mod memory;
mod proc;
mod review;
mod ui;
mod watch;

use app::workbench::Workbench;
use app::{event::Input, App};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::{
    io,
    time::{Duration, Instant},
};
use ui::{Renderer, TerminalSession};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dark: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == "--watch-bridge") {
        if arguments.len() != 2 {
            return Err(io::Error::other("Usage: dark --watch-bridge ROOT"));
        }
        return run_watch_bridge(std::path::Path::new(&arguments[1]));
    }
    let mut debug = false;
    let mut self_test = false;
    let mut panic_test = false;
    let mut review_test = false;
    for arg in arguments {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "dark {}\n\nUsage: dark [OPTIONS]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show version\n      --debug      Show process memory\n      --self-test  Check all seven tabs without a terminal\n      --self-test-review  Load the current Git repository and measure review memory\n      --self-test-panic  Check terminal cleanup on panic\n\nKeys (all require Alt): Alt+1 Review, Alt+2 Edit, Alt+3 Vibe, Alt+4 Git commit,\n      Alt+5 Models, Alt+6 View, Alt+7 Help\n      Alt+l/r panes, Alt+f focus, Alt+n/N hunks, Alt+a accept, Alt+x reject\n      Alt+u restore session, Alt+e editor, Alt+v split/unified\n      Alt+-/Alt+= resize file list, Alt+0 reset width; drag the separator with the left mouse button\n      Alt+arrows or Alt+j/k navigate, Alt+Enter open, Alt+y/n confirm/cancel\n      Alt+q or Alt+Ctrl+c quit; bare keys do not execute commands\n\nEnvironment: DARK_SNAPSHOT_KEEP (default 10), DARK_EDITOR / VISUAL / EDITOR,\n             DARK_TAB_WIDTH (default 8), DARK_WATCH_HELPER, NO_COLOR",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            Some("--version" | "-V") => {
                println!("dark {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--debug") => debug = true,
            Some("--self-test") => self_test = true,
            Some("--self-test-review") => review_test = true,
            Some("--self-test-panic") => panic_test = true,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {}. Try --help.", arg.to_string_lossy()),
                ))
            }
        }
    }
    if review_test {
        return run_review_self_test();
    }
    if self_test {
        return run_self_test();
    }
    run_terminal(debug, panic_test)
}

fn memory_status() -> String {
    match memory::current_bytes() {
        Ok(bytes) => format!("Memory ({}): {} bytes", memory::METRIC, bytes),
        Err(error) => format!("Memory: n/a ({error})"),
    }
}

fn run_terminal(debug: bool, panic_test: bool) -> io::Result<()> {
    let mut _session = TerminalSession::enter()?;
    let mut app = App::new();
    let mut renderer = Renderer::new();
    let mut output = io::stdout();
    let (width, height) = terminal::size()?;
    renderer.draw_view(
        &mut output,
        &ui::View {
            message: "Đang tải repository và tạo snapshot…",
            show_files: true,
            show_diff: true,
            ..ui::View::default()
        },
        width,
        height,
    )?;
    let mut workbench = Workbench::open(&std::env::current_dir()?);
    let mut watcher = open_watcher(&mut workbench);
    let mut changes = PendingChanges::default();
    let mut status = if debug {
        memory_status()
    } else {
        String::new()
    };
    crossterm::execute!(
        output,
        crossterm::terminal::SetTitle(workbench.title(app.screen.title()))
    )?;
    workbench.draw(
        &mut renderer,
        &mut output,
        app.screen.index(),
        width,
        height,
        &status,
    )?;
    if panic_test {
        std::thread::sleep(Duration::from_millis(500));
        panic!("intentional terminal cleanup test");
    }
    let mut next_sample = Instant::now() + Duration::from_secs(1);
    while app.running {
        let mut timeout = watcher.as_ref().map(|_| Duration::from_millis(75));
        if debug {
            let until_sample = next_sample.saturating_duration_since(Instant::now());
            timeout = Some(timeout.map_or(until_sample, |wait| wait.min(until_sample)));
        }
        let mut dirty = false;
        if let Some(event) = Input::next(timeout)? {
            if matches!(event, Event::Resize(_, _)) {
                renderer.invalidate();
                workbench.cancel_drag();
            }
            if let Event::Key(key) = event {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    if workbench.modal() {
                        if app::is_quit(key) {
                            app.running = false;
                        } else {
                            workbench.key(key);
                        }
                    } else {
                        let previous = app.screen;
                        app.handle_event(event);
                        if previous != app.screen {
                            workbench.cancel_drag();
                        }
                        if app.screen == app::Screen::Review && previous != app.screen {
                            workbench.refresh();
                        }
                        if app.screen == app::Screen::Review
                            && key.modifiers.contains(KeyModifiers::ALT)
                            && workbench.key(key)
                        {
                            drop(_session);
                            let result = open_editor(&workbench);
                            _session = TerminalSession::enter()?;
                            renderer.invalidate();
                            workbench.refresh();
                            if let Err(error) = result {
                                workbench.message = format!("Editor: {error}");
                            }
                        }
                    }
                    dirty = true;
                }
            } else if let Event::Mouse(mouse) = event {
                if app.screen == app::Screen::Review {
                    let (width, height) = terminal::size()?;
                    dirty = workbench.mouse(mouse, width, height);
                } else {
                    workbench.cancel_drag();
                }
            } else {
                dirty = app.handle_event(event);
            }
        }
        if !app.running {
            break;
        }
        if let Some(active) = &mut watcher {
            match active.take_changed() {
                Ok(true) => changes.note(Instant::now()),
                Ok(false) => {}
                Err(error) => {
                    workbench.message =
                        format!("Tự cập nhật đã dừng: {error}. Đổi tab để tải lại.");
                    watcher = None;
                    dirty = true;
                }
            }
        }
        if changes.ready(Instant::now()) && !workbench.modal() {
            workbench.refresh_external();
            changes.clear();
            dirty |= app.screen == app::Screen::Review;
        }
        if debug && Instant::now() >= next_sample {
            status = memory_status();
            next_sample = Instant::now() + Duration::from_secs(1);
            dirty = true;
        }
        if dirty && app.running {
            crossterm::execute!(
                output,
                crossterm::terminal::SetTitle(workbench.title(app.screen.title()))
            )?;
            let (width, height) = terminal::size()?;
            workbench.draw(
                &mut renderer,
                &mut output,
                app.screen.index(),
                width,
                height,
                &status,
            )?;
        }
    }
    Ok(())
}

fn open_watcher(workbench: &mut Workbench) -> Option<watch::Watcher> {
    let root = &workbench.review.as_ref()?.repo.root;
    match watch::Watcher::new(root) {
        Ok(watcher) => {
            // Close the gap between the initial diff load and watch registration.
            workbench.refresh_external();
            Some(watcher)
        }
        Err(error) => {
            workbench.message = format!("Chưa bật tự cập nhật: {error}. Đổi tab để tải lại.");
            None
        }
    }
}

/// Only notification deadlines are polled; no file or Git scan runs while idle.
#[derive(Default)]
struct PendingChanges {
    first: Option<Instant>,
    last: Option<Instant>,
}

impl PendingChanges {
    fn note(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = Some(now);
    }

    fn ready(&self, now: Instant) -> bool {
        self.last
            .is_some_and(|last| now.duration_since(last) >= Duration::from_millis(150))
            || self
                .first
                .is_some_and(|first| now.duration_since(first) >= Duration::from_millis(500))
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(windows)]
fn run_watch_bridge(root: &std::path::Path) -> io::Result<()> {
    use std::io::Write;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn PeekNamedPipe(
            pipe: *mut std::ffi::c_void,
            buffer: *mut std::ffi::c_void,
            size: u32,
            read: *mut u32,
            available: *mut u32,
            left: *mut u32,
        ) -> i32;
    }
    let mut watcher = watch::Watcher::new(root)?;
    let mut output = io::stdout().lock();
    writeln!(output, "READY")?;
    output.flush()?;
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    loop {
        // A closed stdin pipe means the WSL owner exited, including abrupt exits.
        let alive = unsafe {
            PeekNamedPipe(
                input,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if alive == 0 {
            return Ok(());
        }
        if watcher.take_changed()? {
            writeln!(output, "CHANGED")?;
            output.flush()?;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(windows))]
fn run_watch_bridge(_root: &std::path::Path) -> io::Result<()> {
    Err(io::Error::other(
        "The notification bridge requires Windows dark.exe",
    ))
}

fn run_self_test() -> io::Result<()> {
    let mut app = App::new();
    let mut renderer = Renderer::new();
    let mut sink = io::sink();
    renderer.draw(&mut sink, app.screen, 80, 24, "")?;
    let mut peak = sample_or_report()?;
    for digit in ['2', '3', '4', '5', '6', '7', '1'] {
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char(digit),
            KeyModifiers::ALT,
        )));
        renderer.draw(&mut sink, app.screen, 80, 24, "")?;
        peak = peak.max(sample_or_report()?);
    }
    for (width, height) in [(20, 5), (120, 40), (80, 24)] {
        app.handle_event(Event::Resize(width, height));
        renderer.invalidate();
        renderer.draw(&mut sink, app.screen, width, height, "")?;
        peak = peak.max(sample_or_report()?);
    }
    let mut idle = 0;
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(100));
        let bytes = sample_or_report()?;
        idle = idle.max(bytes);
        peak = peak.max(bytes);
    }
    println!(
        "memory_metric={}\nidle_bytes={idle}\npeak_sampled_bytes={peak}\nscreens=7",
        memory::METRIC
    );
    Ok(())
}

fn sample_or_report() -> io::Result<u64> {
    memory::current_bytes().inspect_err(|error| {
        println!("memory_metric=unavailable\nreason={error}\nscreens=7");
    })
}

fn run_review_self_test() -> io::Result<()> {
    let mut workbench = Workbench::open(&std::env::current_dir()?);
    if workbench.review.is_none() || workbench.modal() {
        return Err(io::Error::other(workbench.message.clone()));
    }
    // Include the active notification backend in the release memory acceptance test.
    let _watcher = open_watcher(&mut workbench);
    #[cfg(any(windows, target_os = "linux"))]
    if _watcher.is_none() {
        return Err(io::Error::other(workbench.message.clone()));
    }
    let mut renderer = Renderer::new();
    let mut output = io::sink();
    let mut peak = sample_or_report()?;
    let files = workbench
        .review
        .as_ref()
        .map(|r| r.files.len())
        .unwrap_or(0);
    for _ in 0..files.max(1) {
        for (width, height) in [(160, 44), (120, 44), (100, 30), (80, 24)] {
            workbench.draw(&mut renderer, &mut output, 0, width, height, "")?;
            peak = peak.max(sample_or_report()?);
        }
        workbench.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::ALT));
    }
    // Exercise both resized layouts and the drag path in the same RAM fixture.
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for (width, height) in [(160, 44), (80, 24)] {
        workbench.draw(&mut renderer, &mut output, 0, width, height, "")?;
        for code in ['-', '=', '=', '0'] {
            workbench.key(KeyEvent::new(KeyCode::Char(code), KeyModifiers::ALT));
            workbench.draw(&mut renderer, &mut output, 0, width, height, "")?;
            peak = peak.max(sample_or_report()?);
        }
        let separator = ui::Layout::new(width, height, true, true, false).files as u16 - 1;
        for (kind, column) in [
            (MouseEventKind::Down(MouseButton::Left), separator),
            (MouseEventKind::Drag(MouseButton::Left), width - 2),
            (MouseEventKind::Up(MouseButton::Left), width - 2),
        ] {
            workbench.mouse(
                MouseEvent {
                    kind,
                    column,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                },
                width,
                height,
            );
        }
        workbench.draw(&mut renderer, &mut output, 0, width, height, "")?;
        peak = peak.max(sample_or_report()?);
        workbench.key(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::ALT));
    }
    let mut idle = 0;
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(100));
        idle = idle.max(sample_or_report()?);
        peak = peak.max(idle);
    }
    println!("memory_metric={}\nidle_bytes={idle}\npeak_sampled_bytes={peak}\nscreens=7\nreview_files={files}",memory::METRIC);
    Ok(())
}

fn open_editor(workbench: &Workbench) -> io::Result<()> {
    let review = workbench
        .review
        .as_ref()
        .ok_or_else(|| io::Error::other("No repository"))?;
    let path = workbench
        .selected_path()
        .ok_or_else(|| io::Error::other("No selected file"))?;
    let full = review.repo.safe_path(&path)?;
    let command = std::env::var("DARK_EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "code --wait --goto {file}:{line}".into());
    let mut arguments = editor_arguments(&command)?;
    let program = arguments.remove(0);
    let had_file = arguments.iter().any(|a| a.contains("{file}"));
    for argument in &mut arguments {
        *argument = argument
            .replace("{file}", &full.to_string_lossy())
            .replace("{line}", &workbench.selected_line().to_string());
    }
    if !had_file {
        arguments.push(full.to_string_lossy().into_owned());
    }
    let result = std::process::Command::new(program)
        .args(arguments)
        .current_dir(&review.repo.root)
        .status()?;
    if result.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("Editor exited with {result}")))
    }
}

fn editor_arguments(command: &str) -> io::Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in command.chars() {
        if Some(ch) == quote {
            quote = None;
        } else if quote.is_none() && (ch == '\'' || ch == '"') {
            quote = Some(ch);
        } else if quote.is_none() && ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() {
        return Err(io::Error::other("Unclosed quote in editor command"));
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        return Err(io::Error::other("Editor command is empty"));
    }
    Ok(args)
}

#[cfg(test)]
mod editor_tests {
    use super::*;
    #[test]
    fn editor_arguments_preserve_quoted_program_and_paths_without_a_shell() {
        assert_eq!(
            editor_arguments("\"C:\\Program Files\\Editor\\editor.exe\" --goto '{file}:{line}'")
                .unwrap(),
            vec![
                "C:\\Program Files\\Editor\\editor.exe",
                "--goto",
                "{file}:{line}"
            ]
        );
        assert!(editor_arguments("\"unfinished").is_err());
        assert!(editor_arguments(" ").is_err());
    }

    #[test]
    fn notifications_coalesce_without_starving_a_continuous_writer() {
        let start = Instant::now();
        let mut changes = PendingChanges::default();
        assert!(!changes.ready(start));
        changes.note(start);
        changes.note(start + Duration::from_millis(100));
        assert!(!changes.ready(start + Duration::from_millis(200)));
        assert!(changes.ready(start + Duration::from_millis(250)));
        changes.note(start + Duration::from_millis(490));
        assert!(changes.ready(start + Duration::from_millis(500)));
        changes.clear();
        assert!(!changes.ready(start + Duration::from_secs(1)));
    }
}
