use crate::git::Repo;
use crate::review::{FileDiff, LineKind, Review, SnapshotOptions};
use crate::ui::{
    DiffCell, DiffKind, DiffRow, FileEntry, FileGroup, Layout, Overlay, Renderer, View,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum Item {
    Group(usize),
    File(usize),
}
enum Prompt {
    Oversized,
    Undo { paths: Vec<PathBuf>, token: String },
}
#[derive(Clone, Copy)]
struct Row {
    hunk: usize,
    old: Option<usize>,
    new: Option<usize>,
    header: bool,
}

pub struct Workbench {
    pub review: Option<Review>,
    pub message: String,
    pub show_files: bool,
    pub show_diff: bool,
    pub focus_files: bool,
    pub unified: bool,
    selected: usize,
    cursor: usize,
    collapsed: [bool; 3],
    hunk: usize,
    scroll: usize,
    prompt_scroll: usize,
    prompt: Option<Prompt>,
    diff: FileDiff,
    viewed: HashSet<(PathBuf, String)>,
    completed: HashMap<PathBuf, String>,
    project: String,
    branch: String,
    snapshot_time: String,
    retain: usize,
}

impl Workbench {
    pub fn open(path: &Path) -> Self {
        let options = SnapshotOptions::default();
        let retain = options.retain_sessions;
        let result = Repo::discover(path).and_then(|repo| Review::open(repo, options));
        let (review, message) = match result {
            Ok(review) => (Some(review), String::new()),
            Err(error) => (None, format!("Không mở được Review: {error}")),
        };
        let project = review
            .as_ref()
            .and_then(|r| r.repo.root.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let branch = review
            .as_ref()
            .map(|r| {
                let (ahead, behind) = r.repo.ahead_behind();
                let mut branch = r.repo.branch().to_string();
                if ahead > 0 {
                    branch.push_str(&format!(" ↑{ahead}"));
                }
                if behind > 0 {
                    branch.push_str(&format!(" ↓{behind}"));
                }
                branch
            })
            .unwrap_or_default();
        let prompt = review
            .as_ref()
            .filter(|r| r.needs_confirmation)
            .map(|_| Prompt::Oversized);
        let mut this = Self {
            review,
            message,
            show_files: true,
            show_diff: true,
            focus_files: true,
            unified: false,
            selected: 0,
            cursor: 1,
            collapsed: [false; 3],
            hunk: 0,
            scroll: 0,
            prompt_scroll: 0,
            prompt,
            diff: FileDiff::default(),
            viewed: HashSet::new(),
            completed: HashMap::new(),
            project,
            branch,
            snapshot_time: local_clock(),
            retain,
        };
        if this.prompt.is_none() {
            this.load();
        }
        this.sync_cursor();
        this
    }

    pub fn modal(&self) -> bool {
        self.prompt.is_some()
    }

    pub fn title(&self, tab: &str) -> String {
        format!(
            "dark · {} · {} · {} · {}/{} đã xem",
            self.project,
            self.branch,
            tab,
            self.completed.len(),
            self.review.as_ref().map(|r| r.files.len()).unwrap_or(0)
        )
    }

    fn sync_cursor(&mut self) {
        self.cursor = self
            .items()
            .iter()
            .position(|i| matches!(i,Item::File(index) if *index==self.selected))
            .unwrap_or(0);
    }
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.review
            .as_ref()?
            .files
            .get(self.selected)
            .map(|f| f.path.clone())
    }
    pub fn selected_line(&self) -> usize {
        self.diff
            .hunks
            .get(self.hunk)
            .map(|h| h.new_start.max(1) as usize)
            .unwrap_or(1)
    }

    fn items(&self) -> Vec<Item> {
        let mut items = Vec::new();
        if let Some(review) = &self.review {
            for group in 0..3 {
                items.push(Item::Group(group));
                if !self.collapsed[group] {
                    for (index, file) in review.files.iter().enumerate() {
                        if in_group(file, group) {
                            items.push(Item::File(index));
                        }
                    }
                }
            }
        }
        items
    }

    pub fn load(&mut self) {
        self.hunk = 0;
        self.scroll = 0;
        let Some(review) = &self.review else {
            return;
        };
        let Some(file) = review.files.get(self.selected) else {
            self.diff = FileDiff::default();
            return;
        };
        match review.diff(&file.path) {
            Ok(diff) => {
                if self
                    .completed
                    .get(&file.path)
                    .is_some_and(|v| *v != patch_fingerprint(&diff))
                {
                    self.completed.remove(&file.path);
                }
                self.diff = diff;
                if self.diff.binary {
                    self.message =
                        "File nhị phân hoặc không UTF-8: dùng editor ngoài; không reject theo hunk"
                            .into();
                }
            }
            Err(error) => {
                self.diff = FileDiff::default();
                self.message = if error.to_string().contains("external editor") {
                    "Diff quá lớn: nhấn e để xem trong editor ngoài. File được giữ nguyên.".into()
                } else {
                    format!("Không tải được diff: {error}")
                };
            }
        }
    }

    pub fn refresh(&mut self) {
        let path = self.selected_path();
        if let Some(review) = &mut self.review {
            match review.refresh() {
                Ok(()) => {
                    self.completed.retain(|path, version| {
                        review
                            .diff(path)
                            .is_ok_and(|diff| patch_fingerprint(&diff) == *version)
                    });
                    self.selected = path
                        .and_then(|p| review.files.iter().position(|f| f.path == p))
                        .unwrap_or(0);
                    self.load();
                    self.sync_cursor();
                }
                Err(error) => self.message = error.to_string(),
            }
        }
    }

    /// Returns an external-editor request. Modal input never reaches global shortcuts.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if self.modal() {
            self.prompt_key(key.code);
            return false;
        }
        self.message.clear();
        match key.code {
            KeyCode::Char('l') => {
                self.show_files = !self.show_files;
                self.focus_files = self.show_files;
            }
            KeyCode::Char('r') => {
                self.show_diff = !self.show_diff;
                self.focus_files = !self.show_diff && self.show_files;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if self.show_files && self.show_diff {
                    self.focus_files = !self.focus_files;
                }
            }
            KeyCode::Char('v') => {
                self.unified = !self.unified;
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(15),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(15),
            KeyCode::Home => self.scroll = 0,
            KeyCode::Char('n') if !key.modifiers.contains(KeyModifiers::SHIFT) => self.next_hunk(1),
            KeyCode::Char('N') | KeyCode::Char('n') => self.next_hunk(-1),
            KeyCode::Enter => {
                if self.focus_files {
                    match self.items().get(self.cursor).copied() {
                        Some(Item::Group(group)) => self.collapsed[group] = !self.collapsed[group],
                        Some(Item::File(index)) => {
                            self.selected = index;
                            self.load();
                            self.focus_files = false;
                        }
                        None => {}
                    }
                } else {
                    self.message = "Sửa trong dark có ở M2; nhấn e để mở editor ngoài".into();
                }
            }
            KeyCode::Left | KeyCode::Right if self.focus_files => {
                let group = match self.items().get(self.cursor) {
                    Some(Item::Group(g)) => *g,
                    Some(Item::File(i)) => self
                        .review
                        .as_ref()
                        .and_then(|r| r.files.get(*i))
                        .map(|f| {
                            if f.untracked {
                                2
                            } else if f.staged {
                                0
                            } else {
                                1
                            }
                        })
                        .unwrap_or(0),
                    None => 0,
                };
                self.collapsed[group] = key.code == KeyCode::Left;
            }
            KeyCode::Char('a') => {
                if let (Some(path), Some(hunk)) =
                    (self.selected_path(), self.diff.hunks.get(self.hunk))
                {
                    self.viewed.insert((path, hunk_fingerprint(hunk)));
                    if let Some(path) = self.selected_path() {
                        if self
                            .diff
                            .hunks
                            .iter()
                            .all(|h| self.viewed.contains(&(path.clone(), hunk_fingerprint(h))))
                        {
                            self.completed.insert(path, patch_fingerprint(&self.diff));
                        }
                    }
                    self.message = "Đã accept: giữ nguyên working tree, không stage".into();
                    self.next_hunk(1);
                }
            }
            KeyCode::Char('x') => {
                if let Some(path) = self.selected_path() {
                    if let Some(review) = &mut self.review {
                        let result = review.reject(&path, &self.diff, self.hunk);
                        match result {
                            Ok(()) => {
                                self.refresh();
                                self.message = "Đã reject hunk; index giữ nguyên".into();
                            }
                            Err(error) => {
                                let text = error.to_string();
                                self.message = if text.contains("staged") {
                                    "Hunk đã được stage: hãy unstage trước; dark hỗ trợ unstage ở M3".into()
                                } else {
                                    text
                                };
                            }
                        }
                    }
                }
            }
            KeyCode::Char('u') => {
                if let Some(review) = &self.review {
                    let confirmation = (|| -> io::Result<_> {
                        let before = review.undo_token()?;
                        let plan = review.undo_plan()?;
                        let token = review.undo_token()?;
                        if before != token {
                            return Err(io::Error::other(
                                "File đang thay đổi; hãy mở lại hộp xác nhận u",
                            ));
                        }
                        Ok((plan, token))
                    })();
                    match confirmation {
                        Ok((plan, token)) => {
                            self.prompt = Some(Prompt::Undo { paths: plan, token });
                            self.prompt_scroll = 0;
                        }
                        Err(error) => self.message = error.to_string(),
                    }
                }
            }
            KeyCode::Char('e') => return self.selected_path().is_some(),
            KeyCode::Char('?') => self.message = "Hỏi agent có ở M5".into(),
            KeyCode::Char('/') => self.message = "Ghi chú gửi agent có ở M5".into(),
            KeyCode::Char(' ') | KeyCode::Char('s') | KeyCode::Char('c') | KeyCode::Char('d') => {
                self.message = "Stage, commit và bỏ cả file có ở M3".into()
            }
            _ => {}
        }
        false
    }

    fn move_cursor(&mut self, delta: isize) {
        if !self.focus_files {
            self.scroll = self.scroll.saturating_add_signed(delta);
            return;
        }
        let items = self.items();
        self.cursor = self
            .cursor
            .saturating_add_signed(delta)
            .min(items.len().saturating_sub(1));
        if let Some(Item::File(index)) = items.get(self.cursor) {
            self.selected = *index;
            self.load();
            self.sync_cursor();
        }
    }

    fn next_hunk(&mut self, delta: isize) {
        let count = self.diff.hunks.len();
        if delta > 0 && self.hunk + 1 < count {
            self.hunk += 1;
        } else if delta < 0 && self.hunk > 0 {
            self.hunk -= 1;
        } else if let Some(review) = &self.review {
            let count = review.files.len();
            if count == 0 {
                return;
            }
            self.selected = if delta > 0 {
                (self.selected + 1) % count
            } else {
                (self.selected + count - 1) % count
            };
            self.load();
            if delta < 0 {
                self.hunk = self.diff.hunks.len().saturating_sub(1);
            }
        }
        self.scroll = usize::MAX;
    }

    fn prompt_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Down | KeyCode::PageDown => {
                self.prompt_scroll = self.prompt_scroll.saturating_add(1)
            }
            KeyCode::Up | KeyCode::PageUp => {
                self.prompt_scroll = self.prompt_scroll.saturating_sub(1)
            }
            KeyCode::Esc | KeyCode::Char('n') => {
                self.prompt = None;
                self.message = "Đã hủy; snapshot chưa sẵn sàng thì thao tác ghi vẫn bị chặn".into();
            }
            KeyCode::Char('y') => {
                let prompt = self.prompt.take();
                let oversized = matches!(prompt, Some(Prompt::Oversized));
                let result = match prompt {
                    Some(Prompt::Oversized) => {
                        self.review.as_mut().map(|r| r.skip_oversized(self.retain))
                    }
                    Some(Prompt::Undo { token, .. }) => {
                        self.review.as_mut().map(|r| r.undo_checked(&token))
                    }
                    None => None,
                };
                match result {
                    Some(Ok(())) => {
                        self.refresh();
                        self.message = "Đã hoàn tất; index và nhánh giữ nguyên".into();
                    }
                    Some(Err(error)) => {
                        self.message = friendly_error(&error);
                        if oversized {
                            self.prompt = Some(Prompt::Oversized);
                        }
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }

    fn rows(&self, split: bool) -> Vec<Row> {
        let mut rows = Vec::new();
        for (hi, hunk) in self.diff.hunks.iter().enumerate() {
            rows.push(Row {
                hunk: hi,
                old: None,
                new: None,
                header: true,
            });
            let mut at = 0;
            while at < hunk.lines.len() {
                if hunk.lines[at].kind == LineKind::Context {
                    rows.push(Row {
                        hunk: hi,
                        old: Some(at),
                        new: Some(at),
                        header: false,
                    });
                    at += 1;
                    continue;
                }
                let mut removed = Vec::new();
                let mut added = Vec::new();
                while at < hunk.lines.len() && hunk.lines[at].kind != LineKind::Context {
                    if hunk.lines[at].kind == LineKind::Delete {
                        removed.push(at);
                    } else {
                        added.push(at);
                    }
                    at += 1;
                }
                for i in 0..removed.len().max(added.len()) {
                    let old = removed.get(i).copied();
                    let new = added.get(i).copied();
                    if split {
                        rows.push(Row {
                            hunk: hi,
                            old,
                            new,
                            header: false,
                        });
                    } else {
                        if old.is_some() {
                            rows.push(Row {
                                hunk: hi,
                                old,
                                new: None,
                                header: false,
                            });
                        }
                        if new.is_some() {
                            rows.push(Row {
                                hunk: hi,
                                old: None,
                                new,
                                header: false,
                            });
                        }
                    }
                }
            }
        }
        rows
    }

    pub fn draw(
        &mut self,
        renderer: &mut Renderer,
        writer: &mut impl Write,
        tab: usize,
        width: u16,
        height: u16,
        memory: &str,
    ) -> io::Result<()> {
        let layout = Layout::new(width, height, self.show_files, self.show_diff, self.unified);
        let indexes = self.rows(layout.split);
        if self.scroll == usize::MAX {
            self.scroll = indexes
                .iter()
                .position(|r| r.hunk == self.hunk)
                .unwrap_or(0);
        }
        self.scroll = self.scroll.min(indexes.len().saturating_sub(1));
        let visible: Vec<Row> = indexes
            .iter()
            .skip(self.scroll)
            .take(usize::from(height.saturating_sub(4)))
            .copied()
            .collect();
        let spans: Vec<_> = visible
            .iter()
            .map(|row| {
                let h = &self.diff.hunks[row.hunk];
                if h.lines.len() > crate::diff::MAX_BLOCK_LINES {
                    return None;
                }
                match paired_lines(h, row.old, row.new) {
                    (Some(o), Some(n)) if o != n => {
                        crate::diff::word_spans(&h.lines[o].text, &h.lines[n].text)
                    }
                    _ => None,
                }
            })
            .collect();
        let mut lines = Vec::new();
        for (row, spans) in visible.iter().zip(&spans) {
            let h = &self.diff.hunks[row.hunk];
            let cell = |index: usize, old: bool| {
                let line = &h.lines[index];
                DiffCell {
                    number: if old { line.old_line } else { line.new_line }.map(|n| n as usize),
                    text: &line.text,
                    kind: match line.kind {
                        LineKind::Context => DiffKind::Context,
                        LineKind::Add => DiffKind::Added,
                        LineKind::Delete => DiffKind::Removed,
                    },
                    spans: spans
                        .as_ref()
                        .map(|s| {
                            if old {
                                s.old.as_slice()
                            } else {
                                s.new.as_slice()
                            }
                        })
                        .unwrap_or(&[]),
                }
            };
            lines.push(DiffRow {
                old: row.old.map(|i| cell(i, true)),
                new: row.new.map(|i| cell(i, false)),
                header: row.header.then_some(h.header.as_str()),
                selected: row.hunk == self.hunk,
            });
        }
        let mut lists: [Vec<FileEntry>; 3] = std::array::from_fn(|_| Vec::new());
        let mut viewed_files = 0;
        if let Some(review) = &self.review {
            for (index, file) in review.files.iter().enumerate() {
                let viewed = self.completed.contains_key(&file.path);
                if viewed {
                    viewed_files += 1;
                }
                for (g, list) in lists.iter_mut().enumerate() {
                    if in_group(file, g) {
                        list.push(FileEntry {
                            path: &file.display_path,
                            status: file.status,
                            staged: g == 0,
                            selected: index == self.selected,
                            viewed,
                            added: file.added.unwrap_or(0) as usize,
                            removed: file.deleted.unwrap_or(0) as usize,
                        });
                    }
                }
            }
        }
        let selected_group = match self.items().get(self.cursor) {
            Some(Item::Group(g)) => Some(*g),
            _ => None,
        };
        let groups: Vec<_> = ["Staged", "Changes", "Untracked"]
            .iter()
            .enumerate()
            .map(|(i, name)| FileGroup {
                name,
                collapsed: self.collapsed[i],
                files: &lists[i],
                selected: self.focus_files && selected_group == Some(i),
            })
            .collect();
        let count = self.review.as_ref().map(|r| r.files.len()).unwrap_or(0);
        let status = format!("{viewed_files}/{count} file đã xem  {memory}");
        let hunk = format!(
            "hunk {}/{}",
            if self.diff.hunks.is_empty() {
                0
            } else {
                self.hunk + 1
            },
            self.diff.hunks.len()
        );
        let snapshot = format!("{} · so với HEAD", self.snapshot_time);
        let path = self
            .review
            .as_ref()
            .and_then(|r| r.files.get(self.selected))
            .map(|f| f.display_path.as_str())
            .unwrap_or("Không có thay đổi");
        let overlay_lines = self.overlay_lines();
        let title = match self.prompt {
            Some(Prompt::Oversized) => "Snapshot: cần quyết định",
            Some(Prompt::Undo { .. }) => "Khôi phục về lúc mở phiên dark",
            None => "",
        };
        let prompt_start = self
            .prompt_scroll
            .min(overlay_lines.len().saturating_sub(1));
        let end = (prompt_start + usize::from(height.saturating_sub(8))).min(overlay_lines.len());
        let overlay = self.prompt.as_ref().map(|_| Overlay {
            title,
            lines: &overlay_lines[prompt_start..end],
            footer: "y xác nhận · n/Esc hủy · ↑↓ xem danh sách",
        });
        let file = self
            .review
            .as_ref()
            .and_then(|r| r.files.get(self.selected));
        renderer.draw_view(
            writer,
            &View {
                tab,
                project: &self.project,
                branch: &self.branch,
                snapshot: &snapshot,
                status: &status,
                message: &self.message,
                groups: &groups,
                diff_path: path,
                hunk: &hunk,
                lines: &lines,
                show_files: self.show_files,
                show_diff: self.show_diff,
                focus_files: self.focus_files,
                unified: self.unified,
                overlay,
                file_scroll: self
                    .cursor
                    .saturating_sub(crate::ui::layout::file_list_rows(height) - 1),
                diff_added: file.and_then(|f| f.added).unwrap_or(0) as usize,
                diff_removed: file.and_then(|f| f.deleted).unwrap_or(0) as usize,
            },
            width,
            height,
        )
    }

    fn overlay_lines(&self) -> Vec<String> {
        match &self.prompt {
            Some(Prompt::Oversized) => {
                let Some(review) = &self.review else {
                    return Vec::new();
                };
                let mut lines=vec![format!("Có {} file untracked; {} file vượt chính sách snapshot.",review.untracked_count,review.oversized.len()),"y: bỏ qua các file liệt kê; n: hủy tạo snapshot. Không âm thầm thêm chúng vào Git.".into()];
                lines.extend(
                    review
                        .oversized
                        .iter()
                        .map(|f| format!("{} · {} byte", f.path.display(), f.bytes)),
                );
                lines
            }
            Some(Prompt::Undo { paths, .. }) => {
                let mut lines = vec![
                    "u khôi phục về LÚC MỞ PHIÊN, không phải trước lượt agent.".into(),
                    "Bao gồm cả thay đổi bạn tự làm sau khi mở dark. Index và nhánh giữ nguyên."
                        .into(),
                    "File untracked mới ngoài snapshot được giữ lại.".into(),
                    format!("{} file bị ảnh hưởng:", paths.len()),
                ];
                lines.extend(paths.iter().map(|p| p.display().to_string()));
                lines
            }
            None => Vec::new(),
        }
    }
}

fn in_group(file: &crate::git::StatusFile, group: usize) -> bool {
    match group {
        0 => file.staged,
        1 => file.changed && !file.untracked,
        _ => file.untracked,
    }
}
fn hunk_fingerprint(hunk: &crate::review::Hunk) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    hunk.old_start.hash(&mut hash);
    hunk.new_start.hash(&mut hash);
    for line in &hunk.lines {
        line.text.hash(&mut hash);
        (line.kind as u8).hash(&mut hash);
    }
    format!("{:016x}", hash.finish())
}

fn patch_fingerprint(diff: &FileDiff) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    diff.patch.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

fn paired_lines(
    hunk: &crate::review::Hunk,
    old: Option<usize>,
    new: Option<usize>,
) -> (Option<usize>, Option<usize>) {
    if let (Some(o), Some(n)) = (old, new) {
        return (Some(o), Some(n));
    }
    let Some(index) = old.or(new) else {
        return (None, None);
    };
    if hunk.lines[index].kind == LineKind::Context {
        return (old, new);
    }
    let start = (0..index)
        .rev()
        .find(|&i| hunk.lines[i].kind == LineKind::Context)
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = (index..hunk.lines.len())
        .find(|&i| hunk.lines[i].kind == LineKind::Context)
        .unwrap_or(hunk.lines.len());
    let removed: Vec<_> = (start..end)
        .filter(|&i| hunk.lines[i].kind == LineKind::Delete)
        .collect();
    let added: Vec<_> = (start..end)
        .filter(|&i| hunk.lines[i].kind == LineKind::Add)
        .collect();
    let position = if old.is_some() {
        removed.iter().position(|&i| i == index)
    } else {
        added.iter().position(|&i| i == index)
    };
    position
        .map(|i| (removed.get(i).copied(), added.get(i).copied()))
        .unwrap_or((old, new))
}

fn friendly_error(error: &io::Error) -> String {
    let text = error.to_string();
    if text.contains("changed") && (text.contains("confirmation") || text.contains("prepared")) {
        "File đã đổi sau khi mở xác nhận. Không khôi phục; nhấn u để xem lại danh sách.".into()
    } else if text.contains("recovery limit") {
        "Có file untracked vượt giới hạn snapshot. Hãy chuyển file lớn ra ngoài repo hoặc mở lại dark để chọn bỏ qua.".into()
    } else {
        format!("Không thực hiện được; dữ liệu được giữ để kiểm tra: {text}")
    }
}

fn local_clock() -> String {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLocalTime(value: *mut u16);
        }
        // SYSTEMTIME is eight consecutive WORDs: year, month, weekday, day, hour, minute, second, milliseconds.
        let mut time = [0u16; 8];
        unsafe {
            GetLocalTime(time.as_mut_ptr());
        }
        format!("{:02}:{:02}", time[4], time[5])
    }
    #[cfg(not(windows))]
    {
        let directory = std::env::temp_dir().join(format!("dark-clock-{}", std::process::id()));
        let mut command = std::process::Command::new("date");
        command.arg("+%H:%M");
        let value = crate::proc::run(&mut command, &directory, 1024)
            .ok()
            .and_then(|r| std::fs::read_to_string(r.path).ok())
            .unwrap_or_else(|| "n/a".into());
        let _ = std::fs::remove_dir_all(directory);
        value.trim().to_owned()
    }
}
