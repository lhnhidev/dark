use super::{
    layout::{self, Layout},
    syntax::{self, Lang, State, Token},
    theme as t,
};
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
    pub file_width: Option<u16>,
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
    pub lang: Lang,
    /// Syntax state at the start of this line for this side.
    pub state: State,
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
            truecolor: t::detect_truecolor(
                std::env::var("COLORTERM").ok().as_deref(),
                std::env::var("TERM").ok().as_deref(),
                std::env::var_os("WT_SESSION").is_some(),
                std::env::var("TERM_PROGRAM").ok().as_deref(),
                cfg!(windows),
            ),
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
    /// Style currently active in the terminal; only changes are emitted.
    fg: Option<(u8, u8, u8)>,
    bg: Option<(u8, u8, u8)>,
    bold: bool,
    reverse: bool,
}
impl<'a> Line<'a> {
    fn new(
        out: &'a mut String,
        limit: usize,
        color: bool,
        truecolor: bool,
        tab_width: usize,
    ) -> Self {
        Self {
            out,
            col: 0,
            limit,
            color,
            truecolor,
            tab_width,
            fg: None,
            bg: None,
            bold: false,
            reverse: false,
        }
    }
    /// Leave the terminal in its default style at the end of the row.
    fn finish(&mut self) {
        if self.color {
            if self.fg.is_some() || self.bg.is_some() || self.bold {
                let _ = write!(self.out, "{}", ResetColor);
            }
        } else {
            if self.bold {
                let _ = write!(self.out, "{}", SetAttribute(Attribute::NormalIntensity));
            }
            if self.reverse {
                let _ = write!(self.out, "{}", SetAttribute(Attribute::NoReverse));
            }
        }
        self.fg = None;
        self.bg = None;
        self.bold = false;
        self.reverse = false;
    }
    fn set_color(&mut self, code: u8, rgb: (u8, u8, u8)) {
        match t::color(rgb, self.truecolor) {
            Color::Rgb { r, g, b } => {
                let _ = write!(self.out, "\x1b[{code};2;{r};{g};{b}m");
            }
            Color::AnsiValue(index) => {
                let _ = write!(self.out, "\x1b[{code};5;{index}m");
            }
            _ => unreachable!("theme emits RGB or indexed colors"),
        }
    }
    fn put(&mut self, text: &str, max: usize, s: Style) {
        let end = self.col.saturating_add(max).min(self.limit);
        if self.col >= end {
            return;
        }
        let bold = s.bold || (!self.color && s.selected);
        if self.color {
            if self.fg != Some(s.fg) {
                self.set_color(38, s.fg);
                self.fg = Some(s.fg);
            }
            if self.bg != Some(s.bg) {
                self.set_color(48, s.bg);
                self.bg = Some(s.bg);
            }
        } else if s.selected != self.reverse {
            let attr = if s.selected {
                Attribute::Reverse
            } else {
                Attribute::NoReverse
            };
            let _ = write!(self.out, "{}", SetAttribute(attr));
            self.reverse = s.selected;
        }
        if bold != self.bold {
            let attr = if bold {
                Attribute::Bold
            } else {
                Attribute::NormalIntensity
            };
            let _ = write!(self.out, "{}", SetAttribute(attr));
            self.bold = bold;
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
    }
    fn fill(&mut self, end: usize, s: Style) {
        while self.col < end.min(self.limit) {
            let n = (end.min(self.limit) - self.col).min(128);
            self.put(&" ".repeat(n), n, s);
        }
    }
}
pub(super) fn make_line(
    out: &mut String,
    v: &View<'_>,
    width: u16,
    height: u16,
    row: u16,
    options: (bool, bool, usize),
) {
    let (color, truecolor, tab_width) = options;
    out.clear();
    let mut l = Line::new(
        out,
        usize::from(width.saturating_sub(1)),
        color,
        truecolor,
        tab_width,
    );
    build_line(&mut l, v, width, height, row);
    l.finish();
}
fn build_line(l: &mut Line<'_>, v: &View<'_>, width: u16, height: u16, row: u16) {
    let layout = Layout::with_file_width(
        width,
        height,
        v.show_files,
        v.show_diff,
        v.unified,
        v.file_width,
    );
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
            l.put(&format!("Alt+{}", i + 1), 5, number);
            l.put(&format!(" {}", name), name.len() + 1, s);
        }
        if !layout.short_tabs {
            let info = format!("{} · ⎇ {} · snapshot {}", v.project, v.branch, v.snapshot);
            let info_width = UnicodeWidthStr::width(info.as_str());
            let start = l.limit.saturating_sub(info_width).max(l.col + 2);
            l.fill(start, panel);
            l.put(&info, l.limit.saturating_sub(l.col), muted);
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
                    ("Alt+a", "accept"),
                    ("Alt+x", "reject"),
                    ("Alt+-/=/0", "c\u{1ed9}t"),
                    ("Alt+l/r", "khung"),
                    ("Alt+n/N", "hunk"),
                    ("Alt+u", "ho\u{e0}n t\u{e1}c"),
                    ("Alt+e", "editor ngo\u{e0}i"),
                    ("Alt+q", "tho\u{e1}t"),
                ]
            } else {
                &[("Alt+1", "Review"), ("Alt+q", "tho\u{e1}t")]
            };
            let key_style = style(t::TEXT, t::PANEL_ALT);
            for (key, label) in hints {
                if let Some((first, second)) = key.split_once('/').filter(|_| *key == "Alt+n/N") {
                    l.put(first, hint_end.saturating_sub(l.col), key_style);
                    l.put("/", hint_end.saturating_sub(l.col), muted);
                    l.put(second, hint_end.saturating_sub(l.col), key_style);
                } else {
                    l.put(key, hint_end.saturating_sub(l.col), key_style);
                }
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
            l.put("Ấn Alt+l hoặc Alt+r để hiện lại", l.limit, normal);
        }
        l.fill(l.limit, normal);
        return;
    }
    let both = layout.files > 0 && layout.diff > 0;
    let rows = (row, height);
    if layout.files > 0 {
        pane(l, v, rows, layout.files, true, (layout.split, both));
    }
    if layout.diff > 0 {
        pane(l, v, rows, layout.diff, false, (layout.split, both));
    }
}
fn edge(active: bool, glyph: &str) -> (&str, Style) {
    if active {
        (glyph, style(t::ACCENT, t::BACKGROUND))
    } else {
        (" ", style(t::TEXT, t::BACKGROUND))
    }
}
fn pane(
    l: &mut Line<'_>,
    v: &View<'_>,
    (row, height): (u16, u16),
    width: usize,
    files: bool,
    (split, both): (bool, bool),
) {
    let active = !both || files == v.focus_files;
    let panel = style(t::TEXT, t::PANEL);
    let start = l.col;
    if row == 1 {
        if files {
            let title = width - usize::from(both).min(width);
            file_title(l, v, title);
            l.fill(start + title, panel);
            if both {
                l.put("│", 1, style(t::BORDER, t::PANEL));
            }
        } else {
            diff_title(l, v, width, split);
        }
        return;
    }
    let bottom = row == height - 2;
    let own_left = files || !both;
    if own_left {
        let (g, s) = edge(active, if bottom { "└" } else { "│" });
        l.put(g, 1, s);
    }
    let content = width.saturating_sub(1 + usize::from(own_left));
    let end = l.col + content;
    if bottom {
        let (g, s) = edge(active, "─");
        while l.col < end {
            l.put(g, 1, s);
        }
    } else if files {
        file_line(l, v, row, height, content);
    } else {
        diff_line(l, v, row, content, split);
    }
    if files && both {
        // Shared middle column: border of whichever pane is active.
        let glyph = match (bottom, v.focus_files) {
            (false, _) => "│",
            (true, true) => "┘",
            (true, false) => "└",
        };
        l.put(glyph, 1, style(t::ACCENT, t::BACKGROUND));
    } else {
        let (g, s) = edge(active, if bottom { "┘" } else { "│" });
        l.put(g, 1, s);
    }
}
fn file_title(l: &mut Line<'_>, v: &View<'_>, width: usize) {
    let end = l.col + width;
    let panel = style(t::TEXT, t::PANEL);
    let muted = style(t::MUTED, t::PANEL);
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
    let full = format!(" {count} file · {viewed}/{count} đã xem");
    let compact = format!(" {count} · {viewed}/{count}");
    let rest = if 8 + UnicodeWidthStr::width(full.as_str()) <= width {
        &full
    } else {
        &compact
    };
    l.put(" ", 1, panel);
    l.put("Changes", width.saturating_sub(1), panel);
    l.put(rest, end.saturating_sub(l.col), muted);
    l.fill(end, panel);
}
fn diff_title(l: &mut Line<'_>, v: &View<'_>, width: usize, split: bool) {
    let end = l.col + width;
    let panel = style(t::TEXT, t::PANEL);
    let muted = style(t::MUTED, t::PANEL);
    let added = format!("+{}", v.diff_added);
    let removed = format!("−{}", v.diff_removed);
    let mode = if split { "split" } else { "unified" };
    let right = UnicodeWidthStr::width(added.as_str())
        + UnicodeWidthStr::width(removed.as_str())
        + mode.len()
        + 5;
    let right = right.min(width);
    let title_end = end - right;
    l.put(" ", 1, panel);
    l.put(v.diff_path, title_end.saturating_sub(l.col), panel);
    if !v.hunk.is_empty() {
        l.put("  ", 2, panel);
        l.put(v.hunk, title_end.saturating_sub(l.col), muted);
    }
    l.fill(title_end, panel);
    l.put(" ", 1, panel);
    l.put(
        &added,
        end.saturating_sub(l.col),
        style(t::FILE_ADDED, t::PANEL),
    );
    l.put(" ", 1, panel);
    l.put(
        &removed,
        end.saturating_sub(l.col),
        style(t::FILE_DELETED, t::PANEL),
    );
    l.put("  ", 2, panel);
    l.put(mode, end.saturating_sub(l.col), muted);
    l.fill(end, panel);
}
fn file_line(l: &mut Line<'_>, v: &View<'_>, row: u16, height: u16, width: usize) {
    let end = l.col + width;
    let normal = style(t::TEXT, t::BACKGROUND);
    let commit = layout::commit_rows(height);
    let top = height.saturating_sub(2 + commit);
    if row >= top {
        commit_line(l, usize::from(row - top), height, width);
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
                file_row(l, file, end);
                return;
            }
            index = index.saturating_sub(group.files.len());
        }
    }
    l.fill(end, normal);
}
fn file_row(l: &mut Line<'_>, file: &FileEntry<'_>, end: usize) {
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
    let added = (file.added > 0).then(|| format!("+{}", file.added));
    let removed = (file.removed > 0).then(|| format!("−{}", file.removed));
    let parts = usize::from(added.is_some()) + usize::from(removed.is_some());
    let counts_width = added.as_ref().map_or(0, |a| a.len())
        + removed
            .as_ref()
            .map_or(0, |r| UnicodeWidthStr::width(r.as_str()))
        + parts.saturating_sub(1);
    // Reserve one gap before the counts; they end at the content edge.
    let reserved = if counts_width > 0 {
        counts_width + 1
    } else {
        0
    };
    let check = if file.viewed { 2 } else { 0 };
    let available = end.saturating_sub(l.col + reserved + check);
    let label = compact_path(file.path, available);
    let split = label.rfind(['/', '\\']).map_or(0, |i| i + 1);
    l.put(&label[..split], available, style(t::LINE_NUMBER, s.bg));
    l.put(&label[split..], end.saturating_sub(l.col), s);
    if file.viewed {
        l.put(" ✓", 2, s);
    }
    if counts_width > 0 {
        l.fill(end.saturating_sub(counts_width), s);
        if let Some(a) = &added {
            l.put(a, end.saturating_sub(l.col), style(t::FILE_ADDED, s.bg));
        }
        if let Some(r) = &removed {
            if added.is_some() {
                l.put(" ", 1, s);
            }
            l.put(r, end.saturating_sub(l.col), style(t::FILE_DELETED, s.bg));
        }
    }
    l.fill(end, s);
}
fn commit_line(l: &mut Line<'_>, k: usize, height: u16, width: usize) {
    let end = l.col + width;
    let dim = style(t::LINE_NUMBER, t::BACKGROUND);
    let line = style(t::BORDER, t::BACKGROUND);
    let tall = height >= layout::TALL_COMMIT_HEIGHT;
    let hint = if tall { 5 } else { 3 };
    if k == 0 {
        l.put(&"─".repeat(width), width, line);
    } else if k == 1 {
        l.put(" Commit message", width, dim);
    } else if k == hint {
        let text = if width > 22 {
            " Alt+g stage · Alt+c commit"
        } else {
            "Alt+g stage · Alt+c commit"
        };
        l.put(text, width, dim);
    } else if !tall {
        l.put(" [ Có ở M3 ]", width, dim);
    } else {
        let inner = width.saturating_sub(4);
        l.put(" ", 1, dim);
        match k {
            2 => l.put(&format!("┌{}┐", "─".repeat(inner)), inner + 2, line),
            3 => {
                l.put("│", 1, line);
                l.put(" Có ở M3", inner, dim);
                l.fill(end.saturating_sub(2), dim);
                l.put("│", 1, line);
            }
            _ => l.put(&format!("└{}┘", "─".repeat(inner)), inner + 2, line),
        }
    }
    l.fill(end, style(t::TEXT, t::BACKGROUND));
}
const ELLIPSIS: &str = "…";
pub(super) fn compact_path(path: &str, available: usize) -> String {
    if UnicodeWidthStr::width(path) <= available {
        return path.to_owned();
    }
    let split = path.rfind(['/', '\\']).map_or(0, |i| i + 1);
    let (dir, base) = path.split_at(split);
    let base_width = UnicodeWidthStr::width(base);
    if base_width > available {
        if available == 0 {
            return String::new();
        }
        let budget = available - 1;
        // Keep at least one tail column so the end of the name stays visible;
        // with room for both, keep at least one head column too.
        let head_budget = if budget >= 2 {
            (budget / 4).max(1).min(budget - 1)
        } else {
            0
        };
        let mut head = String::new();
        let mut used = 0;
        for g in base.graphemes(true) {
            let w = UnicodeWidthStr::width(g);
            if used + w > head_budget {
                break;
            }
            head.push_str(g);
            used += w;
        }
        let mut tail = Vec::new();
        let mut tail_used = 0;
        for g in base.graphemes(true).rev() {
            let w = UnicodeWidthStr::width(g);
            if used + tail_used + w > budget {
                break;
            }
            tail.push(g);
            tail_used += w;
        }
        let tail: String = tail.into_iter().rev().collect();
        return format!("{head}{ELLIPSIS}{tail}");
    }
    // Keep whole trailing directory components behind a `…/` prefix.
    let components = dir.split(['/', '\\']).filter(|p| !p.is_empty()).count();
    for keep in (1..components).rev() {
        let mut tail_start = None;
        let mut seen = 0;
        for (i, c) in dir.char_indices().rev() {
            if (c == '/' || c == '\\') && i + 1 != dir.len() {
                seen += 1;
                if seen == keep {
                    tail_start = Some(i + 1);
                    break;
                }
            }
        }
        let Some(tail_start) = tail_start else {
            continue;
        };
        let candidate = format!("{ELLIPSIS}/{}{base}", &dir[tail_start..]);
        if UnicodeWidthStr::width(candidate.as_str()) <= available {
            return candidate;
        }
    }
    base.to_owned()
}
fn diff_line(l: &mut Line<'_>, v: &View<'_>, row: u16, width: usize, split: bool) {
    let end = l.col + width;
    let bg = style(t::TEXT, t::BACKGROUND);
    let panel = style(t::TEXT, t::PANEL);
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
    l.put(" ", 1, style(t::TEXT, bg));
    l.put(sign, 1, style(fg, bg));
    l.put(" ", 1, style(t::TEXT, bg));
    let lang = if l.color { c.lang } else { Lang::Plain };
    let mut state = c.state;
    let mut span_at = 0;
    let mut last_end = 0;
    syntax::highlight(lang, &mut state, c.text, |range, token| {
        let (fg, bold) = match token {
            Token::Text => (t::TEXT, false),
            Token::Keyword => (t::SYNTAX_KEYWORD, false),
            Token::String => (t::SYNTAX_STRING, false),
            Token::Comment => (t::SYNTAX_COMMENT, false),
            Token::Function => (t::SYNTAX_FUNCTION, false),
            Token::Constant => (t::SYNTAX_CONSTANT, false),
            Token::Type => (t::SYNTAX_TYPE, false),
            Token::Tag => (t::SYNTAX_TAG, false),
            Token::Heading => (t::SYNTAX_CONSTANT, true),
        };
        let mut pos = range.start;
        while pos < range.end {
            // Skip word spans that are invalid or already passed.
            while let Some(span) = c.spans.get(span_at) {
                let valid = span.start >= last_end
                    && span.start <= span.end
                    && span.end <= c.text.len()
                    && c.text.is_char_boundary(span.start)
                    && c.text.is_char_boundary(span.end);
                if valid && span.end > pos {
                    break;
                }
                if valid {
                    last_end = span.end;
                }
                span_at += 1;
            }
            let (piece_end, piece_bg) = match c.spans.get(span_at) {
                Some(span) if span.start <= pos => (range.end.min(span.end), word),
                Some(span) => (range.end.min(span.start), bg),
                None => (range.end, bg),
            };
            let mut s = style(fg, piece_bg);
            s.bold = bold;
            l.put(&c.text[pos..piece_end], end.saturating_sub(l.col), s);
            pos = piece_end;
        }
    });
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
        let mut line = Line::new(&mut out, 4, false, false, 8);
        line.put("界e\u{301}\x1bZ", 4, style(t::TEXT, t::BACKGROUND));
        assert_eq!(out, "界e\u{301} ");
    }
    #[test]
    fn tab_width_follows_column_not_utf8_byte_count() {
        let mut out = String::new();
        let mut line = Line::new(&mut out, 12, false, false, 4);
        line.put("界\tX", 12, style(t::TEXT, t::BACKGROUND));
        assert_eq!(out, "界  X");
    }
    #[test]
    fn narrow_paths_keep_distinguishing_filename_suffixes() {
        assert_eq!(compact_path("src/module_02.ts", 7), "m…02.ts");
        assert_eq!(compact_path("src/module_03.ts", 7), "m…03.ts");
        assert_eq!(compact_path("src/界e\u{301}.rs", 5), "…e\u{301}.rs");
    }
    #[test]
    fn directory_shrinks_from_left_then_disappears() {
        let p = "src/auth/session.ts";
        assert_eq!(compact_path(p, 19), p);
        assert_eq!(compact_path(p, 17), "…/auth/session.ts");
        assert_eq!(compact_path(p, 16), "session.ts");
        assert_eq!(compact_path("a/b/c/d.rs", 9), "…/c/d.rs");
    }
    #[test]
    fn active_pane_border_moves_without_shifting_content() {
        let mut v = View {
            show_files: true,
            show_diff: true,
            focus_files: true,
            ..View::default()
        };
        let mut a = String::new();
        let mut b = String::new();
        make_line(&mut a, &v, 100, 24, 3, (false, false, 8));
        v.focus_files = false;
        make_line(&mut b, &v, 100, 24, 3, (false, false, 8));
        assert_eq!(a.chars().count(), b.chars().count());
        assert!(a.starts_with('│') && !b.starts_with('│'));
        assert_eq!(a.matches('│').count(), 2);
        assert_eq!(b.matches('│').count(), 2);
        let mut bottom = String::new();
        make_line(&mut bottom, &v, 100, 24, 22, (false, false, 8));
        assert!(bottom.contains('└') && bottom.contains('┘'));
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
        assert!(output.contains("Alt+g stage"));
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
        make_line(&mut before, &view, 100, 24, 3, (true, true, 8));
        view.focus_files = true;
        let mut after = String::new();
        make_line(&mut after, &view, 100, 24, 3, (true, true, 8));
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
            assert!(line.contains("Alt+-/=/0"), "{line}");
        }
    }

    #[test]
    fn wider_file_pane_reveals_long_filename_and_keeps_rows_bounded() {
        let entries = [FileEntry {
            path: "src/a_very_long_distinct_filename.rs",
            status: 'M',
            staged: false,
            selected: true,
            viewed: false,
            added: 1,
            removed: 1,
        }];
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
        let mut narrow = String::new();
        make_line(&mut narrow, &view, 120, 24, 3, (false, false, 8));
        view.file_width = Some(60);
        let mut wide = String::new();
        make_line(&mut wide, &view, 120, 24, 3, (false, false, 8));
        assert!(
            !narrow.contains("a_very_long_distinct_filename.rs"),
            "{narrow}"
        );
        assert!(wide.contains("a_very_long_distinct_filename.rs"), "{wide}");
        for color in [false, true] {
            for row in 0..24 {
                let mut line = String::new();
                make_line(&mut line, &view, 120, 24, row, (color, true, 8));
                assert!(replay(&line).2, "row {row}: {line:?}");
            }
        }
    }

    fn diff_view_line(cell: DiffCell<'_>, color: bool) -> String {
        let rows = [DiffRow {
            old: Some(cell),
            new: None,
            header: None,
            selected: false,
        }];
        let view = View {
            show_files: false,
            show_diff: true,
            unified: true,
            lines: &rows,
            ..View::default()
        };
        let mut line = String::new();
        make_line(&mut line, &view, 100, 24, 2, (color, true, 8));
        line
    }

    #[test]
    fn keyword_gets_syntax_color_over_line_background() {
        let line = diff_view_line(
            DiffCell {
                number: Some(1),
                text: "let x",
                kind: DiffKind::Added,
                spans: &[],
                lang: Lang::Rust,
                state: State::default(),
            },
            true,
        );
        assert!(
            line.contains("\x1b[48;2;18;37;29m") && line.contains("\x1b[38;2;255;123;114mlet"),
            "{line:?}"
        );
    }

    #[test]
    fn word_span_keeps_word_background_with_syntax_color() {
        let span = 0..3;
        let spans = std::slice::from_ref(&span);
        let line = diff_view_line(
            DiffCell {
                number: Some(1),
                text: "let x",
                kind: DiffKind::Added,
                spans,
                lang: Lang::Rust,
                state: State::default(),
            },
            true,
        );
        assert!(
            line.contains("\x1b[38;2;255;123;114m\x1b[48;2;22;57;35mlet"),
            "{line:?}"
        );
        assert!(line.contains("\x1b[48;2;18;37;29m"), "{line:?}");
    }

    #[test]
    fn no_color_emits_no_syntax_codes() {
        let line = diff_view_line(
            DiffCell {
                number: Some(1),
                text: "let x = \"a\"; // c",
                kind: DiffKind::Added,
                spans: &[],
                lang: Lang::Rust,
                state: State::default(),
            },
            false,
        );
        assert!(line.contains("let x"));
        assert!(!line.contains("38;"), "{line:?}");
    }

    /// Replays escapes and returns (fg-set count, bg-set count, final state is default).
    fn replay(raw: &str) -> (usize, usize, bool) {
        let (mut fg, mut bg, mut bold, mut reverse) =
            (None::<String>, None::<String>, false, false);
        let (mut fgs, mut bgs) = (0, 0);
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                continue;
            }
            chars.next();
            let mut p = String::new();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
                p.push(c);
            }
            match p.as_str() {
                "0" => (fg, bg, bold, reverse) = (None, None, false, false),
                "1" => bold = true,
                "22" => bold = false,
                "7" => reverse = true,
                "27" => reverse = false,
                _ if p.starts_with("38;") => {
                    assert_ne!(fg.as_deref(), Some(p.as_str()), "redundant fg {p}");
                    fg = Some(p);
                    fgs += 1;
                }
                _ if p.starts_with("48;") => {
                    assert_ne!(bg.as_deref(), Some(p.as_str()), "redundant bg {p}");
                    bg = Some(p);
                    bgs += 1;
                }
                _ => {}
            }
        }
        (fgs, bgs, fg.is_none() && bg.is_none() && !bold && !reverse)
    }

    #[test]
    fn adjacent_segments_with_same_style_emit_one_color_sequence() {
        let mut out = String::new();
        let mut line = Line::new(&mut out, 40, true, true, 8);
        let a = style(t::TEXT, t::BACKGROUND);
        line.put("ab", 2, a);
        line.put("cd", 2, a);
        line.put("ef", 2, style(t::MUTED, t::BACKGROUND));
        line.finish();
        assert_eq!(out.matches("38;2;").count(), 2, "{out:?}");
        assert_eq!(out.matches("48;2;").count(), 1, "{out:?}");
        assert!(out.contains("abcd"), "{out:?}");
        assert!(out.ends_with("\x1b[0m"), "{out:?}");
        assert_eq!(out.matches("\x1b[0m").count(), 1, "{out:?}");
    }

    #[test]
    fn every_row_ends_reset_with_color_and_in_monochrome() {
        let entries = [FileEntry {
            path: "src/lib.rs",
            status: 'M',
            staged: false,
            selected: true,
            viewed: true,
            added: 3,
            removed: 2,
        }];
        let groups = [FileGroup {
            name: "Changes",
            collapsed: false,
            selected: true,
            files: &entries,
        }];
        for overlay in [
            None,
            Some(Overlay {
                title: "t",
                lines: &[],
                footer: "f",
            }),
        ] {
            let view = View {
                show_files: true,
                show_diff: true,
                groups: &groups,
                status: "status",
                overlay,
                ..View::default()
            };
            for color in [true, false] {
                for width in [80u16, 160] {
                    for row in 0..30 {
                        let mut line = String::new();
                        make_line(&mut line, &view, width, 30, row, (color, true, 8));
                        assert!(replay(&line).2, "row {row} color={color}: {line:?}");
                    }
                }
            }
        }
    }

    fn highlighted_frame_bytes() -> (usize, usize) {
        let texts: Vec<String> = (0..40)
            .map(|i| {
                format!("    let value_{i} = compute(\"text\", {i}) + other::<u32>(); // note")
            })
            .collect();
        let rows: Vec<DiffRow> = texts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let cell = |kind| DiffCell {
                    number: Some(i + 1),
                    text,
                    kind,
                    spans: &[],
                    lang: Lang::Rust,
                    state: State::default(),
                };
                DiffRow {
                    old: Some(cell(DiffKind::Removed)),
                    new: Some(cell(DiffKind::Added)),
                    header: None,
                    selected: false,
                }
            })
            .collect();
        let view = View {
            show_files: true,
            show_diff: true,
            lines: &rows,
            ..View::default()
        };
        let mut r = Renderer::new();
        r.color = true;
        r.truecolor = true;
        let mut out = Vec::new();
        r.draw_view(&mut out, &view, 160, 44).unwrap();
        let raw = String::from_utf8(out).unwrap();
        let sgr = raw.matches("\x1b[38;").count();
        (raw.len(), sgr)
    }

    #[test]
    fn highlighted_split_frame_stays_small() {
        let (bytes, fg_codes) = highlighted_frame_bytes();
        // Dense highlighted split diff: about 20 fg changes per row at most.
        assert!(fg_codes < 44 * 40, "{fg_codes} fg codes");
        assert!(bytes < 40_000, "{bytes} bytes");
        eprintln!("frame bytes={bytes} fg_codes={fg_codes}");
    }
}
