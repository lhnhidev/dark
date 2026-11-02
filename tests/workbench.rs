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

/// Rendered screen from row 2 on. The tab bar shows the snapshot clock, which
/// differs between two workbenches opened in different minutes (and reads
/// `n/a` on Linux when parallel tests race for the clock helper directory).
fn below_tab_bar(screen: &str) -> &str {
    &screen[screen.find("[2;1H").expect("row 2 cursor move")..]
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
fn external_refresh_preserves_selected_file_and_hunk() {
    let before: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let changed = before
        .replace("line 2\n", "first change\n")
        .replace("line 35\n", "second change\n");
    let f = Fixture::new(&[("a.txt", &before), ("b.txt", "old\n")]);
    f.write("a.txt", &changed);
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Char('n'));
    render(&mut w);
    let line = w.selected_line();
    assert!(line > 20);
    f.write("b.txt", "external file change\n");
    w.refresh_external();
    assert_eq!(w.selected_path().unwrap(), PathBuf::from("a.txt"));
    assert_eq!(w.selected_line(), line);
    assert!(plain(&render(&mut w)).contains("hunk 2/2"));
    f.write(
        "a.txt",
        &changed.replace("second change", "new external text"),
    );
    w.refresh_external();
    assert_eq!(w.selected_line(), line);
    assert!(plain(&render(&mut w)).contains("new external text"));
}

#[test]
fn external_refresh_does_not_replace_an_open_restore_confirmation() {
    let f = Fixture::new(&[("a.txt", "original\n")]);
    f.write("a.txt", "initial change\n");
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Char('u'));
    assert!(w.modal());
    f.write("a.txt", "external while confirming\n");
    w.refresh_external();
    assert!(w.modal());
    key(&mut w, KeyCode::Char('y'));
    assert_eq!(
        fs::read_to_string(f.0.join("a.txt")).unwrap(),
        "external while confirming\n"
    );
}

#[test]
fn external_refresh_selects_a_valid_file_when_current_change_disappears() {
    let f = Fixture::new(&[("a.txt", "original\n"), ("b.txt", "original\n")]);
    f.write("a.txt", "changed\n");
    let mut w = Workbench::open(&f.0);
    f.write("a.txt", "original\n");
    f.write("b.txt", "new change\n");
    w.refresh_external();
    assert_eq!(w.selected_path().unwrap(), PathBuf::from("b.txt"));
    assert!(plain(&render(&mut w)).contains("new change"));
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
    // Same detection as Renderer::new, so the expectation follows the platform.
    let truecolor = ui::theme::detect_truecolor(
        std::env::var("COLORTERM").ok().as_deref(),
        std::env::var("TERM").ok().as_deref(),
        std::env::var_os("WT_SESSION").is_some(),
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        cfg!(windows),
    );
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
    // List height depends only on terminal height: cover the minimum and both
    // sides of the commit-box switch (31/32) at 80 columns, plus the tall
    // reference size. Every draw is slow in debug builds.
    let sizes = [(80u16, 24u16), (80, 31), (80, 32), (160, 44)];
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

// ---------------------------------------------------------------------------
// Diff syntax highlighting end to end (real git fixture through Workbench).
// ---------------------------------------------------------------------------

/// Colored runs of drawn text: (SGR foreground parameters, visible text).
/// The foreground is `2;r;g;b` or `5;n`, exactly what the renderer emitted.
fn color_runs(ansi: &str) -> Vec<(Option<String>, String)> {
    let mut runs: Vec<(Option<String>, String)> = Vec::new();
    let mut fg: Option<String> = None;
    let mut text = String::new();
    let mut chars = ansi.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            text.push(c);
            continue;
        }
        if !text.is_empty() {
            runs.push((fg.clone(), std::mem::take(&mut text)));
        }
        if chars.next() != Some('[') {
            continue;
        }
        let mut params = String::new();
        let mut last = ' ';
        for p in chars.by_ref() {
            last = p;
            if ('@'..='~').contains(&p) {
                break;
            }
            params.push(p);
        }
        if last == 'm' {
            if let Some(rest) = params.strip_prefix("38;") {
                fg = Some(rest.to_owned());
            } else if params == "0" || params.is_empty() {
                fg = None;
            }
        }
    }
    if !text.is_empty() {
        runs.push((fg, text));
    }
    runs
}

/// The foreground parameters the renderer uses on this platform for a theme color.
fn fg_key(rgb: (u8, u8, u8)) -> String {
    let truecolor = ui::theme::detect_truecolor(
        std::env::var("COLORTERM").ok().as_deref(),
        std::env::var("TERM").ok().as_deref(),
        std::env::var_os("WT_SESSION").is_some(),
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        cfg!(windows),
    );
    match ui::theme::color(rgb, truecolor) {
        crossterm::style::Color::Rgb { r, g, b } => format!("2;{r};{g};{b}"),
        crossterm::style::Color::AnsiValue(n) => format!("5;{n}"),
        _ => unreachable!(),
    }
}

