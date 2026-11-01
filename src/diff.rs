use similar::{capture_diff_slices_deadline, Algorithm, DiffTag};
use std::{
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_BLOCK_LINES: usize = 200;
const MAX_LINE_CHARS: usize = 1_000;
const WORD_DIFF_BUDGET: Duration = Duration::from_millis(10);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WordSpans {
    pub old: Vec<Range<usize>>,
    pub new: Vec<Range<usize>>,
}

/// Returns byte ranges at grapheme boundaries, or None to keep line-only colors.
pub fn word_spans(old: &str, new: &str) -> Option<WordSpans> {
    word_spans_before(old, new, Instant::now() + WORD_DIFF_BUDGET)
}

fn word_spans_before(old: &str, new: &str, deadline: Instant) -> Option<WordSpans> {
    if old.chars().take(MAX_LINE_CHARS + 1).count() > MAX_LINE_CHARS
        || new.chars().take(MAX_LINE_CHARS + 1).count() > MAX_LINE_CHARS
        || Instant::now() >= deadline
    {
        return None;
    }
    if old == new {
        return Some(WordSpans::default());
    }
    // Keep separators too: punctuation and whitespace changes matter in code.
    let old_words: Vec<&str> = old.split_word_bounds().collect();
    let new_words: Vec<&str> = new.split_word_bounds().collect();
    let ops =
        capture_diff_slices_deadline(Algorithm::Myers, &old_words, &new_words, Some(deadline));
    if Instant::now() >= deadline {
        return None;
    }
    let mut spans = WordSpans::default();
    for op in ops {
        if op.tag() != DiffTag::Equal {
            add_span(old, &old_words, op.old_range(), &mut spans.old);
            add_span(new, &new_words, op.new_range(), &mut spans.new);
        }
    }
    (Instant::now() < deadline).then_some(spans)
}

fn add_span(text: &str, words: &[&str], range: Range<usize>, spans: &mut Vec<Range<usize>>) {
    if range.is_empty() {
        return;
    }
    let start: usize = words[..range.start].iter().map(|word| word.len()).sum();
    let end = start + words[range].iter().map(|word| word.len()).sum::<usize>();
    // Unicode word and grapheme boundaries can differ; never split a visible glyph.
    let mut expanded = 0..text.len();
    for (index, _) in text.grapheme_indices(true) {
        if index <= start {
            expanded.start = index;
        }
        if index >= end {
            expanded.end = index;
            break;
        }
    }
    if let Some(previous) = spans.last_mut() {
        if expanded.start <= previous.end {
            previous.end = previous.end.max(expanded.end);
            return;
        }
    }
    spans.push(expanded);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted<'a>(text: &'a str, ranges: &[Range<usize>]) -> Vec<&'a str> {
        ranges.iter().map(|range| &text[range.clone()]).collect()
    }

    #[test]
    fn unchanged_line_has_no_highlights() {
        assert_eq!(
            word_spans("let x = 1;", "let x = 1;"),
            Some(WordSpans::default())
        );
    }

    #[test]
    fn highlights_separate_changes_without_unchanged_words() {
        let spans = word_spans(
            "let old = first + keep + end;",
            "let new = last + keep + end;",
        )
        .unwrap();
        assert_eq!(
            highlighted("let old = first + keep + end;", &spans.old),
            ["old", "first"]
        );
        assert_eq!(
            highlighted("let new = last + keep + end;", &spans.new),
            ["new", "last"]
        );
    }

    #[test]
    fn tracks_punctuation_tabs_and_crlf_exactly() {
        let old = "call(a,\tb);\r\n";
        let new = "call(a; b);\r\n";
        let spans = word_spans(old, new).unwrap();
        assert_eq!(highlighted(old, &spans.old), [",\t"]);
        assert_eq!(highlighted(new, &spans.new), ["; "]);
        let spans = word_spans("a\r\n", "a\n").unwrap();
        assert_eq!(highlighted("a\r\n", &spans.old), ["\r\n"]);
        assert_eq!(highlighted("a\n", &spans.new), ["\n"]);
    }

    #[test]
    fn unicode_changes_preserve_combining_and_emoji_graphemes() {
        let old = "Tiếng e\u{301} 👨‍👩‍👧‍👦 👍🏽!";
        let new = "Việt e\u{300} 👨‍👩‍👦 👍🏻!";
        let spans = word_spans(old, new).unwrap();
        for (text, ranges) in [(old, spans.old), (new, spans.new)] {
            let mut boundaries: Vec<usize> = text.grapheme_indices(true).map(|(i, _)| i).collect();
            boundaries.push(text.len());
            assert!(!ranges.is_empty());
            for range in ranges {
                assert!(boundaries.contains(&range.start));
                assert!(boundaries.contains(&range.end));
                assert!(!text[range].is_empty());
            }
        }
    }

    #[test]
    fn highlights_entire_combining_letters_and_emoji_when_only_suffix_changes() {
        for (old, new) in [("e\u{301}", "e\u{300}"), ("👍🏽", "👍🏻"), ("🇻🇳", "🇻🇺")]
        {
            let spans = word_spans(old, new).unwrap();
            assert_eq!(highlighted(old, &spans.old), [old]);
            assert_eq!(highlighted(new, &spans.new), [new]);
        }
    }

    #[test]
    fn handles_empty_sides() {
        let spans = word_spans("", "new").unwrap();
        assert!(spans.old.is_empty());
        assert_eq!(spans.new.len(), 1);
        assert_eq!(spans.new[0], 0..3);
        let spans = word_spans("old", "").unwrap();
        assert_eq!(spans.old.len(), 1);
        assert_eq!(spans.old[0], 0..3);
        assert!(spans.new.is_empty());
    }

    #[test]
    fn long_lines_use_scalar_limit_and_fall_back() {
        let allowed = "ệ".repeat(MAX_LINE_CHARS);
        assert!(word_spans(&allowed, "x").is_some());
        assert!(word_spans(&format!("{allowed}ệ"), "x").is_none());
        assert!(word_spans("x", &"a".repeat(MAX_LINE_CHARS + 1)).is_none());
    }

    #[test]
    fn expired_budget_falls_back_without_partial_spans() {
        assert!(word_spans_before("old", "new", Instant::now()).is_none());
    }
}
