#![allow(dead_code, unused_imports)]
#[path = "../src/app/mod.rs"]
mod app;
#[path = "../src/diff.rs"]
mod diff;
#[path = "../src/git.rs"]
mod git;
#[path = "../src/proc.rs"]
mod proc;
#[path = "../src/review.rs"]
mod review;
#[path = "../src/ui/mod.rs"]
mod ui;

use app::workbench::Workbench;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "dark-workbench-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let fixture = Self(path);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.name", "Test"]);
        fixture.git(&["config", "user.email", "test@example.invalid"]);
        fixture.git(&["config", "core.autocrlf", "false"]);
        for (name, contents) in files {
            fs::write(fixture.0.join(name), contents).unwrap();
        }
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "-qm", "initial"]);
        fixture
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write(&self, path: &str, contents: &str) {
        fs::write(self.0.join(path), contents).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn key(workbench: &mut Workbench, code: KeyCode) {
    workbench.key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn render(workbench: &mut Workbench) -> String {
    let mut bytes = Vec::new();
    workbench
        .draw(&mut ui::Renderer::new(), &mut bytes, 0, 160, 44, "")
        .unwrap();
    String::from_utf8(bytes).unwrap()
}

fn plain(ansi: &str) -> String {
    let mut chars = ansi.chars();
    let mut result = String::new();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn accepted_file_remains_counted_after_moving_to_another_file() {
    let f = Fixture::new(&[("a.txt", "old a\n"), ("b.txt", "old b\n")]);
    f.write("a.txt", "new a\n");
    f.write("b.txt", "new b\n");
    let mut w = Workbench::open(&f.0);
    assert_eq!(w.selected_path().unwrap(), PathBuf::from("a.txt"));
    key(&mut w, KeyCode::Char('a'));
    assert_eq!(w.selected_path().unwrap(), PathBuf::from("b.txt"));
    let screen = plain(&render(&mut w));
    assert!(screen.contains("1/2"), "{screen}");
    assert!(screen.contains('✓'), "{screen}");
}

#[test]
fn identical_hunks_must_each_be_accepted() {
    let mut original = String::from("same old\n");
    for i in 0..20 {
        original.push_str(&format!("context {i}\n"));
    }
    original.push_str("same old\n");
    let f = Fixture::new(&[("a.txt", &original)]);
    f.write("a.txt", &original.replace("same old", "same new"));
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Char('a'));
    let screen = plain(&render(&mut w));
    assert!(screen.contains("0/1"), "{screen}");
    key(&mut w, KeyCode::Char('a'));
    let screen = plain(&render(&mut w));
    assert!(screen.contains("1/1"), "{screen}");
}

#[test]
fn undo_confirmation_does_not_overwrite_edits_made_after_modal_opens() {
    let f = Fixture::new(&[("a.txt", "old a\n"), ("b.txt", "old b\n")]);
    f.write("a.txt", "startup a\n");
    let mut w = Workbench::open(&f.0);
    f.write("a.txt", "later a\n");
    key(&mut w, KeyCode::Char('u'));
    assert!(w.modal());
    f.write("b.txt", "later b\n");
    key(&mut w, KeyCode::Char('y'));
    assert_eq!(fs::read_to_string(f.0.join("a.txt")).unwrap(), "later a\n");
    assert_eq!(fs::read_to_string(f.0.join("b.txt")).unwrap(), "later b\n");
    assert!(!w.message.is_empty());
}

#[test]
fn unavailable_review_actions_explain_what_happened() {
    let f = Fixture::new(&[("a.txt", "old\n")]);
    f.write("a.txt", "new\n");
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Tab);
    for code in [KeyCode::Enter, KeyCode::Char('?'), KeyCode::Char('/')] {
        key(&mut w, code);
        assert!(!w.message.is_empty(), "no explanation for {code:?}");
    }
}

#[test]
fn unified_diff_renders_word_highlights_when_terminal_has_color() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let f = Fixture::new(&[("a.txt", "let old = 1;\n")]);
    f.write("a.txt", "let new = 1;\n");
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Char('v'));
    let screen = render(&mut w);
    let truecolor = std::env::var("COLORTERM")
        .is_ok_and(|s| s.eq_ignore_ascii_case("truecolor") || s.eq_ignore_ascii_case("24bit"));
    let expected = if truecolor {
        "\x1b[48;2;22;57;35m".to_owned()
    } else if let crossterm::style::Color::AnsiValue(n) =
        ui::theme::color(ui::theme::ADDED_WORD_BG, false)
    {
        format!("\x1b[48;5;{n}m")
    } else {
        unreachable!()
    };
    assert!(
        screen.contains(&expected),
        "unified diff did not render the added-word background"
    );
}

