mod app;
mod diff;
mod git;
mod memory;
mod proc;
mod review;
mod ui;

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
    let mut debug = false;
    let mut self_test = false;
    let mut panic_test = false;
    let mut review_test = false;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "dark {}\n\nUsage: dark [OPTIONS]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show version\n      --debug      Show process memory\n      --self-test  Check all seven tabs without a terminal\n      --self-test-review  Load the current Git repository and measure review memory\n      --self-test-panic  Check terminal cleanup on panic\n\nKeys: 1 Review, 2 Edit, 3 Vibe, 4 Git commit, 5 Models, 6 View, 7 Help\n      l/r panes, Tab focus, n/N hunks, a accept, x reject, u restore session, e editor\n      q or Ctrl-C quit\n\nEnvironment: DARK_SNAPSHOT_KEEP (default 10), DARK_EDITOR / VISUAL / EDITOR,\n             DARK_TAB_WIDTH (default 8), NO_COLOR",
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
        let timeout = debug.then(|| next_sample.saturating_duration_since(Instant::now()));
        let mut dirty = false;
        if let Some(event) = Input::next(timeout)? {
            if matches!(event, Event::Resize(_, _)) {
                renderer.invalidate();
            }
            if let Event::Key(key) = event {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    if workbench.modal() {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            app.running = false;
                        } else {
                            workbench.key(key);
                        }
                    } else {
                        let previous = app.screen;
                        app.handle_event(event);
                        if app.screen == app::Screen::Review && previous != app.screen {
                            workbench.refresh();
                        }
                        if app.screen == app::Screen::Review
                            && key
                                .modifiers
                                .intersection(KeyModifiers::CONTROL | KeyModifiers::ALT)
                                .is_empty()
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
            } else {
                dirty = app.handle_event(event);
            }
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

fn run_self_test() -> io::Result<()> {
    let mut app = App::new();
    let mut renderer = Renderer::new();
    let mut sink = io::sink();
    renderer.draw(&mut sink, app.screen, 80, 24, "")?;
    let mut peak = sample_or_report()?;
    for digit in ['2', '3', '4', '5', '6', '7', '1'] {
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char(digit),
            KeyModifiers::NONE,
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
        workbench.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
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
}