fn runs_with(runs: &[(Option<String>, String)], fg: (u8, u8, u8), text: &str) -> usize {
    let key = fg_key(fg);
    runs.iter()
        .filter(|(f, t)| f.as_deref() == Some(key.as_str()) && t == text)
        .count()
}

fn runs_containing(runs: &[(Option<String>, String)], fg: (u8, u8, u8), text: &str) -> usize {
    let key = fg_key(fg);
    runs.iter()
        .filter(|(f, t)| f.as_deref() == Some(key.as_str()) && t.contains(text))
        .count()
}

const KEYWORD: (u8, u8, u8) = (255, 123, 114);
const COMMENT: (u8, u8, u8) = (139, 148, 158);

fn comment_pair_old_has_comment() -> (&'static str, &'static str) {
    (
        "let a = 1;\n/* note start\nreturn value;\nnote end */\nlet c = 3;\n",
        "let a = 1;\nreturn value;\nlet c = 3;\n",
    )
}

#[test]
fn comment_opened_only_on_removed_side_does_not_color_the_new_side() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let (old, new) = comment_pair_old_has_comment();
    let f = Fixture::new(&[("a.ts", old)]);
    f.write("a.ts", new);
    let mut w = Workbench::open(&f.0);
    let runs = color_runs(&render(&mut w));
    // Old side: the unchanged line sits inside the removed block comment.
    assert_eq!(runs_with(&runs, COMMENT, "return value;"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, COMMENT, "/* note start"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, COMMENT, "note end */"), 1, "{runs:?}");
    // New side: the same line is plain code with a keyword.
    assert_eq!(runs_with(&runs, KEYWORD, "return"), 1, "{runs:?}");
    // The closing `*/` on the old side ends the comment: the next line is code on both sides.
    assert_eq!(runs_with(&runs, KEYWORD, "let"), 4, "{runs:?}");
}

#[test]
fn comment_opened_only_on_added_side_does_not_color_the_old_side() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let (old, new) = comment_pair_old_has_comment();
    let f = Fixture::new(&[("a.ts", new)]);
    f.write("a.ts", old);
    let mut w = Workbench::open(&f.0);
    let runs = color_runs(&render(&mut w));
    assert_eq!(runs_with(&runs, COMMENT, "return value;"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, COMMENT, "/* note start"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, KEYWORD, "return"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, KEYWORD, "let"), 4, "{runs:?}");
}

#[test]
fn unified_view_follows_the_old_side_for_unchanged_lines() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let (old, new) = comment_pair_old_has_comment();
    let f = Fixture::new(&[("a.ts", old)]);
    f.write("a.ts", new);
    let mut w = Workbench::open(&f.0);
    key(&mut w, KeyCode::Char('v'));
    let runs = color_runs(&render(&mut w));
    // One row per line; the removed lines keep their comment color and the
    // context line shows once, never mixing in the new side's color.
    assert_eq!(runs_with(&runs, COMMENT, "/* note start"), 1, "{runs:?}");
    assert_eq!(runs_with(&runs, COMMENT, "note end */"), 1, "{runs:?}");
    assert_eq!(
        runs_with(&runs, COMMENT, "return value;") + runs_with(&runs, KEYWORD, "return"),
        1,
        "{runs:?}"
    );
}

#[test]
fn syntax_state_does_not_leak_from_one_hunk_to_the_next() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let mut old = String::new();
    for i in 0..40 {
        if i == 36 {
            old.push_str("return old;\n");
        } else {
            old.push_str(&format!("let v{i} = {i};\n"));
        }
    }
    let new = old
        .replace("let v3 = 3;", "/* never closed")
        .replace("return old;", "return fresh;");
    let f = Fixture::new(&[("m.ts", &old)]);
    f.write("m.ts", &new);
    let mut w = Workbench::open(&f.0);
    let runs = color_runs(&render(&mut w));
    // First hunk really opens a comment on the new side...
    assert!(runs_containing(&runs, COMMENT, "never") >= 1, "{runs:?}");
    // ...but the second hunk starts clean: `return` is a keyword, never comment.
    assert!(runs_with(&runs, KEYWORD, "return") >= 1, "{runs:?}");
    assert_eq!(runs_containing(&runs, COMMENT, "return"), 0, "{runs:?}");
    assert_eq!(runs_containing(&runs, COMMENT, "fresh"), 0, "{runs:?}");
}

