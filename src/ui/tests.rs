//! Behavior tests for the M1 Review layout (UI_SPEC §3.1-3.3, §4.1, §4.2).
use super::layout::{commit_rows, file_list_rows, Layout, TALL_COMMIT_HEIGHT};
use super::renderer::{
    compact_path, make_line, DiffCell, DiffKind, DiffRow, FileEntry, FileGroup, View,
};
use super::theme as t;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

type Rgb = (u8, u8, u8);
const WIDTHS: [u16; 4] = [160, 120, 100, 80];
const HEIGHTS: [u16; 2] = [24, 44];

#[derive(Clone, Copy, Debug)]
struct Cell {
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    bold: bool,
}

/// Parse one rendered row into one cell per display column.
fn parse(raw: &str) -> Vec<Cell> {
    let mut cells = Vec::new();
    let (mut fg, mut bg, mut bold) = (None, None, false);
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            chars.next();
            let mut params = String::new();
            for p in chars.by_ref() {
                if p.is_ascii_alphabetic() {
                    break;
                }
                params.push(p);
            }
            let n: Vec<u16> = params.split(';').filter_map(|s| s.parse().ok()).collect();
            match n.as_slice() {
                [38, 2, r, g, b] => fg = Some((*r as u8, *g as u8, *b as u8)),
                [48, 2, r, g, b] => bg = Some((*r as u8, *g as u8, *b as u8)),
                [0] => {
                    fg = None;
                    bg = None;
                    bold = false;
                }
                [1] => bold = true,
                [22] => bold = false,
                _ => {}
            }
            continue;
        }
        let w = UnicodeWidthChar::width(c).unwrap_or(0);
        if w == 0 {
            continue;
        }
        cells.push(Cell {
            ch: c,
            fg,
            bg,
            bold,
        });
        for _ in 1..w {
            cells.push(Cell {
                ch: '\0',
                fg,
                bg,
                bold,
            });
        }
    }
    cells
}

fn text(cells: &[Cell]) -> String {
    cells
        .iter()
        .filter(|c| c.ch != '\0')
        .map(|c| c.ch)
        .collect()
}

fn find(cells: &[Cell], needle: &str) -> Option<usize> {
    let hay: Vec<char> = cells.iter().map(|c| c.ch).collect();
    let pat: Vec<char> = needle.chars().collect();
    (0..hay.len().saturating_sub(pat.len() - 1)).find(|&i| hay[i..i + pat.len()] == pat[..])
}

fn render(v: &View<'_>, w: u16, h: u16, color: bool) -> Vec<Vec<Cell>> {
    (0..h)
        .map(|row| {
            let mut s = String::new();
            make_line(&mut s, v, w, h, row, (color, true, 8));
            parse(&s)
        })
        .collect()
}

fn raw_row(v: &View<'_>, w: u16, h: u16, row: u16, color: bool) -> String {
    let mut s = String::new();
    make_line(&mut s, v, w, h, row, (color, true, 8));
    s
}

fn entry(path: &str, status: char, added: usize, removed: usize) -> FileEntry<'_> {
    FileEntry {
        path,
        status,
        staged: false,
        selected: false,
        viewed: false,
        added,
        removed,
    }
}

fn cell<'a>(n: usize, text: &'a str, kind: DiffKind) -> Option<DiffCell<'a>> {
    Some(DiffCell {
        number: Some(n),
        text,
        kind,
        spans: &[],
    })
}

fn files_width(w: u16) -> usize {
    Layout::new(w, 24, true, true, false).files
}

fn border_to_space(s: &str) -> String {
    s.chars()
        .map(|c| {
            if "│└─┘┌┐".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect()
}

fn rows_fixture<'a>(
    groups: &'a [FileGroup<'a>],
    lines: &'a [DiffRow<'a>],
    focus_files: bool,
) -> View<'a> {
    View {
        tab: 0,
        project: "proj",
        branch: "main ↑2",
        snapshot: "10:30",
        status: "0/4 file đã xem",
        groups,
        lines,
        diff_path: "src/app.ts",
        diff_added: 3,
        diff_removed: 1,
        hunk: "hunk 1/1",
        show_files: true,
        show_diff: true,
        focus_files,
        ..View::default()
    }
}

