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

pub fn color(rgb: (u8, u8, u8), truecolor: bool) -> Color {
    if truecolor {
        return Color::Rgb {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
        };
    }
    let mut best = (u32::MAX, 0);
    for index in 16u8..=255 {
        let (r, g, b) = if index < 232 {
            let n = index - 16;
            let component = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (component(n / 36), component(n / 6 % 6), component(n % 6))
        } else {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        };
        let distance = (i32::from(r) - i32::from(rgb.0)).pow(2)
            + (i32::from(g) - i32::from(rgb.1)).pow(2)
            + (i32::from(b) - i32::from(rgb.2)).pow(2);
        if (distance as u32) < best.0 {
            best = (distance as u32, index);
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
}