#[test]
fn hunk_that_starts_inside_an_unchanged_comment_still_draws() {
    let mut old = String::from("/* header comment\n");
    for i in 0..14 {
        old.push_str(&format!(" line a{i}\n"));
    }
    old.push_str("*/\nlet x = 1;\n");
    let f = Fixture::new(&[("c.ts", &old)]);
    f.write("c.ts", &old.replace(" line a8\n", " line a8 changed\n"));
    let mut w = Workbench::open(&f.0);
    let screen = plain(&render(&mut w));
    assert!(screen.contains("line a8 changed"), "{screen}");
    assert!(screen.contains("line a7"), "{screen}");
}

#[test]
fn plain_extension_is_drawn_without_syntax_colors() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let f = Fixture::new(&[("notes.txt", "let a = 1; // old\n")]);
    f.write("notes.txt", "let a = 2; // new\n");
    let mut w = Workbench::open(&f.0);
    let runs = color_runs(&render(&mut w));
    for rgb in [
        ui::theme::SYNTAX_KEYWORD,
        ui::theme::SYNTAX_STRING,
        ui::theme::SYNTAX_COMMENT,
        ui::theme::SYNTAX_FUNCTION,
        ui::theme::SYNTAX_CONSTANT,
        ui::theme::SYNTAX_TYPE,
        ui::theme::SYNTAX_TAG,
    ] {
        assert_eq!(runs_containing(&runs, rgb, "let"), 0, "{runs:?}");
        assert_eq!(runs_containing(&runs, rgb, "// "), 0, "{runs:?}");
    }
}

#[test]
fn very_large_hunk_is_highlighted_to_the_last_line() {
    // One 5,000-line hunk (10,000 diff lines). Only completion is asserted.
    let mut old = String::new();
    let mut new = String::new();
    for i in 0..5000 {
        old.push_str(&format!("let v{i} = {i}; // old\n"));
        new.push_str(&format!("const w{i} = \"{i}\"; /* new */\n"));
    }
    let f = Fixture::new(&[("big.ts", &old)]);
    f.write("big.ts", &new);
    let mut w = Workbench::open(&f.0);
    for _ in 0..340 {
        key(&mut w, KeyCode::PageDown);
    }
    let screen = render(&mut w);
    assert!(plain(&screen).contains("w4999"), "last line not drawn");
}

// Syntax state checkpoints (scan from the nearest checkpoint, not the hunk start).

fn long_hunk_lines(count: usize) -> Vec<review::DiffLine> {
    use review::{DiffLine, LineKind};
    (0..count)
        .map(|i| {
            let kind = match i % 7 {
                0 | 1 => LineKind::Delete,
                2..=4 => LineKind::Add,
                _ => LineKind::Context,
            };
            // Comments open and close on one side only, across checkpoint boundaries.
            let text = match i % 331 {
                5 => "let a = 1; /* open".to_string(),
                200 => "close */ let b = 2;".to_string(),
                17 => "let s = \"quoted\"; // line".to_string(),
                _ => format!("let v{i} = {i};"),
            };
            DiffLine {
                kind,
                old_line: None,
                new_line: None,
                text,
            }
        })
        .collect()
}

fn full_scan(
    lang: ui::syntax::Lang,
    lines: &[review::DiffLine],
) -> Vec<(ui::syntax::State, ui::syntax::State)> {
    use review::LineKind;
    let (mut old, mut new) = Default::default();
    let mut all = Vec::new();
    for line in lines {
        all.push((old, new));
        if line.kind != LineKind::Add {
            ui::syntax::highlight(lang, &mut old, &line.text, |_, _| {});
        }
        if line.kind != LineKind::Delete {
            ui::syntax::highlight(lang, &mut new, &line.text, |_, _| {});
        }
    }
    all
}

#[test]
fn checkpoint_states_equal_full_scan_at_any_scroll_order() {
    let lang = ui::syntax::Lang::Rust;
    let lines = long_hunk_lines(2000);
    let reference = full_scan(lang, &lines);
    // Distinct states must exist, otherwise the comparison proves nothing.
    assert!(reference.iter().any(|s| *s != reference[0]));
    let mut checkpoints = Vec::new();
    // Deliberately jumps forward, back, to a boundary, and past the end.
    for first in [0usize, 1500, 255, 256, 257, 1999, 700, 0, 1024, 1990] {
        let needed: Vec<usize> = (first..(first + 40).min(2000)).collect();
        let got = app::workbench::states_at(lang, &lines, &mut checkpoints, &needed);
        for (i, state) in needed.iter().zip(&got) {
            assert_eq!(*state, reference[*i], "line {i} (first {first})");
        }
    }
    // Sparse, split-style access (two distant groups) is also exact.
    let needed = [100usize, 101, 1700, 1701];
    let got = app::workbench::states_at(lang, &lines, &mut checkpoints, &needed);
    for (i, state) in needed.iter().zip(&got) {
        assert_eq!(*state, reference[*i], "sparse line {i}");
    }
    assert!(checkpoints.len() <= 2000 / 256 + 1);
}

