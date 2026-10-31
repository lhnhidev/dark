mod app;
mod ui;

use app::{event::Input, App};
use crossterm::event::Event;
use crossterm::terminal;
use std::io;
use ui::{Renderer, TerminalSession};

fn main() -> io::Result<()> {
    let _session = TerminalSession::enter()?;
    let mut app = App::new();
    let mut renderer = Renderer::new();
    let mut output = io::stdout();
    let (width, height) = terminal::size()?;
    renderer.draw(&mut output, app.screen, width, height, "")?;
    while app.running {
        if let Some(event) = Input::next(None)? {
            if matches!(event, Event::Resize(_, _)) {
                renderer.invalidate();
            }
            if app.handle_event(event) && app.running {
                let (width, height) = terminal::size()?;
                renderer.draw(&mut output, app.screen, width, height, "")?;
            }
        }
    }
    Ok(())
}
