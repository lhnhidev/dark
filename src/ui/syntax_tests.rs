//! Behavior tests for the diff syntax highlighter (UI_SPEC §2 "Màu cú pháp").
use super::syntax::{highlight, lang_for_path, Lang, State, Token};
use std::ops::Range;

const ALL: [Lang; 16] = [
    Lang::Plain,
    Lang::JsTs,
    Lang::Python,
    Lang::Html,
    Lang::Css,
    Lang::Json,
    Lang::Rust,
    Lang::Go,
    Lang::Java,
    Lang::C,
    Lang::CSharp,
    Lang::Php,
    Lang::Shell,
    Lang::Yaml,
    Lang::Toml,
    Lang::Markdown,
];

type Toks = Vec<(String, Token)>;

fn ranges(lang: Lang, state: &mut State, line: &str) -> Vec<(Range<usize>, Token)> {
    let mut out = Vec::new();
    highlight(lang, state, line, |r, t| out.push((r, t)));
    out
}

/// Highlight one line and assert the range invariants.
fn toks(lang: Lang, state: &mut State, line: &str) -> Toks {
    let mut pos = 0;
    let mut out = Vec::new();
    for (r, t) in ranges(lang, state, line) {
        assert_eq!(r.start, pos, "{lang:?}: gap/overlap in {line:?}");
        assert!(r.start < r.end, "{lang:?}: empty range in {line:?}");
        assert!(
            line.is_char_boundary(r.start) && line.is_char_boundary(r.end),
            "{lang:?}: range {r:?} not on char boundary in {line:?}"
        );
        pos = r.end;
        out.push((line[r].to_string(), t));
    }
    assert_eq!(pos, line.len(), "{lang:?}: line not covered: {line:?}");
    out
}

fn fresh(lang: Lang, line: &str) -> Toks {
    toks(lang, &mut State::default(), line)
}

fn is(all: &Toks, text: &str, tok: Token) -> bool {
    all.iter().any(|(s, t)| s == text && *t == tok)
}

#[track_caller]
fn has(lang: Lang, line: &str, text: &str, tok: Token) {
    let all = fresh(lang, line);
    assert!(is(&all, text, tok), "{text:?} as {tok:?} not in {all:?}");
}

/// `text` (a substring of `line`) must contain only `Token::Text`.
#[track_caller]
fn plain_part(lang: Lang, line: &str, text: &str) {
    let start = line.find(text).unwrap();
    let mut pos = 0;
    for (s, t) in fresh(lang, line) {
        let (a, b) = (pos, pos + s.len());
        pos = b;
        if a < start + text.len() && b > start {
            assert_eq!(t, Token::Text, "{s:?} in {line:?}");
        }
    }
}

#[track_caller]
fn no_token(lang: Lang, line: &str, tok: Token) {
    let all = fresh(lang, line);
    assert!(
        all.iter().all(|(_, t)| *t != tok),
        "unexpected {tok:?} in {all:?}"
    );
}

/// Lines run in order with state carried; returns tokens per line.
fn seq(lang: Lang, lines: &[&str]) -> (Vec<Toks>, State) {
    let mut state = State::default();
    let out = lines.iter().map(|l| toks(lang, &mut state, l)).collect();
    (out, state)
}

