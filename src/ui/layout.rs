#[derive(Debug, PartialEq)]
pub struct Layout {
    pub small: bool,
    pub short_tabs: bool,
    pub files: usize,
    pub diff: usize,
    pub split: bool,
}
impl Layout {
    pub fn new(width: u16, height: u16, show_files: bool, show_diff: bool, unified: bool) -> Self {
        let usable = usize::from(width.saturating_sub(1));
        let files = if show_files {
            if show_diff {
                (usize::from(width) / 5).clamp(24, 40).min(usable)
            } else {
                usable
            }
        } else {
            0
        };
        let diff = if show_diff {
            usable.saturating_sub(files)
        } else {
            0
        };
        Self {
            small: width < 80 || height < 24,
            short_tabs: width < 100,
            files,
            diff,
            split: diff >= 120 && !unified,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widths_follow_spec() {
        for (w, f, split) in [
            (160, 32, true),
            (120, 24, false),
            (100, 24, false),
            (80, 24, false),
        ] {
            let l = Layout::new(w, 24, true, true, false);
            assert_eq!((l.files, l.split), (f, split));
        }
        assert!(Layout::new(79, 24, true, true, false).small);
        assert!(Layout::new(80, 23, true, true, false).small);
        assert!(Layout::new(99, 24, true, true, false).short_tabs);
    }
}
