mod app;

use app::{event::Input, App};
use std::io;

fn main() -> io::Result<()> {
    let mut app = App::new();
    println!("dark | 1 Review | 2 Edit | 3 Models | q Quit");
    println!("{}", app.screen.title());
    while app.running {
        if let Some(event) = Input::next(None)? {
            if app.handle_event(event) && app.running {
                println!("{}", app.screen.title());
            }
        }
    }
    Ok(())
}