fn corpus() -> Vec<String> {
    let mut v: Vec<String> = [
        "",
        " ",
        "   ",
        "\t",
        "\t\t",
        " \t \t ",
        "日本語のコメント",
        "let s = \"日本\\é\"; // ✓ <b>é</b> 3px",
        "😀😀 = '😀' // 😀",
        "e\u{301}x = \"a\u{301}\" # e\u{301}",
        "\u{200d}",
        "\u{3000}## 見出し",
        "\u{3000}[section]",
        "  \u{3000}",
        "key: 値 # コメント",
        "\"unterminated",
        "'unterminated",
        "`unterminated",
        "'",
        "\"",
        "`",
        "\\",
        "\"\\",
        "'\\",
        "\"abc\\",
        "\"\\é",
        "'\\é'",
        "/*",
        "/* unterminated",
        "*/",
        "/**/",
        "/* a */ b /* c",
        "<!--",
        "-->",
        "<!-- a --> <b>",
        "<",
        "<a",
        "<a href=\"x",
        "<a href='",
        "</",
        "</a",
        "<>",
        "<!",
        "<!DOCTYPE",
        "```",
        "~~~",
        "`a`",
        "``",
        "\"\"\"",
        "'''",
        "\"\"\"a\"\"\"b\"\"\"",
        "[",
        "]",
        "[[x]]",
        "[a",
        "[a] # c",
        "= 1",
        "a =",
        ":",
        "key:",
        "key: ",
        "- ",
        "-",
        "- - a: b",
        "#",
        "# ",
        "#!",
        "#include",
        "# include <x>",
        "@",
        "@media",
        "!important",
        "!",
        "0x",
        "0b",
        "1e+",
        "1e+5",
        "1.",
        ".5",
        "-.5px",
        "+1",
        "-",
        "#abc",
        "#ff",
        "100%",
        "a::b",
        "r#\"x\"#",
        "'a",
        "'a'",
        "'é'",
        "'\\n'",
        "<div>{x}</div>",
        "return <",
        "return <A.b-c d=\"1\" />",
        "x = <",
        "x = <a",
        "x = </a",
        "a < b > c",
        "{\"a\": \"b\"}",
        "\"a\" :",
        "\"a\\\" : 1",
        "{ [ ( ) ] }",
        "$",
        "$x",
        "${#x}",
        "a() b() c(",
        "f (",
        "fn",
        "fn (",
        "def ",
        "class ",
        "func",
        "struct",
        "\u{feff}",
        "\0",
        "\r",
        "a\u{0}b",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // Very long lines (~10k chars).
    v.push("a ".repeat(5000));
    v.push("\"x\\\"".repeat(2500));
    v.push("{\"k\": \"v\", ".repeat(900));
    v.push("/* c */ ".repeat(1250));
    v.push("/*".repeat(5000));
    v.push("<a b=\"c\"> ".repeat(1000));
    v.push("`".repeat(10_000));
    v.push("'".repeat(10_001));
    v.push("\"".repeat(10_001));
    v.push("\\".repeat(10_001));
    v.push("日".repeat(3400));
    v.push("é".repeat(5000));
    v.push("😀 a(".repeat(2000));
    v.push("# ".repeat(5000));
    v.push("[a]".repeat(3400));
    v.push("k: v ".repeat(2000));
    v.push("0x".repeat(5000));
    v.push("1e+".repeat(3400));
    v.push(" ".repeat(10_000));
    v.push("\t".repeat(10_000));
    v
}

#[test]
fn every_language_covers_every_tricky_line_without_panicking() {
    let corpus = corpus();
    for lang in ALL {
        // Fresh state per line.
        for line in &corpus {
            toks(lang, &mut State::default(), line);
        }
        // Whole corpus as one stream, state carried from line to line.
        let mut state = State::default();
        let mut reached = vec![State::default()];
        for line in &corpus {
            toks(lang, &mut state, line);
            if !reached.contains(&state) {
                reached.push(state);
            }
        }
        // Every reachable multi-line state followed by every line.
        for start in reached {
            for line in &corpus {
                let mut s = start;
                toks(lang, &mut s, line);
            }
        }
    }
}

#[test]
fn highlighting_is_deterministic_for_the_same_input_and_state() {
    for lang in ALL {
        for line in corpus().iter().take(80) {
            let a = ranges(lang, &mut State::default(), line);
            let b = ranges(lang, &mut State::default(), line);
            assert_eq!(a, b, "{lang:?} {line:?}");
        }
    }
}

#[test]
fn empty_line_emits_nothing_and_keeps_state() {
    for lang in ALL {
        let mut s = State::default();
        assert!(ranges(lang, &mut s, "").is_empty());
        assert_eq!(s, State::default(), "{lang:?}");
    }
}

#[test]
fn state_returns_to_default_after_a_single_line_without_open_constructs() {
    for lang in ALL {
        let mut s = State::default();
        toks(lang, &mut s, "plain words here");
        assert_eq!(s, State::default(), "{lang:?}");
    }
}

#[test]
fn plain_language_is_one_text_token_and_never_changes_state() {
    let mut s = State::default();
    for line in ["let x = 1; // c", "/* open", "```", "<!--", "\"\"\"", "`"] {
        assert_eq!(
            toks(Lang::Plain, &mut s, line),
            vec![(line.to_string(), Token::Text)]
        );
        assert_eq!(s, State::default());
    }
}

// ---------------------------------------------------------------- lang_for_path

#[test]
fn every_spec_extension_maps_to_its_language() {
    let table: &[(&[&str], Lang)] = &[
        (&["js", "jsx", "mjs", "cjs", "ts", "tsx"], Lang::JsTs),
        (&["py"], Lang::Python),
        (&["html", "htm"], Lang::Html),
        (&["css"], Lang::Css),
        (&["json"], Lang::Json),
        (&["rs"], Lang::Rust),
        (&["go"], Lang::Go),
        (&["java"], Lang::Java),
        (&["c", "h", "cpp", "cc", "cxx", "hpp"], Lang::C),
        (&["cs"], Lang::CSharp),
        (&["php"], Lang::Php),
        (&["sh", "bash", "zsh"], Lang::Shell),
        (&["yml", "yaml"], Lang::Yaml),
        (&["toml"], Lang::Toml),
        (&["md", "markdown"], Lang::Markdown),
    ];
    for (exts, lang) in table {
        for ext in *exts {
            for name in [
                format!("file.{ext}"),
                format!("file.{}", ext.to_uppercase()),
                format!("File.{}{}", ext[..1].to_uppercase(), &ext[1..]),
                format!("src/deep/nested/dir/file.{ext}"),
                format!("src\\deep\\nested\\file.{ext}"),
                format!("C:\\proj\\src/mixed\\file.{ext}"),
                format!("a.b.c.{ext}"),
                format!("dir.with.dots/file.{ext}"),
            ] {
                assert_eq!(lang_for_path(&name), *lang, "{name}");
            }
        }
    }
}

#[test]
fn unknown_missing_and_dot_only_names_are_plain() {
    for name in [
        "",
        "Makefile",
        "LICENSE",
        "src/Makefile",
        "src.rs/Makefile",
        "src.rs\\Makefile",
        "dir.js/",
        "notes.txt",
        "data.lock",
        "image.svg",
        "x.kt",
        "x.rb",
        "x.sql",
        "x.swift",
        "archive.tar.gz",
        "file.",
        "file.rsx",
        "file.jsonl",
        "file.r",
        "file.tsc",
        "file.rs.bak",
        ".gitignore",
        ".bashrc",
        ".env",
        ".github/CODEOWNERS",
        "v1.2/README",
    ] {
        assert_eq!(lang_for_path(name), Lang::Plain, "{name:?}");
    }
}

#[test]
fn double_extension_uses_the_last_one() {
    assert_eq!(lang_for_path("types.d.ts"), Lang::JsTs);
    assert_eq!(lang_for_path("page.min.js"), Lang::JsTs);
    assert_eq!(lang_for_path(".eslintrc.json"), Lang::Json);
    assert_eq!(lang_for_path("config.yaml.bak"), Lang::Plain);
    assert_eq!(lang_for_path("Cargo.toml"), Lang::Toml);
    assert_eq!(lang_for_path("README.MD"), Lang::Markdown);
}

// ---------------------------------------------------------------- JS / TS

#[test]
fn js_ts_token_kinds() {
    use Token::*;
    let l = Lang::JsTs;
    has(l, "const x = 42;", "const", Keyword);
    has(l, "export default async function f() {}", "async", Keyword);
    has(l, "const x = 42;", "42", Constant);
    has(l, "x = 0xFF + 1.5e3 + 3.14", "0xFF", Constant);
    has(l, "x = 0xFF + 1.5e3 + 3.14", "1.5e3", Constant);
    has(l, "x = 0xFF + 1.5e3 + 3.14", "3.14", Constant);
    has(l, "foo(1) // hi", "// hi", Comment);
    has(l, "foo(1)", "foo", Function);
    has(l, "function bar(a) {", "bar", Function);
    has(l, "x: Promise<string>", "Promise", Type);
    has(l, "x: Promise<string>", "string", Type);
    has(l, "class Foo extends Bar {", "Foo", Type);
    has(l, "class Foo extends Bar {", "Bar", Type);
    has(l, "x = true; y = null; z = undefined", "true", Constant);
    has(l, "x = true; y = null; z = undefined", "null", Constant);
    has(
        l,
        "x = true; y = null; z = undefined",
        "undefined",
        Constant,
    );
    has(l, "const MAX_SIZE = 1", "MAX_SIZE", Constant);
    plain_part(l, "const my_var = other", "my_var");
    plain_part(l, "x = a + b;", "x = a + b;");
}

#[test]
fn js_ts_string_keeps_going_after_an_escaped_quote() {
    use Token::*;
    let t = fresh(Lang::JsTs, "x = \"a\\\"b\" + 1; // c");
    assert!(is(&t, "\"a", String), "{t:?}");
    assert!(is(&t, "\\\"", Keyword), "escape is a keyword color: {t:?}");
    assert!(is(&t, "b\"", String), "{t:?}");
    assert!(is(&t, "1", Constant), "{t:?}");
    assert!(is(&t, "// c", Comment), "{t:?}");
    // Single quotes and backticks too.
    let t = fresh(Lang::JsTs, "x = 'it\\'s' + 2");
    assert!(is(&t, "s'", String) && is(&t, "2", Constant), "{t:?}");
    let t = fresh(Lang::JsTs, "x = `a\\`b` + 3");
    assert!(is(&t, "b`", String) && is(&t, "3", Constant), "{t:?}");
}

#[test]
fn js_ts_comment_marker_inside_string_is_not_a_comment() {
    no_token(Lang::JsTs, "x = \"// not\" + '/* no */'", Token::Comment);
}

#[test]
fn js_ts_block_comment_spans_lines_and_resumes_code() {
    use Token::*;
    let (t, end) = seq(
        Lang::JsTs,
        &[
            "let a = 1; /* start",
            "return 5; // inside",
            "end */ const b = 2;",
        ],
    );
    assert!(is(&t[0], "let", Keyword) && is(&t[0], "/* start", Comment));
    assert_eq!(t[1], vec![("return 5; // inside".to_string(), Comment)]);
    assert_eq!(t[2][0], ("end */".to_string(), Comment));
    assert!(is(&t[2], "const", Keyword) && is(&t[2], "2", Constant));
    assert_eq!(end, State::default());
}

#[test]
fn js_ts_block_comment_closed_on_same_line_does_not_leak() {
    let (t, end) = seq(Lang::JsTs, &["a /* c */ b", "return 1"]);
    assert!(is(&t[0], "/* c */", Token::Comment));
    assert!(is(&t[1], "return", Token::Keyword));
    assert_eq!(end, State::default());
}

#[test]
fn js_ts_template_literal_spans_lines() {
    use Token::*;
    let (t, end) = seq(
        Lang::JsTs,
        &["const t = `abc", "return 1 // not code", "def` + 1"],
    );
    assert!(is(&t[0], "const", Keyword));
    assert_eq!(t[1], vec![("return 1 // not code".to_string(), String)]);
    assert_eq!(t[2][0], ("def`".to_string(), String));
    assert!(is(&t[2], "1", Constant));
    assert_eq!(end, State::default());
}

#[test]
fn js_ts_jsx_tags_and_attributes() {
    use Token::*;
    let l = Lang::JsTs;
    let t = fresh(l, "return <div className=\"a\"></div>");
    assert_eq!(t.iter().filter(|(s, k)| s == "div" && *k == Tag).count(), 2);
    assert!(is(&t, "className", Constant), "{t:?}");
    assert!(is(&t, "\"a\"", String), "{t:?}");
    has(
        l,
        "  <Button.Primary onClick={go} />",
        "Button.Primary",
        Tag,
    );
    // A comparison is not a tag.
    no_token(l, "if (a < b) {}", Tag);
    no_token(l, "for (i = 0; i < n; i++) {}", Tag);
}

// ---------------------------------------------------------------- Python

#[test]
fn python_token_kinds() {
    use Token::*;
    let l = Lang::Python;
    has(l, "def run(x):", "def", Keyword);
    has(l, "def run(x):", "run", Function);
    has(l, "print(x)", "print", Function);
    has(l, "class foo:", "foo", Type);
    has(l, "class Foo(Base):", "Base", Type);
    has(l, "x: int = 3", "int", Type);
    has(l, "return None", "None", Constant);
    has(l, "x = True or False", "True", Constant);
    has(l, "MAX_RETRIES = 3", "MAX_RETRIES", Constant);
    has(l, "x = 0x1F", "0x1F", Constant);
    has(l, "x = 3.14", "3.14", Constant);
    has(l, "x = 1_000", "1_000", Constant);
    has(l, "if a and not b:", "and", Keyword);
    has(l, "y = 'a' # note", "# note", Comment);
    has(l, "# whole line", "# whole line", Comment);
    plain_part(l, "my_var = other", "my_var");
}

#[test]
fn python_hash_inside_string_is_not_a_comment() {
    no_token(Lang::Python, "x = \"# no\" + '# no'", Token::Comment);
}

#[test]
fn python_string_with_escaped_quote() {
    use Token::*;
    let t = fresh(Lang::Python, "x = 'it\\'s' + 1  # c");
    assert!(is(&t, "s'", String), "{t:?}");
    assert!(is(&t, "1", Constant) && is(&t, "# c", Comment), "{t:?}");
}

#[test]
fn python_triple_quotes_span_lines_for_both_quote_styles() {
    use Token::*;
    for q in ["\"\"\"", "'''"] {
        let (t, end) = seq(
            Lang::Python,
            &[
                &format!("doc = {q}start"),
                "def not_code(): # still string",
                &format!("end{q} + 1"),
            ],
        );
        assert!(!t[0].is_empty());
        assert_eq!(
            t[1],
            vec![("def not_code(): # still string".to_string(), String)]
        );
        assert_eq!(t[2][0], (format!("end{q}"), String));
        assert!(is(&t[2], "1", Constant));
        assert_eq!(end, State::default());
    }
}

#[test]
fn python_triple_quoted_on_one_line_does_not_carry_state() {
    let (t, end) = seq(Lang::Python, &["\"\"\"doc\"\"\"", "return 1"]);
    assert_eq!(t[0], vec![("\"\"\"doc\"\"\"".to_string(), Token::String)]);
    assert!(is(&t[1], "return", Token::Keyword));
    assert_eq!(end, State::default());
}

#[test]
fn python_other_quote_marker_does_not_close_triple() {
    let (t, end) = seq(Lang::Python, &["x = \"\"\"a", "b ''' c", "d\"\"\""]);
    assert_eq!(t[1], vec![("b ''' c".to_string(), Token::String)]);
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- HTML

#[test]
fn html_tags_attributes_strings_and_text() {
    use Token::*;
    let l = Lang::Html;
    let t = fresh(l, "<a href=\"x\">hi</a>");
    assert_eq!(t.iter().filter(|(s, k)| s == "a" && *k == Tag).count(), 2);
    assert!(is(&t, "href", Constant), "{t:?}");
    assert!(is(&t, "\"x\"", String), "{t:?}");
    assert!(
        t.iter().any(|(s, k)| *k == Text && s.contains("hi")),
        "{t:?}"
    );
    has(l, "<input disabled type='text'>", "disabled", Constant);
    has(l, "<input disabled type='text'>", "'text'", String);
    has(l, "<my-element data-id=\"1\">", "my-element", Tag);
    has(l, "<my-element data-id=\"1\">", "data-id", Constant);
    has(l, "<!DOCTYPE html>", "DOCTYPE", Keyword);
    has(l, "<!-- c -->", "<!-- c -->", Comment);
    has(l, "a <!-- c --> b", "<!-- c -->", Comment);
    // Plain text between tags is not highlighted.
    plain_part(l, "<p>if const return 42</p>", "if const return 42");
}

#[test]
fn html_comment_spans_lines() {
    use Token::*;
    let (t, end) = seq(
        Lang::Html,
        &["<!-- start", "<div class=\"x\"> hidden", "end --> <b>t</b>"],
    );
    assert!(is(&t[0], "<!-- start", Comment));
    assert_eq!(
        t[1],
        vec![("<div class=\"x\"> hidden".to_string(), Comment)]
    );
    assert_eq!(t[2][0], ("end -->".to_string(), Comment));
    assert!(is(&t[2], "b", Tag));
    assert_eq!(end, State::default());
}

#[test]
fn html_tag_attributes_continue_on_next_line() {
    use Token::*;
    let (t, end) = seq(Lang::Html, &["<div", "  id=\"a\"", "  hidden>text"]);
    assert!(is(&t[0], "div", Tag));
    assert!(is(&t[1], "id", Constant) && is(&t[1], "\"a\"", String));
    assert!(is(&t[2], "hidden", Constant));
    assert!(t[2].iter().any(|(s, k)| *k == Text && s.contains("text")));
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- CSS

#[test]
fn css_property_vs_selector_and_values() {
    use Token::*;
    let l = Lang::Css;
    has(l, "  color: #fff;", "color", Constant);
    has(l, "  color: #fff;", "#fff", Constant);
    has(l, "  color: #FFAA00CC;", "#FFAA00CC", Constant);
    has(l, "  margin: -10px 0.5em 50%;", "-10px", Constant);
    has(l, "  margin: -10px 0.5em 50%;", "0.5em", Constant);
    has(l, "  margin: -10px 0.5em 50%;", "50%", Constant);
    has(l, "a { color: red }", "color", Constant);
    plain_part(l, "a:hover {", "a:hover");
    plain_part(l, ".card:hover > .title {", ".card:hover > .title");
    plain_part(l, "#main .nav {", "main .nav");
    has(l, "@media screen {", "@media", Keyword);
    has(l, "  color: red !important;", "!important", Keyword);
    has(l, "  background: rgb(0, 0, 0);", "rgb", Function);
    has(l, "content: \"x\\\"y\";", "\"x", String);
    has(l, "content: \"x\\\"y\";", "\\\"", Keyword);
    has(l, "/* c */ a {", "/* c */", Comment);
    // Not a color: hex digits inside an identifier.
    no_token(l, "#feedback {", Constant);
}

#[test]
fn css_block_comment_spans_lines() {
    use Token::*;
    let (t, end) = seq(
        Lang::Css,
        &["a { /* start", "color: red;", "end */ color: blue; }"],
    );
    assert!(is(&t[0], "/* start", Comment));
    assert_eq!(t[1], vec![("color: red;".to_string(), Comment)]);
    assert_eq!(t[2][0], ("end */".to_string(), Comment));
    assert!(is(&t[2], "color", Constant));
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- JSON

#[test]
fn json_key_vs_string_value() {
    use Token::*;
    let l = Lang::Json;
    has(l, "  \"name\": \"dark\",", "\"name\"", Tag);
    has(l, "  \"name\": \"dark\",", "\"dark\"", String);
    has(l, "  \"name\" : \"dark\"", "\"name\"", Tag);
    has(l, "{\"a\": {\"b\": \"c\"}}", "\"a\"", Tag);
    has(l, "{\"a\": {\"b\": \"c\"}}", "\"b\"", Tag);
    has(l, "{\"a\": {\"b\": \"c\"}}", "\"c\"", String);
    has(l, "[\"a\", \"b\"]", "\"a\"", String);
    has(l, "[\"a\", \"b\"]", "\"b\"", String);
    has(l, "  \"a\\\"b\": 1", "\"a\\\"b\"", Tag);
    has(l, "  \"n\": 1.5e3,", "1.5e3", Constant);
    has(l, "  \"n\": -12,", "12", Constant);
    has(l, "  \"ok\": true", "true", Constant);
    has(l, "  \"ok\": false", "false", Constant);
    has(l, "  \"ok\": null", "null", Constant);
    // A value that mentions a keyword is still a string.
    has(l, "  \"k\": \"true\"", "\"true\"", String);
    // Value strings with escapes keep the escape colored as keyword.
    has(l, "  \"k\": \"a\\nb\"", "\\n", Keyword);
}

// ---------------------------------------------------------------- Rust

#[test]
fn rust_token_kinds() {
    use Token::*;
    let l = Lang::Rust;
    has(l, "pub fn main() {", "pub", Keyword);
    has(l, "pub fn main() {", "fn", Keyword);
    has(l, "pub fn main() {", "main", Function);
    has(l, "let s = \"a\";", "\"a\"", String);
    has(l, "let s = \"a\\\"b\";", "\\\"", Keyword);
    has(l, "let s = \"a\\\"b\"; let n = 1;", "1", Constant);
    has(l, "let n = 1_000u32;", "1_000u32", Constant);
    has(l, "let n = 0xFF;", "0xFF", Constant);
    has(l, "let f = 1.5e-3;", "1.5e-3", Constant);
    has(l, "x // c", "// c", Comment);
    has(l, "/// doc", "/// doc", Comment);
    has(l, "println!(\"x\")", "println", Function);
    has(l, "foo(1)", "foo", Function);
    has(l, "let v: Vec<u8>", "Vec", Type);
    has(l, "let v: Vec<u8>", "u8", Type);
    has(l, "struct Foo {", "Foo", Type);
    has(l, "impl Foo {", "Foo", Type);
    has(l, "fn f<'a>(x: &'a str)", "'a", Type);
    has(l, "let c = 'x';", "'x'", String);
    has(l, "const MAX: usize = 1;", "MAX", Constant);
    has(l, "x = true;", "true", Constant);
    has(l, "Self::new()", "Self", Keyword);
    plain_part(l, "let my_var = other;", "my_var");
}

#[test]
fn rust_block_comment_spans_lines() {
    use Token::*;
    let (t, end) = seq(
        Lang::Rust,
        &["let a = 1; /* start", "fn middle() {}", "end */ let b"],
    );
    assert_eq!(t[1], vec![("fn middle() {}".to_string(), Comment)]);
    assert_eq!(t[2][0], ("end */".to_string(), Comment));
    assert!(is(&t[2], "let", Keyword));
    assert_eq!(end, State::default());
}

#[test]
fn rust_comment_marker_inside_string_and_char_is_not_comment() {
    no_token(
        Lang::Rust,
        "let u = \"http://x\"; let c = '/';",
        Token::Comment,
    );
}

// ---------------------------------------------------------------- Go

#[test]
fn go_token_kinds() {
    use Token::*;
    let l = Lang::Go;
    has(l, "package main", "package", Keyword);
    has(l, "func Run(x int) {", "func", Keyword);
    has(l, "func Run(x int) {", "Run", Function);
    has(l, "func Run(x int) {", "int", Type);
    has(l, "type Server struct {", "Server", Type);
    has(l, "x := nil // c", "nil", Constant);
    has(l, "x := nil // c", "// c", Comment);
    has(l, "x := \"a\\\"b\"", "\\\"", Keyword);
    has(l, "x := 0xFF + 10", "0xFF", Constant);
    has(l, "fmt.Println(x)", "Println", Function);
    has(l, "const MaxSize = 1", "const", Keyword);
    has(l, "/* a */ x", "/* a */", Comment);
}

#[test]
fn go_raw_string_spans_lines() {
    use Token::*;
    let (t, end) = seq(
        Lang::Go,
        &["s := `first", "func notCode() // text", "last` + \"x\""],
    );
    assert!(!t[0].is_empty());
    assert_eq!(t[1], vec![("func notCode() // text".to_string(), String)]);
    assert_eq!(t[2][0], ("last`".to_string(), String));
    assert_eq!(end, State::default());
    // A closed raw string on one line leaves no state.
    let (t, end) = seq(Lang::Go, &["s := `raw`", "return 1"]);
    assert!(is(&t[0], "`raw`", String));
    assert!(is(&t[1], "return", Keyword));
    assert_eq!(end, State::default());
}

#[test]
fn go_block_comment_spans_lines() {
    let (t, end) = seq(Lang::Go, &["x /* a", "func b", "c */ y := 1"]);
    assert_eq!(t[1], vec![("func b".to_string(), Token::Comment)]);
    assert_eq!(t[2][0], ("c */".to_string(), Token::Comment));
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- Java / C / C# / PHP

#[test]
fn java_token_kinds() {
    use Token::*;
    let l = Lang::Java;
    has(l, "public class Foo {", "public", Keyword);
    has(l, "public class Foo {", "Foo", Type);
    has(l, "int x = 5;", "int", Type);
    has(l, "int x = 5;", "5", Constant);
    has(l, "String s = \"a\\\"b\";", "String", Type);
    has(l, "String s = \"a\\\"b\";", "\\\"", Keyword);
    has(l, "run(1); // c", "run", Function);
    has(l, "run(1); // c", "// c", Comment);
    has(l, "x = null;", "null", Constant);
    has(l, "static final int MAX_N = 1;", "MAX_N", Constant);
    has(l, "/* a */ x", "/* a */", Comment);
    let (t, end) = seq(l, &["/** doc", " * return me", " */ int y;"]);
    assert_eq!(t[1], vec![(" * return me".to_string(), Token::Comment)]);
    assert!(is(&t[2], "int", Type));
    assert_eq!(end, State::default());
}

#[test]
fn c_family_token_kinds() {
    use Token::*;
    let l = Lang::C;
    has(l, "#include <stdio.h>", "#include", Keyword);
    has(l, "#define MAX 10", "#define", Keyword);
    has(l, "#define MAX 10", "MAX", Constant);
    has(l, "static int x = 0xFF;", "static", Keyword);
    has(l, "static int x = 0xFF;", "int", Type);
    has(l, "static int x = 0xFF;", "0xFF", Constant);
    has(l, "uint8_t b = 1;", "uint8_t", Type);
    has(l, "p = NULL;", "NULL", Constant);
    has(l, "printf(\"hi\\n\");", "printf", Function);
    has(l, "printf(\"hi\\n\");", "\\n", Keyword);
    has(l, "struct Foo {", "Foo", Type);
    has(l, "x = 1; // c", "// c", Comment);
    has(l, "x = 1; /* c */", "/* c */", Comment);
    let (t, end) = seq(l, &["/* a", "int b;", "*/ int c;"]);
    assert_eq!(t[1], vec![("int b;".to_string(), Comment)]);
    assert!(is(&t[2], "int", Type));
    assert_eq!(end, State::default());
}

#[test]
fn csharp_token_kinds() {
    use Token::*;
    let l = Lang::CSharp;
    has(l, "public static void Main()", "public", Keyword);
    has(l, "public static void Main()", "void", Type);
    has(l, "public static void Main()", "Main", Function);
    has(l, "var s = \"a\\\"b\";", "var", Keyword);
    has(l, "var s = \"a\\\"b\";", "\\\"", Keyword);
    has(l, "string s = null;", "string", Type);
    has(l, "string s = null;", "null", Constant);
    has(l, "Run(3); // c", "// c", Comment);
    has(l, "Run(3);", "3", Constant);
    let (t, end) = seq(l, &["/* a", "var b", "*/ var c"]);
    assert_eq!(t[1], vec![("var b".to_string(), Comment)]);
    assert_eq!(end, State::default());
}

#[test]
fn php_token_kinds() {
    use Token::*;
    let l = Lang::Php;
    has(l, "function run($a) {", "function", Keyword);
    has(l, "function run($a) {", "run", Function);
    has(l, "$x = 'a'; # c", "# c", Comment);
    has(l, "$x = 'a'; // c", "// c", Comment);
    has(l, "$x = 12;", "12", Constant);
    has(l, "class Foo {", "Foo", Type);
    has(l, "$x = null;", "null", Constant);
    has(l, "echo \"a\\\"b\";", "echo", Keyword);
    has(l, "echo \"a\\\"b\";", "\\\"", Keyword);
    plain_part(l, "$my_var = $other;", "$my_var");
    let (t, end) = seq(l, &["/* a", "echo 1;", "*/ echo 2;"]);
    assert_eq!(t[1], vec![("echo 1;".to_string(), Comment)]);
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- Shell

#[test]
fn shell_token_kinds() {
    use Token::*;
    let l = Lang::Shell;
    has(l, "if [ -f x ]; then", "if", Keyword);
    has(l, "if [ -f x ]; then", "then", Keyword);
    has(l, "fi", "fi", Keyword);
    has(l, "echo \"hi\" # c", "# c", Comment);
    has(l, "# whole line", "# whole line", Comment);
    has(l, "echo \"hi\"", "\"hi\"", String);
    has(l, "echo 'hi'", "'hi'", String);
    has(l, "echo \"a\\\"b\" x", "\\\"", Keyword);
    has(l, "run() {", "run", Function);
    has(l, "export PATH=1", "export", Keyword);
    // `#` that is not a comment.
    plain_part(l, "echo ${#x}", "${#x}");
    plain_part(l, "echo $# args", "$# args");
    // No block comments in shell.
    no_token(l, "ls /*.txt", Token::Comment);
    let (_, end) = seq(l, &["echo /* open", "echo x"]);
    assert_eq!(end, State::default());
}

// ---------------------------------------------------------------- YAML / TOML

#[test]
fn yaml_keys_values_comments() {
    use Token::*;
    let l = Lang::Yaml;
    has(l, "name: dark", "name", Tag);
    has(l, "  nested_key: value", "nested_key", Tag);
    has(l, "  - port: 8080", "port", Tag);
    has(l, "  - port: 8080", "8080", Constant);
    has(l, "k: \"v\" # c", "\"v\"", String);
    has(l, "k: v # c", "# c", Comment);
    has(l, "# only comment", "# only comment", Comment);
    has(l, "on: true", "true", Constant);
    has(l, "x: null", "null", Constant);
    has(l, "url: http://x.test/a", "url", Tag);
    // A list item without key has no tag.
    no_token(l, "  - just a value", Tag);
    // Keys must not be colored when the colon is inside a value.
    plain_part(l, "- value:no-space", "value:no-space");
}

#[test]
fn toml_sections_keys_values_comments() {
    use Token::*;
    let l = Lang::Toml;
    has(l, "[package]", "[package]", Tag);
    has(l, "[[bin]]", "[[bin]]", Tag);
    has(l, "[dependencies.serde]", "[dependencies.serde]", Tag);
    has(l, "name = \"dark\"", "name", Tag);
    has(l, "name = \"dark\"", "\"dark\"", String);
    has(l, "a.b = 1", "a.b", Tag);
    has(l, "n = 3 # c", "3", Constant);
    has(l, "n = 3 # c", "# c", Comment);
    has(l, "# c", "# c", Comment);
    has(l, "ok = true", "true", Constant);
    has(l, "s = \"a\\\"b\"", "\\\"", Keyword);
    // `=` inside the value is not a key separator.
    let t = fresh(l, "url = \"a=b\"");
    assert!(is(&t, "url", Tag) && !is(&t, "a", Tag), "{t:?}");
}

// ---------------------------------------------------------------- Markdown

#[test]
fn markdown_headings_inline_code_and_fences() {
    use Token::*;
    let l = Lang::Markdown;
    has(l, "# Title", "# Title", Heading);
    has(l, "###### Deep", "###### Deep", Heading);
    has(l, "## Title", "## Title", Heading);
    has(l, "   ## Indented3", "## Indented3", Heading);
    has(l, "#", "#", Heading);
    plain_part(l, "#hashtag", "#hashtag");
    plain_part(l, "####### seven", "####### seven");
    plain_part(l, "    ## code block", "## code block");
    plain_part(l, "text # not heading", "text # not heading");
    has(l, "use `cargo` now", "`cargo`", Constant);
    plain_part(l, "use `unclosed now", "`unclosed now");
    plain_part(l, "**bold** and _it_", "**bold** and _it_");
    // Headings do not highlight code inside.
    let t = fresh(l, "## use `x`");
    assert_eq!(t, vec![("## use `x`".to_string(), Heading)]);
}

#[test]
fn markdown_fence_spans_lines_and_disables_other_rules() {
    use Token::*;
    for fence in ["```", "~~~"] {
        let (t, end) = seq(
            Lang::Markdown,
            &[
                &format!("{fence}rust"),
                "# not heading `x`",
                "let a = 1; // c",
                fence,
                "## real heading",
            ],
        );
        assert_eq!(t[0][0].1, Constant, "fence line {fence}");
        assert_eq!(t[1], vec![("# not heading `x`".to_string(), Text)]);
        assert_eq!(t[2], vec![("let a = 1; // c".to_string(), Text)]);
        assert_eq!(t[3], vec![(fence.to_string(), Constant)]);
        assert_eq!(t[4], vec![("## real heading".to_string(), Heading)]);
        assert_eq!(end, State::default());
    }
}

#[test]
fn markdown_unclosed_fence_keeps_state_for_following_lines() {
    let mut s = State::default();
    toks(Lang::Markdown, &mut s, "```");
    assert_ne!(s, State::default());
    for _ in 0..3 {
        toks(Lang::Markdown, &mut s, "# still inside");
    }
    assert_ne!(s, State::default());
}

// ---------------------------------------------------------------- state independence

#[test]
fn states_for_separate_sides_do_not_influence_each_other() {
    // The caller keeps one State per side: a comment opened on one stream must
    // not change what the other stream produces.
    let mut old = State::default();
    let mut new = State::default();
    toks(Lang::JsTs, &mut old, "/* opened only on old side");
    let after_new = toks(Lang::JsTs, &mut new, "return 1;");
    assert!(is(&after_new, "return", Token::Keyword));
    assert_eq!(new, State::default());
    assert_ne!(old, State::default());
}
