use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::Print,
    terminal::{Clear, ClearType},
};

use crate::app::Screen;

pub struct Renderer {
    hashes: Vec<Option<u64>>,
    width: u16,
    line: String,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            hashes: Vec::new(),
            width: 0,
            line: String::new(),
        }
    }

    pub fn invalidate(&mut self) {
        self.hashes.fill(None);
    }

    pub fn draw(
        &mut self,
        writer: &mut impl Write,
        screen: Screen,
        width: u16,
        height: u16,
        status: &str,
    ) -> io::Result<()> {
        if self.width != width || self.hashes.len() != usize::from(height) {
            self.width = width;
            self.hashes.clear();
            self.hashes.resize(usize::from(height), None);
        }
        let result = (|| {
            for row in 0..height {
                make_line(&mut self.line, row, height, width, screen, status);
                let hash = line_hash(&self.line, width);
                if self.hashes[usize::from(row)] != Some(hash) {
                    queue!(
                        writer,
                        MoveTo(0, row),
                        Clear(ClearType::CurrentLine),
                        Print(&self.line)
                    )?;
                }
            }
            writer.flush()
        })();
        if result.is_err() {
            self.hashes.fill(None);
            return result;
        }
        // Recompute after flush: never retain a second frame or commit a partial write.
        for row in 0..height {
            make_line(&mut self.line, row, height, width, screen, status);
            self.hashes[usize::from(row)] = Some(line_hash(&self.line, width));
        }
        Ok(())
    }
}

fn line_hash(line: &str, width: u16) -> u64 {
    let mut hash = DefaultHasher::new();
    line.hash(&mut hash);
    width.hash(&mut hash);
    hash.finish()
}

fn make_line(line: &mut String, row: u16, height: u16, width: u16, screen: Screen, status: &str) {
    line.clear();
    if row == 0 {
        let header = match screen {
            Screen::Review => "[1 Review]  2 Edit  3 Models  q Quit",
            Screen::Edit => "1 Review  [2 Edit]  3 Models  q Quit",
            Screen::Models => "1 Review  2 Edit  [3 Models]  q Quit",
        };
        line.push_str(header);
    } else if row == 2 {
        line.push_str(screen.title());
        line.push_str(" - Coming soon");
    } else if height > 3 && row == height - 1 {
        // Avoid control sequences and multi-column characters in terminal output.
        line.extend(status.chars().map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                ' '
            }
        }));
    }
    line.truncate(line.len().min(usize::from(width.saturating_sub(1))));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_frame_writes_nothing_and_resize_redraws() {
        let mut renderer = Renderer::new();
        let mut out = Vec::new();
        renderer
            .draw(&mut out, Screen::Review, 40, 4, "ready")
            .unwrap();
        assert!(!out.is_empty());
        out.clear();
        renderer
            .draw(&mut out, Screen::Review, 40, 4, "ready")
            .unwrap();
        assert!(out.is_empty());
        renderer
            .draw(&mut out, Screen::Review, 10, 4, "ready")
            .unwrap();
        assert!(!out.is_empty());
    }
    #[test]
    fn changing_screen_clears_old_lines_even_when_narrow() {
        let mut renderer = Renderer::new();
        let mut out = Vec::new();
        renderer.draw(&mut out, Screen::Review, 12, 4, "").unwrap();
        out.clear();
        renderer.draw(&mut out, Screen::Edit, 12, 4, "").unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("\x1b[2K"));
        assert!(output.contains("Edit"));
    }
    struct FailFlush;
    impl Write for FailFlush {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }
    #[test]
    fn failed_flush_forces_full_redraw() {
        let mut renderer = Renderer::new();
        assert!(renderer
            .draw(&mut FailFlush, Screen::Review, 40, 4, "")
            .is_err());
        let mut out = Vec::new();
        renderer.draw(&mut out, Screen::Review, 40, 4, "").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().matches("\x1b[2K").count(),
            4
        );
    }
}
