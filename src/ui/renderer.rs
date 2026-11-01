use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::Print,
    terminal::{Clear, ClearType},
};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
    let available = usize::from(width.saturating_sub(1));
    if row == 0 {
        let header = match screen {
            Screen::Review => "[1 Review]  2 Edit  3 Models  q Quit",
            Screen::Edit => "1 Review  [2 Edit]  3 Models  q Quit",
            Screen::Models => "1 Review  2 Edit  [3 Models]  q Quit",
        };
        clip_into(line, header, available);
    } else if row == 2 {
        let used = clip_into(line, screen.title(), available);
        clip_into(line, " - Coming soon", available.saturating_sub(used));
    } else if height > 3 && row == height - 1 {
        clip_into(line, status, available);
    }
}

fn clip_into(line: &mut String, input: &str, available: usize) -> usize {
    let mut columns = 0;
    let initial_columns = UnicodeWidthStr::width(line.as_str());
    for grapheme in input.graphemes(true) {
        if columns >= available {
            break;
        }
        if grapheme == "\t" {
            let spaces = (8 - (initial_columns + columns) % 8).min(available - columns);
            for _ in 0..spaces {
                line.push(' ');
            }
            columns += spaces;
        } else if grapheme.chars().any(char::is_control) {
            line.push(' ');
            columns += 1;
        } else {
            let width = UnicodeWidthStr::width(grapheme);
            if width > available - columns {
                break;
            }
            line.push_str(grapheme);
            columns += width;
        }
    }
    columns
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
    #[test]
    fn status_preserves_complete_unicode_clusters() {
        let mut line = String::new();
        let text = "Vi\u{1ec7}t e\u{301} \u{4e2d}\u{6587} \u{1f44d}\u{1f3fd} \u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466} \u{1f1fb}\u{1f1f3}";
        make_line(&mut line, 3, 4, 80, Screen::Review, text);
        assert_eq!(line, text);
        for width in 0..50 {
            make_line(&mut line, 3, 4, width, Screen::Review, text);
            assert!(UnicodeWidthStr::width(line.as_str()) <= usize::from(width.saturating_sub(1)));
            assert!(text.starts_with(&line));
            assert!(
                line.is_empty()
                    || text
                        .graphemes(true)
                        .scan(0, |n, g| {
                            *n += g.len();
                            Some(*n)
                        })
                        .any(|end| end == line.len())
            );
        }
    }

    #[test]
    fn wide_clusters_require_two_columns() {
        let cases = [
            "\u{754c}",
            "\u{1f600}",
            "\u{1f44d}\u{1f3fd}",
            "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
            "\u{1f1fb}\u{1f1f3}",
            "\u{2764}\u{fe0f}",
        ];
        let mut line = String::new();
        for text in cases {
            make_line(&mut line, 3, 4, 2, Screen::Review, text);
            assert!(line.is_empty());
            make_line(&mut line, 3, 4, 3, Screen::Review, text);
            assert_eq!(line, text);
        }
    }

    #[test]
    fn tabs_expand_to_next_stop_and_controls_are_safe() {
        let mut line = String::new();
        make_line(&mut line, 3, 4, 20, Screen::Review, "1234567\tX\x1b\nY");
        assert_eq!(line, "1234567 X  Y");
        make_line(&mut line, 3, 4, 9, Screen::Review, "123456\tX");
        assert_eq!(line, "123456  ");
        for width in 0..=2 {
            make_line(&mut line, 3, 4, width, Screen::Review, "\u{754c}a");
            assert!(line.is_empty());
        }
    }

    #[test]
    fn long_combining_cluster_remains_complete() {
        let mut line = String::new();
        let input = format!("a{}", "\u{301}".repeat(10000));
        make_line(&mut line, 3, 4, 2, Screen::Review, &input);
        assert_eq!(line, input);
    }

    #[test]
    fn tab_stops_follow_existing_unicode_columns() {
        let mut line = String::new();
        clip_into(&mut line, "\u{754c}\u{e9}e\u{301}", 20);
        clip_into(&mut line, "\tX", 16);
        assert_eq!(line, "\u{754c}\u{e9}e\u{301}    X");
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
