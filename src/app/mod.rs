pub mod event;
pub mod workbench;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Review,
    Edit,
    Vibe,
    Git,
    Models,
    View,
    Help,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Review => "Review",
            Self::Edit => "Edit",
            Self::Vibe => "Vibe",
            Self::Git => "Git commit",
            Self::Models => "Models",
            Self::View => "View",
            Self::Help => "Help",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Review => 0,
            Self::Edit => 1,
            Self::Vibe => 2,
            Self::Git => 3,
            Self::Models => 4,
            Self::View => 5,
            Self::Help => 6,
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
                if !key.modifiers.is_empty() && key.modifiers != KeyModifiers::ALT {
                    return false;
                }
                if key.modifiers == KeyModifiers::ALT
                    && !matches!(key.code, KeyCode::Char('1'..='7'))
                {
                    return false;
                }
                let screen = match key.code {
                    KeyCode::Char('1') => Screen::Review,
                    KeyCode::Char('2') => Screen::Edit,
                    KeyCode::Char('3') => Screen::Vibe,
                    KeyCode::Char('4') => Screen::Git,
                    KeyCode::Char('5') => Screen::Models,
                    KeyCode::Char('6') => Screen::View,
                    KeyCode::Char('7') => Screen::Help,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent::new_with_kind(code, modifiers, kind))
    }

    #[test]
    fn switches_screens_and_redraws_only_on_change() {
        let mut app = App::new();
        assert_eq!(app.screen, Screen::Review);
        for (digit, screen) in [
            ('2', Screen::Edit),
            ('3', Screen::Vibe),
            ('4', Screen::Git),
            ('5', Screen::Models),
            ('6', Screen::View),
            ('7', Screen::Help),
            ('1', Screen::Review),
        ] {
            assert!(app.handle_event(key(
                KeyCode::Char(digit),
                KeyModifiers::NONE,
                KeyEventKind::Press
            )));
            assert_eq!(app.screen, screen);
            assert!(!app.handle_event(key(
                KeyCode::Char(digit),
                KeyModifiers::NONE,
                KeyEventKind::Press
            )));
        }
        assert!(app.handle_event(Event::Resize(40, 10)));
        assert!(app.running);
    }

    #[test]
    fn ignores_repeats_releases_and_modified_shortcuts() {
        let mut app = App::new();
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            for code in [KeyCode::Char('2'), KeyCode::Char('q')] {
                assert!(!app.handle_event(key(code, KeyModifiers::NONE, kind)));
            }
        }
        assert!(!app.handle_event(key(
            KeyCode::Char('2'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press
        )));
        assert_eq!(app.screen, Screen::Review);
        assert!(app.running);
    }

    #[test]
    fn quit_keys_stop_without_redrawing() {
        for (code, modifiers) in [
            (KeyCode::Char('q'), KeyModifiers::NONE),
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            let mut app = App::new();
            assert!(!app.handle_event(key(code, modifiers, KeyEventKind::Press)));
            assert!(!app.running);
        }
    }
}
