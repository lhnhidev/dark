use super::{layout::Layout, theme as t};
use crate::app::Screen;
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute},
    terminal::{Clear, ClearType},
};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as FmtWrite;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct View<'a> {
    pub tab: usize,
    pub project: &'a str,
    pub branch: &'a str,
    pub snapshot: &'a str,
    pub status: &'a str,
    pub message: &'a str,
    pub groups: &'a [FileGroup<'a>],
    pub file_scroll: usize,
    pub diff_path: &'a str,
    pub diff_added: usize,
    pub diff_removed: usize,
    pub hunk: &'a str,
    pub lines: &'a [DiffRow<'a>],
    pub show_files: bool,
    pub show_diff: bool,
    pub focus_files: bool,
    pub unified: bool,
    pub overlay: Option<Overlay<'a>>,
}
pub struct Overlay<'a> {
    pub title: &'a str,
    pub lines: &'a [String],
    pub footer: &'a str,
}
pub struct FileGroup<'a> {
    pub name: &'a str,
    pub collapsed: bool,
    pub selected: bool,
    pub files: &'a [FileEntry<'a>],
}
#[derive(Clone, Copy)]
pub struct FileEntry<'a> {
    pub path: &'a str,
    pub status: char,
    pub staged: bool,
    pub selected: bool,
    pub viewed: bool,
    pub added: usize,
    pub removed: usize,
}
pub struct DiffRow<'a> {
    pub old: Option<DiffCell<'a>>,
    pub new: Option<DiffCell<'a>>,
    pub header: Option<&'a str>,
    pub selected: bool,
}
pub struct DiffCell<'a> {
    pub number: Option<usize>,
    pub text: &'a str,
    pub kind: DiffKind,
    pub spans: &'a [Range<usize>],
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Context,
    Added,
    Removed,
}