#[test]
fn switching_focus_never_shifts_content_columns() {
    let files = [
        entry("a/b.ts", 'M', 12, 3),
        entry("README.md", 'A', 5, 0),
        entry("zz.rs", 'D', 0, 7),
    ];
    let groups = [FileGroup {
        name: "Changes",
        collapsed: false,
        selected: false,
        files: &files,
    }];
    let lines = [
        DiffRow {
            old: None,
            new: None,
            header: Some("@@ -40,3 +40,4 @@ fn foo()"),
            selected: false,
        },
        DiffRow {
            old: cell(41, "code", DiffKind::Removed),
            new: cell(41, "code", DiffKind::Added),
            header: None,
            selected: false,
        },
    ];
    for w in WIDTHS {
        for h in HEIGHTS {
            for color in [true, false] {
                let a = rows_fixture(&groups, &lines, true);
                let b = rows_fixture(&groups, &lines, false);
                for row in 0..h {
                    let ra = parse(&raw_row(&a, w, h, row, color));
                    let rb = parse(&raw_row(&b, w, h, row, color));
                    assert_eq!(ra.len(), rb.len(), "w{w} h{h} row{row}");
                    assert_eq!(
                        border_to_space(&text(&ra)),
                        border_to_space(&text(&rb)),
                        "w{w} h{h} row{row}"
                    );
                }
            }
        }
    }
}

#[test]
fn inactive_pane_has_no_border_and_active_pane_uses_accent() {
    let lines: [DiffRow; 0] = [];
    for w in WIDTHS {
        let f = files_width(w);
        let last = usize::from(w) - 2;
        for focus_files in [true, false] {
            let v = rows_fixture(&[], &lines, focus_files);
            let rows = render(&v, w, 24, true);
            for (row, r) in rows.iter().enumerate().take(16).skip(2) {
                let (left, right, mid) = (r[0], r[last], r[f - 1]);
                assert_eq!(mid.ch, '│');
                assert_eq!(mid.fg, Some(t::ACCENT), "shared column is accent");
                if focus_files {
                    assert_eq!((left.ch, left.fg), ('│', Some(t::ACCENT)), "w{w} r{row}");
                    assert_eq!(right.ch, ' ', "w{w} r{row}");
                } else {
                    assert_eq!(left.ch, ' ', "w{w} r{row}");
                    assert_eq!((right.ch, right.fg), ('│', Some(t::ACCENT)), "w{w} r{row}");
                }
            }
            let bottom = &rows[22];
            if focus_files {
                assert_eq!(bottom[0].ch, '└');
                assert_eq!(bottom[f - 1].ch, '┘');
                assert_eq!(bottom[last].ch, ' ');
            } else {
                assert_eq!(bottom[0].ch, ' ');
                assert_eq!(bottom[f - 1].ch, '└');
                assert_eq!(bottom[last].ch, '┘');
                assert_eq!(bottom[f + 2].ch, '─');
                assert_eq!(bottom[f + 2].fg, Some(t::ACCENT));
            }
        }
    }
}

#[test]
fn title_row_is_panel_without_corners_and_border_colored_divider() {
    let lines: [DiffRow; 0] = [];
    for w in WIDTHS {
        for h in HEIGHTS {
            for focus_files in [true, false] {
                let v = rows_fixture(&[], &lines, focus_files);
                let row = &render(&v, w, h, true)[1];
                let s = text(row);
                assert!(!s.contains(['┌', '┐', '└', '┘', '─']), "{s}");
                assert!(row.iter().all(|c| c.bg == Some(t::PANEL)), "w{w}: {s}");
                let mid = row[files_width(w) - 1];
                assert_eq!((mid.ch, mid.fg), ('│', Some(t::BORDER)));
                assert_eq!(row.len(), usize::from(w) - 1);
            }
        }
    }
}

#[test]
fn file_rows_color_path_parts_and_counts() {
    let files = [
        entry("a/b.ts", 'M', 12, 3),
        entry("x/add.rs", 'A', 5, 0),
        entry("x/del.rs", 'D', 0, 7),
        entry("x/none.rs", 'M', 0, 0),
    ];
    let groups = [FileGroup {
        name: "Changes",
        collapsed: false,
        selected: false,
        files: &files,
    }];
    for w in WIDTHS {
        let f = files_width(w);
        let v = rows_fixture(&groups, &[], true);
        let rows = render(&v, w, 24, true);
        // Row 2 is the group header; files follow.
        let both = &rows[3];
        let d = find(both, "a/").expect("directory shown");
        assert_eq!(both[d].fg, Some(t::LINE_NUMBER));
        assert_eq!(both[d + 1].fg, Some(t::LINE_NUMBER));
        let b = find(both, "b.ts").expect("basename shown");
        assert_eq!(both[b].fg, Some(t::TEXT));
        let p = find(both, "+12").unwrap();
        assert!((0..3).all(|i| both[p + i].fg == Some(t::FILE_ADDED)));
        let m = find(both, "−3").unwrap();
        assert!((0..2).all(|i| both[m + i].fg == Some(t::FILE_DELETED)));
        assert!(m + 2 < f, "counts stay left of the pane border");
        assert!(m + 2 >= f - 3, "counts are right aligned");

        let add = text(&rows[4]);
        assert!(add.contains("+5") && !add[..].contains('−'), "{add}");
        let del = text(&rows[5]);
        assert!(del.contains("−7") && !del.contains("+7"), "{del}");
        let none = text(&rows[6][..f - 1]);
        assert!(!none.contains('+') && !none.contains('−'), "{none}");
        let pa = find(&rows[4], "+5").unwrap();
        assert_eq!(rows[4][pa].fg, Some(t::FILE_ADDED));
        let pd = find(&rows[5], "−7").unwrap();
        assert_eq!(rows[5][pd].fg, Some(t::FILE_DELETED));
    }
}

