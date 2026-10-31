use std::{io, time::Duration};

use crossterm::event::{self, Event};

/// The sole terminal event reader; no background thread or event queue.
pub struct Input;

impl Input {
    /// Waits indefinitely when idle, or up to a future source's next deadline.
    pub fn next(timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if let Some(timeout) = timeout {
            if !event::poll(timeout)? {
                return Ok(None);
            }
        }
        event::read().map(Some)
    }
}