pub struct Renderer {
    hashes: Vec<Option<u64>>,
    width: u16,
    line: String,
    color: bool,
    truecolor: bool,
    tab_width: usize,
}
impl Renderer {
    pub fn new() -> Self {
        Self {
            hashes: Vec::new(),
            width: 0,
            line: String::new(),
            color: std::env::var_os("NO_COLOR").is_none(),
            truecolor: std::env::var("COLORTERM").is_ok_and(|s| {
                s.eq_ignore_ascii_case("truecolor") || s.eq_ignore_ascii_case("24bit")
            }),
            tab_width: std::env::var("DARK_TAB_WIDTH")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|n| (1..=16).contains(n))
                .unwrap_or(8),
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
        let tab = screen.index();
        self.draw_view(
            writer,
            &View {
                tab,
                status,
                show_files: true,
                show_diff: true,
                ..View::default()
            },
            width,
            height,
        )
    }
    pub fn draw_view(
        &mut self,
        writer: &mut impl Write,
        view: &View<'_>,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        if self.width != width || self.hashes.len() != usize::from(height) {
            self.width = width;
            self.hashes.clear();
            self.hashes.resize(usize::from(height), None);
        }
        let result = (|| {
            for row in 0..height {
                make_line(
                    &mut self.line,
                    view,
                    width,
                    height,
                    row,
                    (self.color, self.truecolor, self.tab_width),
                );
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
            self.invalidate();
            return result;
        }
        for row in 0..height {
            make_line(
                &mut self.line,
                view,
                width,
                height,
                row,
                (self.color, self.truecolor, self.tab_width),
            );
            self.hashes[usize::from(row)] = Some(line_hash(&self.line, width));
        }
        Ok(())
    }
}
fn line_hash(line: &str, width: u16) -> u64 {
    let mut h = DefaultHasher::new();
    line.hash(&mut h);
    width.hash(&mut h);
    h.finish()
}
#[derive(Clone, Copy)]
struct Style {
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
    bold: bool,
    selected: bool,
}
fn style(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Style {
    Style {
        fg,
        bg,
        bold: false,
        selected: false,
    }
}
struct Line<'a> {
    out: &'a mut String,
    col: usize,
    limit: usize,
    color: bool,
    truecolor: bool,
    tab_width: usize,
}
impl Line<'_> {
    fn put(&mut self, text: &str, max: usize, s: Style) {
        let end = self.col.saturating_add(max).min(self.limit);
        if self.col >= end {
            return;
        }
        if self.color {
            for (code, color) in [
                (38, t::color(s.fg, self.truecolor)),
                (48, t::color(s.bg, self.truecolor)),
            ] {
                match color {
                    Color::Rgb { r, g, b } => {
                        let _ = write!(self.out, "\x1b[{code};2;{r};{g};{b}m");
                    }
                    Color::AnsiValue(index) => {
                        let _ = write!(self.out, "\x1b[{code};5;{index}m");
                    }
                    _ => unreachable!("theme emits RGB or indexed colors"),
                }
            }
        } else if s.selected {
            let _ = write!(self.out, "{}", SetAttribute(Attribute::Reverse));
        }
        if s.bold || (!self.color && s.selected) {
            let _ = write!(self.out, "{}", SetAttribute(Attribute::Bold));
        }
        for g in text.graphemes(true) {
            if self.col >= end {
                break;
            }
            if g == "\t" {
                let n = (self.tab_width - self.col % self.tab_width).min(end - self.col);
                for _ in 0..n {
                    self.out.push(' ');
                }
                self.col += n;
            } else if g.chars().any(char::is_control) {
                self.out.push(' ');
                self.col += 1;
            } else {
                let n = UnicodeWidthStr::width(g);
                if n > end - self.col {
                    break;
                }
                self.out.push_str(g);
                self.col += n;
            }
        }
        if self.color {
            let _ = write!(self.out, "{}", ResetColor);
        }
        if s.bold || (!self.color && s.selected) {
            let _ = write!(self.out, "{}", SetAttribute(Attribute::NormalIntensity));
        }
        if !self.color && s.selected {
            let _ = write!(self.out, "{}", SetAttribute(Attribute::NoReverse));
        }
    }
    fn fill(&mut self, end: usize, s: Style) {
        while self.col < end.min(self.limit) {
            let n = (end.min(self.limit) - self.col).min(128);
            self.put(&" ".repeat(n), n, s);
        }
    }
}
fn make_line(
    out: &mut String,
    v: &View<'_>,
    width: u16,
    height: u16,
    row: u16,
    options: (bool, bool, usize),
) {
    let (color, truecolor, tab_width) = options;
    out.clear();
    let layout = Layout::new(width, height, v.show_files, v.show_diff, v.unified);
    let mut l = Line {
        out,
        col: 0,
        limit: usize::from(width.saturating_sub(1)),
        color,
        truecolor,
        tab_width,
    };
    let normal = style(t::TEXT, t::BACKGROUND);
    let panel = style(t::TEXT, t::PANEL);
    let muted = style(t::MUTED, t::PANEL);
    if layout.small {
        if row == height / 2 {
            l.put("Cửa sổ quá nhỏ, cần ít nhất 80×24", l.limit, normal);
        }
        l.fill(l.limit, normal);
        return;
    }
    if row == 0 {
        let mut brand = style(t::ACCENT, t::PANEL);
        brand.bold = true;
        l.put(" dark ", 6, brand);
        let names = if layout.short_tabs {
            ["Rev", "Edt", "Vib", "Git", "Mod", "Vie", "Hlp"]
        } else {
            [
                "Review",
                "Edit",
                "Vibe",
                "Git commit",
                "Models",
                "View",
                "Help",
            ]
        };
        for (i, name) in names.iter().enumerate() {
            let selected = i == v.tab;
            let mut s = style(
                if selected { t::TEXT } else { t::MUTED },
                if selected { t::ACCENT_MUTED } else { t::PANEL },
            );
            s.bold = selected;
            s.selected = selected;
            l.put(" ", 1, s);
            let mut number = s;
            number.fg = if selected {
                t::FILE_MODIFIED
            } else {
                t::LINE_NUMBER
            };
            l.put(&(i + 1).to_string(), 1, number);
            l.put(&format!(" {} ", name), name.len() + 2, s);
        }
        if !layout.short_tabs {
            l.put(
                &format!("  {} · ⎇ {} · snapshot {}", v.project, v.branch, v.snapshot),
                l.limit.saturating_sub(l.col),
                muted,
            );
        }
        l.fill(l.limit, panel);
        return;
    }
    if row == height - 1 {
        l.put(
            &format!(
                " {}  ",
                ["REVIEW", "EDIT", "VIBE", "GIT", "MODELS", "VIEW", "HELP"]
                    .get(v.tab)
                    .unwrap_or(&"REVIEW")
            ),
            12,
            panel,
        );
        let status_width = UnicodeWidthStr::width(v.status).min(l.limit.saturating_sub(l.col));
        let separator = if status_width > 0 { 2 } else { 0 };
        let hint_end = l.limit.saturating_sub(status_width + separator);
        if !v.message.is_empty() {
            l.put(v.message, hint_end.saturating_sub(l.col), muted);
        } else {
            let hints: &[(&str, &str)] = if v.tab == 0 {
                &[
                    ("a", "accept"),
                    ("x", "reject"),
                    ("l", "danh sách file"),
                    ("r", "khung diff"),
                    ("n/N", "hunk"),
                    ("u", "hoàn tác"),
                    ("e", "editor ngoài"),
                    ("q", "thoát"),
                ]
            } else {
                &[("1", "Review"), ("q", "thoát")]
            };
            for (key, label) in hints {
                l.put(
                    key,
                    hint_end.saturating_sub(l.col),
                    style(t::TEXT, t::PANEL_ALT),
                );
                l.put(
                    &format!(" {label}  "),
                    hint_end.saturating_sub(l.col),
                    muted,
                );
            }
        }
        l.fill(hint_end, panel);
        l.fill(l.limit.saturating_sub(status_width), panel);
        l.put(v.status, status_width, muted);
        l.fill(l.limit, panel);
        return;
    }
    if let Some(overlay) = &v.overlay {
        let top = height
            .saturating_sub((overlay.lines.len() + 4).min(usize::from(height - 4)) as u16)
            / 2;
        let bottom = (top + overlay.lines.len() as u16 + 3).min(height - 2);
        if row >= top && row <= bottom {
            let left = l.limit / 8;
            l.fill(left, normal);
            let right = l.limit - left;
            let border = style(t::ACCENT, t::PANEL);
            l.put("│", 1, border);
            if row == top {
                l.put(overlay.title, right.saturating_sub(l.col + 1), panel);
            } else if row == bottom {
                l.put(overlay.footer, right.saturating_sub(l.col + 1), muted);
            } else if let Some(text) = overlay.lines.get(usize::from(row - top - 1)) {
                l.put(text, right.saturating_sub(l.col + 1), panel);
            }
            l.fill(right.saturating_sub(1), panel);
            l.put("│", 1, border);
            l.fill(l.limit, normal);
            return;
        }
    }
    if v.tab != 0 {
        if row == height / 2 {
            let label = [
                "Review",
                "Edit",
                "Vibe",
                "Git commit",
                "Models",
                "View",
                "Help",
            ]
            .get(v.tab)
            .unwrap_or(&"Review");
            l.fill(l.limit.saturating_sub(label.len()) / 2, normal);
            let mut s = normal;
            s.bold = true;
            l.put(label, label.len(), s);
        } else if row == height / 2 + 1 {
            l.fill(l.limit.saturating_sub(11) / 2, normal);
            l.put("Coming soon", 11, style(t::MUTED, t::BACKGROUND));
        }
        l.fill(l.limit, normal);
        return;
    }
    if !v.show_files && !v.show_diff {
        if row == height / 2 {
            l.put("Ấn l hoặc r để hiện lại", l.limit, normal);
        }
        l.fill(l.limit, normal);
        return;
    }
    if layout.files > 0 {
        pane(&mut l, v, row, height, layout.files, true, layout.split);
    }
    if layout.diff > 0 {
        pane(&mut l, v, row, height, layout.diff, false, layout.split);
    }
}
fn pane(
    l: &mut Line<'_>,
    v: &View<'_>,
    row: u16,
    height: u16,
    width: usize,
    files: bool,
    split: bool,
) {
    let active = if files { v.focus_files } else { !v.focus_files };
    let border = style(if active { t::ACCENT } else { t::BORDER }, t::BACKGROUND);
    let bottom = row == height - 2;
    l.put(
        if bottom {
            "└"
        } else if row == 1 {
            "┌"
        } else {
            "│"
        },
        1,
        border,
    );
    let end = l.col + width.saturating_sub(2);
    if bottom {
        while l.col < end {
            l.put("─", 1, border);
        }
    } else if files {
        file_line(l, v, row, height, width.saturating_sub(2));
    } else {
        diff_line(l, v, row, width.saturating_sub(2), split);
    }
    l.put(
        if bottom {
            "┘"
        } else if row == 1 {
            "┐"
        } else {
            "│"
        },
        1,
        border,
    );
}
fn file_line(l: &mut Line<'_>, v: &View<'_>, row: u16, height: u16, width: usize) {
    let start = l.col;
    let end = start + width;
    let panel = style(t::TEXT, t::PANEL);
    let normal = style(t::TEXT, t::BACKGROUND);
    if row == 1 {
        let count = v
            .groups
            .iter()
            .flat_map(|g| g.files)
            .map(|f| f.path)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let viewed = v
            .groups
            .iter()
            .flat_map(|g| g.files)
            .filter(|f| f.viewed)
            .map(|f| f.path)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let full = format!(" Changes {count} file · {viewed}/{count} đã xem");
        let compact = format!("Changes {count} · {viewed}/{count}");
        let label = if UnicodeWidthStr::width(full.as_str()) <= width {
            &full
        } else {
            &compact
        };
        l.put(label, width, panel);
        l.fill(end, panel);
        return;
    }
    if row >= height.saturating_sub(5) {
        let text = match height - row {
            5 => " Commit message",
            4 => " [ Có ở M3 ]",
            3 => "Space stage · c commit",
            _ => "",
        };
        l.put(text, width, style(t::LINE_NUMBER, t::BACKGROUND));
        l.fill(end, normal);
        return;
    }
    let mut index = usize::from(row - 2).saturating_add(v.file_scroll);
    for group in v.groups {
        if index == 0 {
            let mut group_style = style(
                t::MUTED,
                if group.selected {
                    t::ACCENT_MUTED
                } else {
                    t::BACKGROUND
                },
            );
            group_style.selected = group.selected;
            l.put(
                if group.selected { "▌" } else { " " },
                1,
                style(t::ACCENT, group_style.bg),
            );
            l.put(
                &format!(
                    "{} {} ({})",
                    if group.collapsed { '▸' } else { '▾' },
                    group.name,
                    group.files.len()
                ),
                width.saturating_sub(1),
                group_style,
            );
            l.fill(end, group_style);
            return;
        }
        index -= 1;
        if !group.collapsed {
            if let Some(file) = group.files.get(index) {
                let mut s = style(
                    t::TEXT,
                    if file.selected {
                        t::ACCENT_MUTED
                    } else {
                        t::BACKGROUND
                    },
                );
                s.selected = file.selected;
                l.put(
                    if file.selected { "▌" } else { " " },
                    1,
                    style(t::ACCENT, s.bg),
                );
                l.put(if file.staged { "[x] " } else { "[ ] " }, 4, s);
                let fg = match file.status {
                    'A' | '?' => t::FILE_ADDED,
                    'D' => t::FILE_DELETED,
                    _ => t::FILE_MODIFIED,
                };
                l.put(&format!("{} ", file.status), 2, style(fg, s.bg));
                let count = format!(" +{} −{}", file.added, file.removed);
                let count_width = UnicodeWidthStr::width(count.as_str());
                let available = end.saturating_sub(l.col + count_width);
                let label = compact_path(file.path, available);
                l.put(&label, available, s);
                if file.viewed {
                    l.put(" ✓", 2, s);
                }
                l.fill(end.saturating_sub(count_width), s);
                l.put(&count, end.saturating_sub(l.col), s);
                l.fill(end, s);
                return;
            }
            index = index.saturating_sub(group.files.len());
        }
    }
    l.fill(end, normal);
}
fn compact_path(path: &str, available: usize) -> String {
    if UnicodeWidthStr::width(path) <= available {
        return path.to_owned();
    }
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    if UnicodeWidthStr::width(basename) <= available {
        return basename.to_owned();
    }
    if available == 0 {
        return String::new();
    }
    let mut suffix = Vec::new();
    let mut used = 1;
    for grapheme in basename.graphemes(true).rev() {
        let width = UnicodeWidthStr::width(grapheme);
        if used + width > available {
            break;
        }
        suffix.push(grapheme);
        used += width;
    }
    format!("…{}", suffix.into_iter().rev().collect::<String>())
}
fn diff_line(l: &mut Line<'_>, v: &View<'_>, row: u16, width: usize, split: bool) {
    let end = l.col + width;
    let bg = style(t::TEXT, t::BACKGROUND);
    let panel = style(t::TEXT, t::PANEL);
    if row == 1 {
        let details = format!(
            " +{} −{}  {}",
            v.diff_added,
            v.diff_removed,
            if split { "split" } else { "unified" }
        );
        let details_width = UnicodeWidthStr::width(details.as_str()).min(width);
        let title_end = end.saturating_sub(details_width);
        l.put(
            &format!(" {}  {}", v.diff_path, v.hunk),
            title_end.saturating_sub(l.col),
            panel,
        );
        l.fill(title_end, panel);
        l.put(&details, details_width, panel);
        l.fill(end, panel);
        return;
    }
    let index = usize::from(row - 2);
    if let Some(d) = v.lines.get(index) {
        if let Some(header) = d.header {
            l.put(
                if d.selected { "▌" } else { " " },
                1,
                style(t::ACCENT, t::PANEL),
            );
            l.put(header, end.saturating_sub(l.col), style(t::MUTED, t::PANEL));
            l.fill(end, panel);
            return;
        }
        l.put(
            if d.selected { "▌" } else { " " },
            1,
            style(t::ACCENT, t::BACKGROUND),
        );
        let available = end.saturating_sub(l.col);
        if split {
            let half = available.saturating_sub(1) / 2;
            cell(l, d.old.as_ref(), half);
            l.put("│", 1, style(t::BORDER, t::BACKGROUND));
            cell(l, d.new.as_ref(), end.saturating_sub(l.col));
        } else {
            cell(l, d.old.as_ref().or(d.new.as_ref()), available);
        }
    } else {
        l.fill(end, bg);
    }
}
fn cell(l: &mut Line<'_>, cell: Option<&DiffCell<'_>>, width: usize) {
    let end = l.col + width;
    let Some(c) = cell else {
        l.fill(end, style(t::TEXT, t::PANEL));
        return;
    };
    let (bg, word, sign, fg) = match c.kind {
        DiffKind::Context => (t::BACKGROUND, t::BACKGROUND, " ", t::TEXT),
        DiffKind::Added => (t::ADDED_BG, t::ADDED_WORD_BG, "+", t::ADDED_SIGN),
        DiffKind::Removed => (t::REMOVED_BG, t::REMOVED_WORD_BG, "−", t::REMOVED_SIGN),
    };
    let n = c
        .number
        .map_or_else(|| "     ".to_string(), |n| format!("{n:>5}"));
    l.put(&n, 5, style(t::LINE_NUMBER, bg));
    l.put(sign, 1, style(fg, bg));
    l.put(" ", 1, style(t::TEXT, bg));
    let mut offset = 0;
    for span in c.spans {
        if span.start < offset
            || span.end > c.text.len()
            || !c.text.is_char_boundary(span.start)
            || !c.text.is_char_boundary(span.end)
        {
            continue;
        }
        l.put(
            &c.text[offset..span.start],
            end.saturating_sub(l.col),
            style(t::TEXT, bg),
        );
        l.put(
            &c.text[span.start..span.end],
            end.saturating_sub(l.col),
            style(t::TEXT, word),
        );
        offset = span.end;
    }
    l.put(
        &c.text[offset..],
        end.saturating_sub(l.col),
        style(t::TEXT, bg),
    );
    l.fill(end, style(t::TEXT, bg));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn style_changes_invalidate_line_and_monochrome_marks_selection() {
        let entry = FileEntry {
            path: "x.rs",
            status: 'M',
            staged: false,
            selected: false,
            viewed: false,
            added: 1,
            removed: 1,
        };
        let entries = [entry];
        let groups = [FileGroup {
            name: "Changes",
            collapsed: false,
            selected: false,
            files: &entries,
        }];
        let mut view = View {
            show_files: true,
            show_diff: true,
            groups: &groups,
            ..View::default()
        };
        let mut r = Renderer::new();
        r.color = false;
        let mut out = Vec::new();
        r.draw_view(&mut out, &view, 100, 24).unwrap();
        out.clear();
        r.draw_view(&mut out, &view, 100, 24).unwrap();
        assert!(out.is_empty());
        let selected = [FileEntry {
            selected: true,
            ..entries[0]
        }];
        let changed = [FileGroup {
            name: "Changes",
            collapsed: false,
            selected: false,
            files: &selected,
        }];
        view.groups = &changed;
        r.draw_view(&mut out, &view, 100, 24).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("▌"));
        assert!(output.contains("\x1b[7m"));
        assert!(!output.contains("\x1b[38;"));
    }
    #[test]
    fn clipped_text_preserves_graphemes_and_sanitizes_controls() {
        let mut out = String::new();
        let mut line = Line {
            out: &mut out,
            col: 0,
            limit: 4,
            color: false,
            truecolor: false,
            tab_width: 8,
        };
        line.put("界e\u{301}\x1bZ", 4, style(t::TEXT, t::BACKGROUND));
        assert_eq!(out, "界e\u{301} ");
    }
    #[test]
    fn tab_width_follows_column_not_utf8_byte_count() {
        let mut out = String::new();
        let mut line = Line {
            out: &mut out,
            col: 0,
            limit: 12,
            color: false,
            truecolor: false,
            tab_width: 4,
        };
        line.put("界\tX", 12, style(t::TEXT, t::BACKGROUND));
        assert_eq!(out, "界  X");
    }
    #[test]
    fn narrow_paths_keep_distinguishing_filename_suffixes() {
        assert_eq!(compact_path("src/module_02.ts", 7), "…_02.ts");
        assert_eq!(compact_path("src/module_03.ts", 7), "…_03.ts");
        assert_eq!(compact_path("src/界e\u{301}.rs", 5), "…e\u{301}.rs");
    }
    #[test]
    fn narrow_header_footer_and_disabled_commit_remain_readable() {
        let entries = [FileEntry {
            path: "src/module_02.ts",
            status: 'M',
            staged: false,
            selected: false,
            viewed: false,
            added: 50,
            removed: 50,
        }];
        let groups = [FileGroup {
            name: "Changes",
            collapsed: false,
            selected: false,
            files: &entries,
        }];
        let v = View {
            show_files: true,
            show_diff: true,
            groups: &groups,
            status: "0/1 file đã xem",
            ..View::default()
        };
        let mut r = Renderer::new();
        r.color = false;
        let mut output = Vec::new();
        r.draw_view(&mut output, &v, 80, 24).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Changes 1 · 0/1"));
        assert!(output.contains("Có ở M3"));
        assert!(output.contains("Space stage · c commit"));
        assert!(output.contains("  0/1 file đã xem"));
    }
    struct FailFlush;
    impl Write for FailFlush {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("fail"))
        }
    }
    #[test]
    fn failed_flush_does_not_commit_hashes() {
        let mut r = Renderer::new();
        let v = View::default();
        assert!(r.draw_view(&mut FailFlush, &v, 80, 24).is_err());
        let mut out = Vec::new();
        r.draw_view(&mut out, &v, 80, 24).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().matches("\x1b[2K").count(),
            24
        );
    }
    #[test]
    fn status_is_visible_and_file_scroll_keeps_selected_group() {
        let files = [FileEntry {
            path: "first",
            status: 'M',
            staged: false,
            selected: false,
            viewed: true,
            added: 1,
            removed: 2,
        }];
        let groups = [
            FileGroup {
                name: "Changes",
                collapsed: false,
                selected: false,
                files: &files,
            },
            FileGroup {
                name: "Untracked",
                collapsed: true,
                selected: true,
                files: &[],
            },
        ];
        let view = View {
            show_files: true,
            show_diff: true,
            groups: &groups,
            file_scroll: 2,
            status: "RAM 123 B",
            ..View::default()
        };
        let mut r = Renderer::new();
        r.color = false;
        let mut output = Vec::new();
        r.draw_view(&mut output, &view, 160, 24).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Untracked (0)"));
        assert!(!output.contains("first"));
        assert!(output.contains("RAM 123 B"));
        assert!(output.contains("1/1 đã xem"));
    }
    #[test]
    fn focus_color_change_redraws_unchanged_text() {
        let mut view = View {
            show_files: true,
            show_diff: true,
            ..View::default()
        };
        let mut r = Renderer::new();
        r.color = true;
        r.truecolor = true;
        let mut output = Vec::new();
        r.draw_view(&mut output, &view, 100, 24).unwrap();
        output.clear();
        let mut before = String::new();
        make_line(&mut before, &view, 100, 24, 1, (true, true, 8));
        view.focus_files = true;
        let mut after = String::new();
        make_line(&mut after, &view, 100, 24, 1, (true, true, 8));
        assert_ne!(before, after);
        r.draw_view(&mut output, &view, 100, 24).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("187;128;9"), "{output:?}");
        assert!(output.contains("\x1b[2K"));
    }

    #[test]
    fn partially_staged_file_is_counted_once_across_groups() {
        let entries = [FileEntry {
            path: "same.rs",
            status: 'M',
            staged: true,
            selected: true,
            viewed: true,
            added: 1,
            removed: 1,
        }];
        let groups = [
            FileGroup {
                name: "Staged",
                collapsed: false,
                selected: false,
                files: &entries,
            },
            FileGroup {
                name: "Changes",
                collapsed: false,
                selected: false,
                files: &entries,
            },
        ];
        let view = View {
            groups: &groups,
            show_files: true,
            show_diff: true,
            ..View::default()
        };
        let mut line = String::new();
        make_line(&mut line, &view, 160, 44, 1, (false, false, 8));
        assert!(line.contains("Changes 1 file · 1/1 đã xem"), "{line}");
    }

    #[test]
    fn primary_review_actions_are_discoverable_at_both_capture_widths() {
        let view = View {
            show_files: true,
            show_diff: true,
            status: "0/30 file đã xem",
            ..View::default()
        };
        for width in [80, 160] {
            let mut line = String::new();
            make_line(&mut line, &view, width, 24, 23, (false, false, 8));
            assert!(line.contains("accept"), "{line}");
            assert!(line.contains("reject"), "{line}");
        }
    }
}