#[test]
fn compact_path_shrinks_directory_then_drops_it_and_keeps_basename() {
    let p = "src/auth/session.ts";
    assert_eq!(compact_path(p, 19), p);
    assert_eq!(compact_path(p, 18), "…/auth/session.ts");
    assert_eq!(compact_path(p, 17), "…/auth/session.ts");
    assert_eq!(compact_path(p, 16), "session.ts");
    assert_eq!(compact_path(p, 10), "session.ts");
    // Long basename: cut in the middle, head and tail both visible.
    let long = "src/very_long_filename_component.ts";
    let out = compact_path(long, 12);
    assert!(UnicodeWidthStr::width(out.as_str()) <= 12, "{out}");
    assert!(out.contains('…') && !out.starts_with('…'), "{out}");
    assert!(out.ends_with(".ts"), "{out}");
    assert!(out.starts_with("ve"), "head is about a quarter: {out}");
}

#[test]
fn narrow_numbered_names_stay_distinct_with_visible_suffix() {
    let outs: Vec<String> = ["src/module_01.ts", "src/module_11.ts", "src/module_21.ts"]
        .iter()
        .map(|p| compact_path(p, 7))
        .collect();
    for o in &outs {
        assert_eq!(UnicodeWidthStr::width(o.as_str()), 7, "{o}");
        assert!(o.ends_with("1.ts") && o.contains('…'), "{o}");
        assert!(!o.starts_with('…'), "{o}");
    }
    assert_eq!(outs[0], "m…01.ts");
    assert!(outs[0] != outs[1] && outs[1] != outs[2] && outs[0] != outs[2]);
}

#[test]
fn compact_path_respects_display_width_for_wide_graphemes() {
    for path in [
        "src/界界界界界界.rs",
        "界/界界/文件文件文件文件文件.md",
        "a/e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}.rs",
        "👩‍💻/👩‍💻👩‍💻👩‍💻👩‍💻.rs",
    ] {
        for available in 0..=30 {
            let out = compact_path(path, available);
            assert!(
                UnicodeWidthStr::width(out.as_str()) <= available,
                "{path} @ {available} -> {out}"
            );
        }
    }
}

#[test]
fn rendered_rows_have_exact_width_with_long_and_wide_paths_at_80() {
    let files = [
        entry("src/components/very_long_filename_component.ts", 'M', 0, 0),
        entry("src/界界界界界界界界界界界界界界.rs", 'M', 4, 4),
        entry("src/module_02.ts", 'M', 50, 50),
    ];
    let groups = [FileGroup {
        name: "Changes",
        collapsed: false,
        selected: false,
        files: &files,
    }];
    for w in WIDTHS {
        for focus_files in [true, false] {
            let v = rows_fixture(&groups, &[], focus_files);
            let rows = render(&v, w, 24, true);
            for (i, r) in rows.iter().enumerate() {
                assert_eq!(r.len(), usize::from(w) - 1, "w{w} row{i}: {}", text(r));
            }
        }
    }
    let v = rows_fixture(&groups, &[], true);
    let rows = render(&v, 80, 24, true);
    let long = text(&rows[3]);
    let f = files_width(80);
    let label = compact_path("src/components/very_long_filename_component.ts", f - 2 - 7);
    assert!(long.contains(&label), "{long} / {label}");
    assert!(long.contains('…') && !long.contains("…ent.ts"), "{long}");
    // Disambiguating suffix of similar names stays visible.
    assert!(text(&rows[5]).contains('…') || text(&rows[5]).contains("module_02"));
}

