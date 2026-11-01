use crossterm::style::Color;

pub const BACKGROUND: (u8, u8, u8) = (0x0d, 0x11, 0x17);
pub const PANEL: (u8, u8, u8) = (0x1e, 0x23, 0x29);
pub const PANEL_ALT: (u8, u8, u8) = (0x27, 0x2b, 0x31);
pub const BORDER: (u8, u8, u8) = (0x34, 0x39, 0x3f);
pub const TEXT: (u8, u8, u8) = (0xe6, 0xed, 0xf3);
pub const MUTED: (u8, u8, u8) = (0xad, 0xae, 0xb1);
pub const LINE_NUMBER: (u8, u8, u8) = (0x87, 0x8c, 0x92);
pub const ACCENT: (u8, u8, u8) = (0xbb, 0x80, 0x09);
pub const ACCENT_MUTED: (u8, u8, u8) = (0x39, 0x2d, 0x14);
pub const ADDED_BG: (u8, u8, u8) = (0x12, 0x25, 0x1d);
pub const ADDED_WORD_BG: (u8, u8, u8) = (0x16, 0x39, 0x23);
pub const ADDED_SIGN: (u8, u8, u8) = (0x2e, 0xa0, 0x43);
pub const REMOVED_BG: (u8, u8, u8) = (0x3c, 0x1e, 0x21);
pub const REMOVED_WORD_BG: (u8, u8, u8) = (0x4f, 0x23, 0x25);
pub const REMOVED_SIGN: (u8, u8, u8) = (0xf8, 0x51, 0x49);
pub const FILE_ADDED: (u8, u8, u8) = (0x77, 0xc1, 0x85);
pub const FILE_DELETED: (u8, u8, u8) = (0xfa, 0x8e, 0x89);
pub const FILE_MODIFIED: (u8, u8, u8) = (0xd3, 0xac, 0x5f);

/// Minimum max-min channel spread for a color to keep its hue in the 256 fallback.
const CHROMATIC_SPREAD: u8 = 16;
/// Weight of the hue mismatch against RGB distance when picking a chromatic cube entry.
const HUE_WEIGHT: f32 = 12000.0;

/// Unit vector of the channel deviations from the channel mean (zero for grays).
fn hue_direction((r, g, b): (u8, u8, u8)) -> (f32, f32, f32) {
    let (r, g, b) = (f32::from(r), f32::from(g), f32::from(b));
    let mean = (r + g + b) / 3.0;
    let (dr, dg, db) = (r - mean, g - mean, b - mean);
    let norm = (dr * dr + dg * dg + db * db).sqrt();
    if norm == 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        (dr / norm, dg / norm, db / norm)
    }
}

/// Decide from environment values whether 24-bit color can be used.
pub fn detect_truecolor(
    colorterm: Option<&str>,
    term: Option<&str>,
    wt_session: bool,
    term_program: Option<&str>,
    is_windows: bool,
) -> bool {
    is_windows
        || colorterm
            .is_some_and(|s| s.eq_ignore_ascii_case("truecolor") || s.eq_ignore_ascii_case("24bit"))
        || term
            .is_some_and(|s| s.contains("truecolor") || s.contains("24bit") || s.contains("direct"))
        || wt_session
        || term_program.is_some_and(|s| matches!(s, "iTerm.app" | "WezTerm" | "vscode" | "ghostty"))
}

fn xterm_rgb(index: u8) -> (u8, u8, u8) {
    if index < 232 {
        let n = index - 16;
        let component = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
        (component(n / 36), component(n / 6 % 6), component(n % 6))
    } else {
        let v = 8 + (index - 232) * 10;
        (v, v, v)
    }
}

