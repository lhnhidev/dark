/// Terminal height from which the commit input is drawn as a 3-row box.
pub const TALL_COMMIT_HEIGHT: u16 = 32;

/// Rows used by the disabled commit area: separator, label, input, hint.
pub fn commit_rows(height: u16) -> u16 {
    if height >= TALL_COMMIT_HEIGHT {
        6
    } else {
        4
    }
}

/// Rows available for the file list: below the title row, above the commit
/// area and the bottom border row.
pub fn file_list_rows(height: u16) -> usize {
    usize::from(height.saturating_sub(4 + commit_rows(height))).max(1)
}

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
        Self::with_file_width(width, height, show_files, show_diff, unified, None)
    }

    pub fn with_file_width(
        width: u16,
        height: u16,
        show_files: bool,
        show_diff: bool,
        unified: bool,
        file_width: Option<u16>,
    ) -> Self {
        let usable = usize::from(width.saturating_sub(1));
        let files = if show_files {
            if show_diff {
                file_width.map_or_else(
                    || (usize::from(width) / 5).clamp(24, 40).min(usable),
                    |requested| {
                        usize::from(requested)
                            .clamp(24, usable.saturating_sub(40).max(24))
                            .min(usable)
                    },
                )
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
    #[test]
    fn custom_width_clamps_and_split_tracks_available_diff() {
        let layout = |width, requested| {
            Layout::with_file_width(width, 24, true, true, false, Some(requested))
        };
        assert_eq!((layout(100, 1).files, layout(100, 1).diff), (24, 75));
        assert_eq!((layout(100, 200).files, layout(100, 200).diff), (59, 40));
        assert!(layout(200, 40).split);
        assert!(!layout(200, 100).split);
        assert_eq!(
            Layout::with_file_width(100, 24, true, false, false, Some(30)).files,
            99
        );
    }
}