fn diff_rows_text(unified: bool, w: u16) -> Vec<String> {
    let lines = [
        DiffRow {
            old: cell(41, "code", DiffKind::Removed),
            new: None,
            header: None,
            selected: false,
        },
        DiffRow {
            old: None,
            new: cell(41, "code", DiffKind::Added),
            header: None,
            selected: false,
        },
    ];
    let mut v = rows_fixture(&[], &lines, false);
    v.unified = unified;
    render(&v, w, 24, true).iter().map(|r| text(r)).collect()
}

#[test]
fn diff_rows_use_number_sign_text_format_in_unified_and_split() {
    for w in WIDTHS {
        for unified in [false, true] {
            let rows = diff_rows_text(unified, w);
            assert!(
                rows[2].contains("   41 − code"),
                "w{w} u{unified}: {}",
                rows[2]
            );
            assert!(
                rows[3].contains("   41 + code"),
                "w{w} u{unified}: {}",
                rows[3]
            );
        }
    }
    // Split mode really is used at 160 and has two columns on a paired row.
    let paired = [DiffRow {
        old: cell(41, "old", DiffKind::Removed),
        new: cell(41, "new", DiffKind::Added),
        header: None,
        selected: false,
    }];
    let v = rows_fixture(&[], &paired, false);
    let r = text(&render(&v, 160, 24, true)[2]);
    assert!(
        r.contains("   41 − old") && r.contains("   41 + new"),
        "{r}"
    );
}

#[test]
fn hunk_header_keeps_function_context() {
    let lines = [DiffRow {
        old: None,
        new: None,
        header: Some("@@ -40,3 +40,4 @@ function foo() {"),
        selected: false,
    }];
    for w in WIDTHS {
        let v = rows_fixture(&[], &lines, false);
        let r = render(&v, w, 24, true);
        let s = text(&r[2]);
        assert!(
            s.contains("@@ -40,3 +40,4 @@ function foo() {"),
            "w{w}: {s}"
        );
        let i = find(&r[2], "@@").unwrap();
        assert_eq!(r[2][i].fg, Some(t::MUTED));
        assert_eq!(r[2][i].bg, Some(t::PANEL));
    }
}

#[test]
fn tab_bar_right_info_is_flush_right_and_tab_colors_match_spec() {
    for w in [160u16, 120, 100] {
        let v = rows_fixture(&[], &[], true);
        let row = &render(&v, w, 24, true)[0];
        let s = text(row);
        let info = "proj · ⎇ main ↑2 · snapshot 10:30";
        if w >= 120 {
            assert!(s.ends_with(info), "w{w}: {s:?}");
        } else {
            // Not enough room: info is clipped, never overlaps the tabs.
            assert!(s.contains("7 Help") && s.contains("proj"), "w{w}: {s:?}");
        }
        assert_eq!(row.len(), usize::from(w) - 1);
        let sel = find(row, "Review").unwrap();
        assert_eq!(row[sel].bg, Some(t::ACCENT_MUTED));
        assert!(row[sel].bold);
        assert_eq!(row[sel].fg, Some(t::TEXT));
        let num = find(row, "1").unwrap();
        assert_eq!(row[num].fg, Some(t::FILE_MODIFIED));
        let other = find(row, "Edit").unwrap();
        assert_eq!(row[other].bg, Some(t::PANEL));
        assert_eq!(row[other].fg, Some(t::MUTED));
        let brand = find(row, "dark").unwrap();
        assert_eq!(row[brand].fg, Some(t::ACCENT));
        assert!(row[brand].bold);
        let m = find(row, "proj").unwrap();
        assert_eq!(row[m].fg, Some(t::MUTED));
    }
    // Narrow: short tab labels and no right info.
    let v = rows_fixture(&[], &[], true);
    let s = text(&render(&v, 80, 24, true)[0]);
    assert!(s.contains("Rev") && !s.contains("snapshot"), "{s}");
}

#[test]
fn status_bar_keys_are_boxed_on_panel_alt_and_n_slash_n_is_two_boxes() {
    for w in WIDTHS {
        let mut v = rows_fixture(&[], &[], true);
        v.status = "";
        let row = &render(&v, w, 24, true)[23];
        assert!(row.iter().all(|c| c.bg.is_some()));
        let a = find(row, "a accept").unwrap();
        assert_eq!(row[a].bg, Some(t::PANEL_ALT));
        assert_eq!(row[a].fg, Some(t::TEXT));
        assert_eq!(row[a + 1].bg, Some(t::PANEL));
        let n = find(row, "n/N hunk").unwrap();
        assert_eq!(row[n].bg, Some(t::PANEL_ALT));
        assert_eq!((row[n + 1].ch, row[n + 1].fg), ('/', Some(t::MUTED)));
        assert_eq!(row[n + 1].bg, Some(t::PANEL));
        assert_eq!(row[n + 2].bg, Some(t::PANEL_ALT));
        let mode = find(row, "REVIEW").unwrap();
        assert_eq!(row[mode].fg, Some(t::TEXT));
        assert_eq!(row[mode].bg, Some(t::PANEL));
    }
}