pub fn color(rgb: (u8, u8, u8), truecolor: bool) -> Color {
    if truecolor {
        return Color::Rgb {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
        };
    }
    let chromatic = rgb.0.max(rgb.1).max(rgb.2) - rgb.0.min(rgb.1).min(rgb.2) >= CHROMATIC_SPREAD;
    let source_hue = hue_direction(rgb);
    let mut best = (f32::MAX, 0);
    for index in 16u8..=255 {
        let (r, g, b) = xterm_rgb(index);
        let is_gray = r == g && g == b;
        if chromatic && is_gray {
            continue;
        }
        let distance = (i32::from(r) - i32::from(rgb.0)).pow(2)
            + (i32::from(g) - i32::from(rgb.1)).pow(2)
            + (i32::from(b) - i32::from(rgb.2)).pow(2);
        let mut score = distance as f32;
        if chromatic {
            let d = hue_direction((r, g, b));
            let hue_error = (d.0 - source_hue.0).powi(2)
                + (d.1 - source_hue.1).powi(2)
                + (d.2 - source_hue.2).powi(2);
            score += HUE_WEIGHT * hue_error;
        }
        if score < best.0 {
            best = (score, index);
        }
    }
    Color::AnsiValue(best.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_matches_specification() {
        assert_eq!(
            [
                BACKGROUND,
                PANEL,
                PANEL_ALT,
                BORDER,
                TEXT,
                MUTED,
                LINE_NUMBER,
                ACCENT,
                ACCENT_MUTED,
                ADDED_BG,
                ADDED_WORD_BG,
                ADDED_SIGN,
                REMOVED_BG,
                REMOVED_WORD_BG,
                REMOVED_SIGN,
                FILE_ADDED,
                FILE_DELETED,
                FILE_MODIFIED
            ],
            [
                (13, 17, 23),
                (30, 35, 41),
                (39, 43, 49),
                (52, 57, 63),
                (230, 237, 243),
                (173, 174, 177),
                (135, 140, 146),
                (187, 128, 9),
                (57, 45, 20),
                (18, 37, 29),
                (22, 57, 35),
                (46, 160, 67),
                (60, 30, 33),
                (79, 35, 37),
                (248, 81, 73),
                (119, 193, 133),
                (250, 142, 137),
                (211, 172, 95)
            ]
        );
        assert_eq!(color((0, 0, 0), false), Color::AnsiValue(16));
    }

    #[test]
    fn truecolor_detection_truth_table() {
        assert!(!detect_truecolor(None, None, false, None, false));
        assert!(!detect_truecolor(
            Some("yes"),
            Some("xterm-256color"),
            false,
            Some("Apple_Terminal"),
            false
        ));
        assert!(detect_truecolor(None, None, false, None, true));
        assert!(detect_truecolor(
            Some("truecolor"),
            None,
            false,
            None,
            false
        ));
        assert!(detect_truecolor(
            Some("TrueColor"),
            None,
            false,
            None,
            false
        ));
        assert!(detect_truecolor(Some("24bit"), None, false, None, false));
        assert!(detect_truecolor(
            None,
            Some("xterm-truecolor"),
            false,
            None,
            false
        ));
        assert!(detect_truecolor(
            None,
            Some("foot-24bit"),
            false,
            None,
            false
        ));
        assert!(detect_truecolor(
            None,
            Some("xterm-direct"),
            false,
            None,
            false
        ));
        assert!(detect_truecolor(None, None, true, None, false));
        for program in ["iTerm.app", "WezTerm", "vscode", "ghostty"] {
            assert!(detect_truecolor(None, None, false, Some(program), false));
        }
    }

    fn fallback_rgb(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
        match color(rgb, false) {
            Color::AnsiValue(i) => xterm_rgb(i),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn fallback_keeps_hue_of_chromatic_colors() {
        for rgb in [ADDED_BG, ADDED_WORD_BG, FILE_ADDED, ADDED_SIGN] {
            let (r, g, b) = fallback_rgb(rgb);
            assert!(g > r && g > b, "{rgb:?} -> {:?}", (r, g, b));
        }
        for rgb in [REMOVED_BG, REMOVED_WORD_BG, FILE_DELETED, REMOVED_SIGN] {
            let (r, g, b) = fallback_rgb(rgb);
            assert!(r > g && r > b, "{rgb:?} -> {:?}", (r, g, b));
        }
        for rgb in [ACCENT_MUTED, ACCENT] {
            let (r, g, b) = fallback_rgb(rgb);
            assert!(r > b && g > b && r >= g, "{rgb:?} -> {:?}", (r, g, b));
        }
    }

    #[test]
    fn fallback_keeps_neutral_colors_gray() {
        for rgb in [BACKGROUND, PANEL, TEXT] {
            let (r, g, b) = fallback_rgb(rgb);
            assert!(r == g && g == b, "{rgb:?} -> {:?}", (r, g, b));
        }
    }
}