fn screen_rows(workbench: &mut Workbench, width: u16, height: u16) -> Vec<String> {
    let mut bytes = Vec::new();
    workbench
        .draw(&mut ui::Renderer::new(), &mut bytes, 0, width, height, "")
        .unwrap();
    let text = plain(&String::from_utf8(bytes).unwrap());
    let cells: Vec<char> = text.chars().collect();
    cells
        .chunks(usize::from(width) - 1)
        .map(|row| row.iter().collect())
        .collect()
}

fn assert_selected_visible(workbench: &mut Workbench, width: u16, height: u16, context: &str) {
    let name = workbench
        .selected_path()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let rows = screen_rows(workbench, width, height);
    assert_eq!(rows.len(), usize::from(height), "{context}");
    let separator = 2 + ui::layout::file_list_rows(height);
    // The cursor row is either the selected file or, while the cursor rests on
    // a group header, that group; either way it must be inside the list area.
    let marked: Vec<usize> = (0..rows.len())
        .filter(|&r| rows[r].chars().nth(1) == Some('▌'))
        .collect();
    assert!(
        !marked.is_empty(),
        "{context}: no cursor row at {width}x{height}"
    );
    for &row in &marked {
        assert!(
            (2..separator).contains(&row),
            "{context}: cursor row {row} outside list 2..{separator} ({width}x{height}): {}",
            rows[row]
        );
    }
    let on_group = marked.iter().any(|&r| {
        ["Staged (", "Changes (", "Untracked ("]
            .iter()
            .any(|g| rows[r].contains(g))
    });
    assert!(
        on_group || marked.iter().any(|&r| rows[r].contains(&name)),
        "{context}: selected {name} not shown at {width}x{height}"
    );
    assert!(
        rows[separator].chars().nth(1) == Some('─'),
        "{context}: commit separator at row {separator}: {}",
        rows[separator]
    );
}

#[test]
fn selected_file_always_visible() {
    let mut files = Vec::new();
    for i in 0..40 {
        files.push((format!("f{i:02}.txt"), format!("old {i}\n")));
    }
    let initial: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, c)| (n.as_str(), c.as_str()))
        .collect();
    let f = Fixture::new(&initial);
    for (name, _) in &files {
        f.write(name, "new\n");
    }
    for i in 0..4 {
        f.write(&format!("u{i}.txt"), "untracked\n");
    }
    // List height depends only on terminal height: cover the minimum, both
    // sides of the commit-box switch (31/32) and a tall screen at 80 columns,
    // plus the reference size. Every draw is slow in debug builds.
    let sizes = [(80u16, 24u16), (80, 31), (80, 32), (80, 60), (160, 44)];
    let check = |w: &mut Workbench, context: &str| {
        for &(width, height) in &sizes {
            assert_selected_visible(w, width, height, context);
        }
    };
    for collapse_first in [false, true] {
        let mut w = Workbench::open(&f.0);
        if collapse_first {
            // Cursor starts on the first group; Left collapses it.
            key(&mut w, KeyCode::Left);
        }
        check(&mut w, "start");
        for step in 0..48 {
            key(&mut w, KeyCode::Down);
            check(&mut w, &format!("down {step}"));
        }
        assert_eq!(w.selected_path().unwrap(), PathBuf::from("u3.txt"));
        for step in 0..48 {
            key(&mut w, KeyCode::Up);
            check(&mut w, &format!("up {step}"));
        }
    }
}
