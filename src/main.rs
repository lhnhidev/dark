mod app;
mod memory;
mod ui;

use app::{event::Input, App};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::{
    io,
    time::{Duration, Instant},
};
use ui::{Renderer, TerminalSession};

fn main() -> io::Result<()> {
    let mut debug = false;
    let mut self_test = false;
    let mut panic_test = false;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "dark {}\n\nUsage: dark [OPTIONS]\n\nOptions:\n  -h, --help       Show this help\n  -V, --version    Show version\n      --debug      Show process memory\n      --self-test  Check navigation and memory without a terminal\n      --self-test-panic  Check terminal cleanup on panic\n\nKeys: 1 Review, 2 Edit, 3 Models, q or Ctrl-C quit",
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
            Some("--self-test-panic") => panic_test = true,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {}", arg.to_string_lossy()),
                ))
            }
        }
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
    let _session = TerminalSession::enter()?;
    let mut app = App::new();
    let mut renderer = Renderer::new();
    let mut output = io::stdout();
    let mut status = if debug {
        memory_status()
    } else {
        String::new()
    };
    let (width, height) = terminal::size()?;
    renderer.draw(&mut output, app.screen, width, height, &status)?;
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
            dirty = app.handle_event(event);
        }
        if debug && Instant::now() >= next_sample {
            status = memory_status();
            next_sample = Instant::now() + Duration::from_secs(1);
            dirty = true;
        }
        if dirty && app.running {
            let (width, height) = terminal::size()?;
            renderer.draw(&mut output, app.screen, width, height, &status)?;
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
    for digit in ['2', '3', '1'] {
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
        "memory_metric={}\nidle_bytes={idle}\npeak_sampled_bytes={peak}\nscreens=3",
        memory::METRIC
    );
    Ok(())
}

fn sample_or_report() -> io::Result<u64> {
    memory::current_bytes().inspect_err(|error| {
        println!("memory_metric=unavailable\nreason={error}\nscreens=3");
    })
}
