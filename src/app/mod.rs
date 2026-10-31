pub mod event;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Review,
    Edit,
    Models,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Review => "Review",
            Self::Edit => "Edit",
            Self::Models => "Models",
        }
    }
}

pub struct App {
    pub screen: Screen,
    pub running: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Review,
            running: true,
        }
    }

    /// Returns whether the visible state needs to be drawn again.
    pub fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::Resize(_, _) => true,
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.running = false;
                    return false;
                }
                if !key.modifiers.is_empty() {
                    return false;
                }
                let screen = match key.code {
                    KeyCode::Char('1') => Screen::Review,
                    KeyCode::Char('2') => Screen::Edit,
                    KeyCode::Char('3') => Screen::Models,
                    KeyCode::Char('q') => {
                        self.running = false;
                        return false;
                    }
                    _ => return false,
                };
                let changed = self.screen != screen;
                self.screen = screen;
                changed
            }
            _ => false,
        }
    }
}