fn long_comment_fixture() -> Fixture {
    let mut old = String::new();
    for i in 0..600 {
        old.push_str(&format!("let v{i} = {i};\n"));
    }
    let mut new = String::new();
    for i in 0..600 {
        match i {
            10 => new.push_str("/* open comment\n"),
            450 => new.push_str("close */\n"),
            _ => new.push_str(&format!("let w{i} = {i};\n")),
        }
    }
    let f = Fixture::new(&[("long.ts", &old)]);
    f.write("long.ts", &new);
    f
}

fn comment_covers(w: &mut Workbench, text: &str) -> bool {
    let runs = color_runs(&render(w));
    runs_containing(&runs, COMMENT, text) >= 1
}

#[test]
fn comment_across_checkpoints_stays_comment_when_scrolling_both_ways() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let f = long_comment_fixture();
    for unified in [false, true] {
        let mut w = Workbench::open(&f.0);
        if unified {
            key(&mut w, KeyCode::Char('v'));
        }
        // Rows between line 10 and 900 are inside the comment; after 900 they are not.
        let mut seen_inside = false;
        for _ in 0..80 {
            key(&mut w, KeyCode::PageDown);
            let screen = plain(&render(&mut w));
            if screen.contains("w300 =") || screen.contains("w350 =") {
                assert!(
                    comment_covers(&mut w, "w300 =") || comment_covers(&mut w, "w350 ="),
                    "unified={unified}"
                );
                seen_inside = true;
            }
        }
        assert!(seen_inside, "never scrolled into the comment");
        for _ in 0..160 {
            key(&mut w, KeyCode::PageUp);
            render(&mut w);
        }
        for _ in 0..80 {
            key(&mut w, KeyCode::PageDown);
            render(&mut w);
        }
    }
}

#[test]
fn checkpoints_reset_when_file_selection_changes() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    // a.ts opens a comment that never closes; b.ts has plain code.
    let mut a_old = String::new();
    let mut b_old = String::new();
    for i in 0..600 {
        a_old.push_str(&format!("let a{i} = {i};\n"));
        b_old.push_str(&format!("let b{i} = {i};\n"));
    }
    let a_new = a_old.replacen("let a2 = 2;", "/* open", 1);
    let b_new = b_old.replace("let b300 = 300;", "let b300 = 301;");
    let f = Fixture::new(&[("a.ts", &a_old), ("b.ts", &b_old)]);
    f.write("a.ts", &a_new);
    f.write("b.ts", &b_new);
    let mut w = Workbench::open(&f.0);
    for _ in 0..5 {
        key(&mut w, KeyCode::PageDown);
        render(&mut w);
    }
    // Move to b.ts and compare to a workbench that never saw a.ts.
    w.focus_files = true;
    key(&mut w, KeyCode::Down);
    assert_eq!(w.selected_path().unwrap(), PathBuf::from("b.ts"));
    let switched = render(&mut w);
    let mut fresh = Workbench::open(&f.0);
    key(&mut fresh, KeyCode::Down);
    assert_eq!(fresh.selected_path().unwrap(), PathBuf::from("b.ts"));
    assert_eq!(below_tab_bar(&switched), below_tab_bar(&render(&mut fresh)));
}

#[test]
fn checkpoints_reset_after_refresh_with_changed_content() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    let mut old = String::new();
    for i in 0..900 {
        old.push_str(&format!("let v{i} = {i};\n"));
    }
    let opened = old.replacen("let v1 = 1;", "/* open", 1);
    let f = Fixture::new(&[("r.ts", &old)]);
    f.write("r.ts", &opened);
    let mut w = Workbench::open(&f.0);
    for _ in 0..3 {
        key(&mut w, KeyCode::PageDown);
        render(&mut w);
    }
    // The comment is removed on disk; refresh must not keep stale states.
    let closed = old.replacen("let v1 = 1;", "let v1 = 11;", 1);
    f.write("r.ts", &closed);
    w.refresh();
    let after = render(&mut w);
    let mut fresh = Workbench::open(&f.0);
    assert_eq!(below_tab_bar(&after), below_tab_bar(&render(&mut fresh)));
    assert_eq!(runs_containing(&color_runs(&after), COMMENT, "v1"), 0);
}