#[test]
fn commit_area_is_disabled_and_consistent_with_file_list_rows() {
    let owned: Vec<String> = (0..60).map(|i| format!("f{i}.rs")).collect();
    let files: Vec<FileEntry> = owned.iter().map(|p| entry(p, 'M', 1, 1)).collect();
    let groups = [FileGroup {
        name: "Changes",
        collapsed: false,
        selected: false,
        files: &files,
    }];
    for w in WIDTHS {
        let f = files_width(w);
        for h in [24u16, 31, 32, 44] {
            for focus_files in [true, false] {
                let v = rows_fixture(&groups, &[], focus_files);
                let rows = render(&v, w, h, true);
                let sep = (2..usize::from(h) - 2)
                    .find(|&r| rows[r][1].ch == '─')
                    .expect("separator row");
                assert_eq!(sep, 2 + file_list_rows(h), "w{w} h{h}");
                assert_eq!(usize::from(h) - 2 - sep, usize::from(commit_rows(h)));
                assert!(
                    text(&rows[sep - 1]).contains(".rs"),
                    "last list row is a file: {}",
                    text(&rows[sep - 1])
                );
                for c in &rows[sep][1..f - 1] {
                    assert_eq!((c.ch, c.fg), ('─', Some(t::BORDER)));
                }
                assert!(text(&rows[sep + 1]).contains("Commit message"));
                let tall = h >= TALL_COMMIT_HEIGHT;
                let area = &rows[sep..usize::from(h) - 2];
                let joined: String = area.iter().map(|r| text(r)).collect();
                assert_eq!(joined.contains('┌'), tall, "h{h}");
                assert_eq!(joined.contains("[ Có ở M3 ]"), !tall, "h{h}");
                assert!(joined.contains("Space stage · c commit"));
                for r in area {
                    for c in &r[1..f - 1] {
                        assert_ne!(c.bg, Some(t::PANEL_ALT));
                        if c.ch != ' ' {
                            assert!(
                                c.fg == Some(t::LINE_NUMBER) || c.fg == Some(t::BORDER),
                                "w{w} h{h} {c:?}"
                            );
                        }
                    }
                }
                let hint = find(&rows[usize::from(h) - 3], "Space stage").unwrap();
                assert_eq!(rows[usize::from(h) - 3][hint].fg, Some(t::LINE_NUMBER));
                let label = find(&rows[sep + 1], "Commit message").unwrap();
                assert_eq!(rows[sep + 1][label].fg, Some(t::LINE_NUMBER));
                if tall {
                    assert!(text(&rows[sep + 3]).contains("Có ở M3"));
                } else {
                    assert!(text(&rows[sep + 2]).contains("[ Có ở M3 ]"));
                }
            }
        }
    }
}

#[test]
fn no_color_keeps_active_pane_and_selection_recognizable() {
    let mut files = [entry("a/b.ts", 'M', 1, 1), entry("c.rs", 'M', 1, 1)];
    files[0].selected = true;
    let groups = [FileGroup {
        name: "Changes",
        collapsed: false,
        selected: false,
        files: &files,
    }];
    for w in WIDTHS {
        let last = usize::from(w) - 2;
        for focus_files in [true, false] {
            let v = rows_fixture(&groups, &[], focus_files);
            let raw = raw_row(&v, w, 24, 3, false);
            assert!(!raw.contains("38;") && !raw.contains("48;"), "{raw:?}");
            let r = parse(&raw);
            assert_eq!(r[0].ch == '│', focus_files, "w{w}");
            assert_eq!(r[last].ch == '│', !focus_files, "w{w}");
            assert_eq!(r[1].ch, '▌', "selected row marker");
            assert_ne!(parse(&raw_row(&v, w, 24, 4, false))[1].ch, '▌');
            let bottom = parse(&raw_row(&v, w, 24, 22, false));
            assert_eq!(bottom[0].ch == '└', focus_files);
            assert_eq!(bottom[last].ch == '┘', !focus_files);
        }
    }
}
