//! Tiny hand-written syntax highlighter for the Review diff.
//!
//! It works line by line: `highlight` emits contiguous byte ranges that cover the
//! whole line. Only a one-byte `State` crosses lines (block comments, triple-quoted
//! strings, fenced code, HTML comments). It is deliberately approximate.

use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Plain,
    JsTs,
    Python,
    Html,
    Css,
    Json,
    Rust,
    Go,
    Java,
    C,
    CSharp,
    Php,
    Shell,
    Yaml,
    Toml,
    Markdown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Text,
    Keyword,
    String,
    Comment,
    Function,
    Constant,
    Type,
    Tag,
    /// Markdown heading: constant color, bold.
    Heading,
}

/// Multi-line state carried from the end of one line to the start of the next.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct State(u8);

const BLOCK: u8 = 1;
const TRIPLE_SINGLE: u8 = 2;
const TRIPLE_DOUBLE: u8 = 3;
const HTML_COMMENT: u8 = 4;
const FENCE: u8 = 5;
const BACKTICK: u8 = 6;
const HTML_TAG: u8 = 7;

pub fn lang_for_path(path: &str) -> Lang {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let Some((_, ext)) = name.rsplit_once('.') else {
        return Lang::Plain;
    };
    match ext.to_ascii_lowercase().as_str() {
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => Lang::JsTs,
        "py" => Lang::Python,
        "html" | "htm" => Lang::Html,
        "css" => Lang::Css,
        "json" => Lang::Json,
        "rs" => Lang::Rust,
        "go" => Lang::Go,
        "java" => Lang::Java,
        "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" => Lang::C,
        "cs" => Lang::CSharp,
        "php" => Lang::Php,
        "sh" | "bash" | "zsh" => Lang::Shell,
        "yml" | "yaml" => Lang::Yaml,
        "toml" => Lang::Toml,
        "md" | "markdown" => Lang::Markdown,
        _ => Lang::Plain,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keys {
    None,
    Json,
    Yaml,
    Toml,
}

struct Cfg {
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    line: &'static [&'static str],
    quotes: &'static [u8],
    block: bool,
    backtick_multi: bool,
    triple: bool,
    cap_type: bool,
    calls: bool,
    jsx: bool,
    preproc: bool,
    rust: bool,
    keys: Keys,
}

const BASE: Cfg = Cfg {
    keywords: &[],
    types: &[],
    line: &["//"],
    quotes: b"\"'",
    block: true,
    backtick_multi: false,
    triple: false,
    cap_type: true,
    calls: true,
    jsx: false,
    preproc: false,
    rust: false,
    keys: Keys::None,
};

static JS: Cfg = Cfg {
    keywords: &[
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "of",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        "async",
        "await",
        "static",
        "as",
        "interface",
        "implements",
        "public",
        "private",
        "protected",
        "readonly",
        "abstract",
        "declare",
        "namespace",
        "keyof",
    ],
    types: &[
        "string", "number", "boolean", "any", "never", "unknown", "bigint",
    ],
    quotes: b"\"'`",
    backtick_multi: true,
    jsx: true,
    ..BASE
};
static PY: Cfg = Cfg {
    keywords: &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
        "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
        "yield",
    ],
    types: &[
        "int",
        "str",
        "float",
        "bool",
        "list",
        "dict",
        "set",
        "tuple",
        "bytes",
        "object",
        "bytearray",
    ],
    line: &["#"],
    block: false,
    triple: true,
    ..BASE
};
static RUST: Cfg = Cfg {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "type",
        "unsafe", "use", "where", "while",
    ],
    types: &[
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "bool", "char", "str",
    ],
    rust: true,
    ..BASE
};
static GO: Cfg = Cfg {
    keywords: &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "type",
        "var",
    ],
    types: &[
        "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64",
        "uintptr", "float32", "float64", "string", "bool", "byte", "rune", "error", "any",
    ],
    quotes: b"\"'`",
    backtick_multi: true,
    ..BASE
};
static JAVA: Cfg = Cfg {
    keywords: &[
        "abstract",
        "assert",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "interface",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "void",
        "volatile",
        "while",
        "var",
        "record",
    ],
    types: &[
        "int", "long", "short", "byte", "char", "float", "double", "boolean",
    ],
    ..BASE
};
static C: Cfg = Cfg {
    keywords: &[
        "auto",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "constexpr",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "explicit",
        "extern",
        "for",
        "friend",
        "goto",
        "if",
        "inline",
        "namespace",
        "new",
        "operator",
        "private",
        "protected",
        "public",
        "register",
        "return",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "throw",
        "try",
        "typedef",
        "typename",
        "union",
        "using",
        "virtual",
        "volatile",
        "while",
    ],
    types: &[
        "int", "long", "short", "char", "float", "double", "void", "unsigned", "signed", "bool",
        "size_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t",
        "uint64_t",
    ],
    preproc: true,
    ..BASE
};
static CS: Cfg = Cfg {
    keywords: &[
        "abstract",
        "as",
        "base",
        "break",
        "case",
        "catch",
        "checked",
        "class",
        "const",
        "continue",
        "default",
        "delegate",
        "do",
        "else",
        "enum",
        "event",
        "explicit",
        "extern",
        "finally",
        "fixed",
        "for",
        "foreach",
        "goto",
        "if",
        "implicit",
        "in",
        "interface",
        "internal",
        "is",
        "lock",
        "namespace",
        "new",
        "operator",
        "out",
        "override",
        "params",
        "private",
        "protected",
        "public",
        "readonly",
        "ref",
        "return",
        "sealed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "unchecked",
        "unsafe",
        "using",
        "virtual",
        "volatile",
        "while",
        "var",
        "async",
        "await",
    ],
    types: &[
        "bool", "byte", "char", "decimal", "double", "float", "int", "long", "object", "sbyte",
        "short", "string", "uint", "ulong", "ushort", "void", "dynamic",
    ],
    ..BASE
};
static PHP: Cfg = Cfg {
    keywords: &[
        "abstract",
        "and",
        "array",
        "as",
        "break",
        "callable",
        "case",
        "catch",
        "class",
        "clone",
        "const",
        "continue",
        "declare",
        "default",
        "do",
        "echo",
        "else",
        "elseif",
        "empty",
        "extends",
        "final",
        "finally",
        "fn",
        "for",
        "foreach",
        "function",
        "global",
        "if",
        "implements",
        "include",
        "include_once",
        "instanceof",
        "interface",
        "isset",
        "namespace",
        "new",
        "or",
        "print",
        "private",
        "protected",
        "public",
        "require",
        "require_once",
        "return",
        "static",
        "switch",
        "throw",
        "trait",
        "try",
        "unset",
        "use",
        "var",
        "while",
        "yield",
    ],
    types: &[
        "int", "float", "bool", "string", "mixed", "void", "iterable", "object",
    ],
    line: &["//", "#"],
    ..BASE
};
static SHELL: Cfg = Cfg {
    keywords: &[
        "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do", "done",
        "in", "function", "select", "time", "return", "exit", "break", "continue", "local",
        "export", "readonly", "declare", "unset", "shift", "source", "alias",
    ],
    line: &["#"],
    quotes: b"\"'`",
    block: false,
    cap_type: false,
    ..BASE
};
static JSON: Cfg = Cfg {
    line: &[],
    quotes: b"\"",
    block: false,
    cap_type: false,
    calls: false,
    keys: Keys::Json,
    ..BASE
};
static YAML: Cfg = Cfg {
    line: &["#"],
    block: false,
    cap_type: false,
    calls: false,
    keys: Keys::Yaml,
    ..BASE
};
static TOML: Cfg = Cfg {
    line: &["#"],
    block: false,
    cap_type: false,
    calls: false,
    keys: Keys::Toml,
    ..BASE
};

fn cfg_for(lang: Lang) -> &'static Cfg {
    match lang {
        Lang::JsTs => &JS,
        Lang::Python => &PY,
        Lang::Rust => &RUST,
        Lang::Go => &GO,
        Lang::Java => &JAVA,
        Lang::C => &C,
        Lang::CSharp => &CS,
        Lang::Php => &PHP,
        Lang::Shell => &SHELL,
        Lang::Json => &JSON,
        Lang::Yaml => &YAML,
        _ => &TOML,
    }
}

const CONSTANTS: &[&str] = &[
    "true",
    "false",
    "null",
    "nil",
    "None",
    "True",
    "False",
    "undefined",
    "NULL",
    "nullptr",
    "NaN",
    "Infinity",
];

/// Collects tokens, fills gaps with `Text` and merges neighbours of the same kind.
struct Emit<F: FnMut(Range<usize>, Token)> {
    f: F,
    pos: usize,
    pending: Option<(usize, usize, Token)>,
}
impl<F: FnMut(Range<usize>, Token)> Emit<F> {
    fn push(&mut self, a: usize, b: usize, tok: Token) {
        if let Some((s, e, t)) = self.pending {
            if t == tok && e == a {
                self.pending = Some((s, b, t));
                return;
            }
            (self.f)(s..e, t);
        }
        self.pending = Some((a, b, tok));
    }
    fn emit(&mut self, a: usize, b: usize, tok: Token) {
        let a = a.max(self.pos);
        if a >= b {
            return;
        }
        if a > self.pos {
            self.push(self.pos, a, Token::Text);
        }
        self.push(a, b, tok);
        self.pos = b;
    }
    fn finish(&mut self, len: usize) {
        if len > self.pos {
            self.push(self.pos, len, Token::Text);
            self.pos = len;
        }
        if let Some((s, e, t)) = self.pending.take() {
            (self.f)(s..e, t);
        }
    }
}

pub fn highlight(lang: Lang, state: &mut State, line: &str, out: impl FnMut(Range<usize>, Token)) {
    let mut e = Emit {
        f: out,
        pos: 0,
        pending: None,
    };
    match lang {
        Lang::Plain => {}
        Lang::Markdown => markdown(state, line, &mut e),
        Lang::Html => html(state, line, &mut e),
        Lang::Css => css(state, line, &mut e),
        _ => code(cfg_for(lang), state, line, &mut e),
    }
    e.finish(line.len());
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}
fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80
}
fn next_nonspace(b: &[u8], mut i: usize) -> Option<u8> {
    while i < b.len() {
        if b[i] != b' ' && b[i] != b'\t' {
            return Some(b[i]);
        }
        i += 1;
    }
    None
}
fn number_end(b: &[u8], i: usize) -> usize {
    let n = b.len();
    let mut j = i;
    if b[j] == b'0' && j + 1 < n && matches!(b[j + 1], b'x' | b'X' | b'b' | b'B' | b'o' | b'O') {
        j += 2;
        while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
            j += 1;
        }
        return j;
    }
    while j < n {
        let c = b[j];
        if c.is_ascii_alphanumeric() || c == b'_' {
            if (c == b'e' || c == b'E')
                && j + 2 < n
                && (b[j + 1] == b'+' || b[j + 1] == b'-')
                && b[j + 2].is_ascii_digit()
            {
                j += 2;
            }
            j += 1;
        } else if c == b'.' && j + 1 < n && b[j + 1].is_ascii_digit() {
            j += 1;
        } else {
            break;
        }
    }
    j
}
/// Index after the closing quote (or line end) and whether it was closed.
fn string_end(b: &[u8], from: usize, quote: u8) -> (usize, bool) {
    let mut j = from;
    while j < b.len() {
        if b[j] == b'\\' {
            j += 2;
        } else if b[j] == quote {
            return (j + 1, true);
        } else {
            j += 1;
        }
    }
    (b.len(), false)
}
/// Emit a string body starting at `from` (after the opening quote); escapes become keywords.
fn string_body<F: FnMut(Range<usize>, Token)>(
    line: &str,
    from: usize,
    quote: u8,
    e: &mut Emit<F>,
) -> (usize, bool) {
    let b = line.as_bytes();
    let n = b.len();
    let mut seg = from;
    let mut j = from;
    while j < n {
        if b[j] == b'\\' && j + 1 < n {
            let mut k = j + 2;
            while k < n && !line.is_char_boundary(k) {
                k += 1;
            }
            e.emit(seg, j, Token::String);
            e.emit(j, k, Token::Keyword);
            seg = k;
            j = k;
        } else if b[j] == quote {
            e.emit(seg, j + 1, Token::String);
            return (j + 1, true);
        } else {
            j += 1;
        }
    }
    e.emit(seg, n, Token::String);
    (n, false)
}
fn is_all_caps(w: &str) -> bool {
    w.len() >= 2
        && w.as_bytes()[0].is_ascii_uppercase()
        && w.bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
}

fn code<F: FnMut(Range<usize>, Token)>(cfg: &Cfg, state: &mut State, line: &str, e: &mut Emit<F>) {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    match state.0 {
        BLOCK => match line.find("*/") {
            Some(p) => {
                e.emit(0, p + 2, Token::Comment);
                i = p + 2;
                *state = State(0);
            }
            None => {
                e.emit(0, n, Token::Comment);
                return;
            }
        },
        TRIPLE_SINGLE | TRIPLE_DOUBLE => {
            let marker = if state.0 == TRIPLE_SINGLE {
                "'''"
            } else {
                "\"\"\""
            };
            match line.find(marker) {
                Some(p) => {
                    e.emit(0, p + 3, Token::String);
                    i = p + 3;
                    *state = State(0);
                }
                None => {
                    e.emit(0, n, Token::String);
                    return;
                }
            }
        }
        BACKTICK => {
            let (end, closed) = string_body(line, 0, b'`', e);
            i = end;
            if closed {
                *state = State(0);
            } else {
                return;
            }
        }
        _ => {}
    }
    // Key prefixes for YAML / TOML.
    if i == 0 && matches!(cfg.keys, Keys::Yaml | Keys::Toml) {
        i = key_prefix(cfg.keys, line, e);
    }
    let mut after_fn = false;
    let mut after_type = false;
    let mut in_tag = false;
    while i < n {
        let c = b[i];
        if c == b' ' || c == b'\t' {
            i += 1;
            continue;
        }
        if cfg.preproc && c == b'#' && line[..i].trim().is_empty() {
            let mut j = i + 1;
            while j < n && b[j].is_ascii_alphabetic() {
                j += 1;
            }
            e.emit(i, j, Token::Keyword);
            i = j;
            continue;
        }
        if cfg.line.iter().any(|m| {
            b[i..].starts_with(m.as_bytes())
                && (*m != "#" || i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t')
        }) {
            e.emit(i, n, Token::Comment);
            return;
        }
        if cfg.block && b[i..].starts_with(b"/*") {
            match line[i + 2..].find("*/") {
                Some(p) => {
                    let end = i + 2 + p + 2;
                    e.emit(i, end, Token::Comment);
                    i = end;
                    continue;
                }
                None => {
                    e.emit(i, n, Token::Comment);
                    *state = State(BLOCK);
                    return;
                }
            }
        }
        if cfg.triple && (b[i..].starts_with(b"\"\"\"") || b[i..].starts_with(b"'''")) {
            let (marker, st) = if c == b'"' {
                ("\"\"\"", TRIPLE_DOUBLE)
            } else {
                ("'''", TRIPLE_SINGLE)
            };
            match line[i + 3..].find(marker) {
                Some(p) => {
                    let end = i + 3 + p + 3;
                    e.emit(i, end, Token::String);
                    i = end;
                    continue;
                }
                None => {
                    e.emit(i, n, Token::String);
                    *state = State(st);
                    return;
                }
            }
        }
        if cfg.jsx && c == b'<' && jsx_tag_start(line, i) {
            let mut j = i + 1;
            if b[j] == b'/' {
                j += 1;
            }
            let name = j;
            while j < n && (is_ident(b[j]) || matches!(b[j], b'.' | b'-' | b':')) {
                j += 1;
            }
            if j > name {
                e.emit(name, j, Token::Tag);
                i = j;
                in_tag = true;
                continue;
            }
        }
        if in_tag && c == b'>' {
            in_tag = false;
        }
        if cfg.quotes.contains(&c) {
            if cfg.rust && c == b'\'' {
                let char_len = line[i + 1..].chars().next().map_or(0, char::len_utf8);
                let is_char = b.get(i + 1) == Some(&b'\\')
                    || (char_len > 0 && b.get(i + 1 + char_len) == Some(&b'\''));
                if !is_char {
                    let mut j = i + 1;
                    while j < n && is_ident(b[j]) {
                        j += 1;
                    }
                    e.emit(i, j, Token::Type);
                    i = j.max(i + 1);
                    continue;
                }
            }
            if cfg.keys == Keys::Json {
                let (end, _) = string_end(b, i + 1, c);
                if next_nonspace(b, end) == Some(b':') {
                    e.emit(i, end, Token::Tag);
                    i = end;
                    continue;
                }
            }
            e.emit(i, i + 1, Token::String);
            let (end, closed) = string_body(line, i + 1, c, e);
            i = end;
            if c == b'`' && cfg.backtick_multi && !closed {
                *state = State(BACKTICK);
                return;
            }
            continue;
        }
        if c.is_ascii_digit() {
            let end = number_end(b, i);
            e.emit(i, end, Token::Constant);
            i = end;
            continue;
        }
        if is_ident_start(c) {
            let s = i;
            while i < n && is_ident(b[i]) {
                i += 1;
            }
            let w = &line[s..i];
            let mut tok = Token::Text;
            let mut next_fn = false;
            let mut next_type = false;
            let call = cfg.calls && next_nonspace(b, i) == Some(b'(');
            let macro_call = cfg.rust
                && b.get(i) == Some(&b'!')
                && matches!(b.get(i + 1), Some(b'(' | b'[' | b'{'));
            if CONSTANTS.contains(&w) {
                tok = Token::Constant;
            } else if cfg.keywords.contains(&w) {
                tok = Token::Keyword;
                next_fn = matches!(w, "fn" | "def" | "function" | "func");
                next_type = matches!(w, "class" | "struct" | "enum" | "trait" | "interface");
            } else if after_fn {
                tok = Token::Function;
            } else if after_type {
                tok = Token::Type;
            } else if in_tag && b.get(i) == Some(&b'=') {
                tok = Token::Constant;
            } else if cfg.types.contains(&w) {
                tok = Token::Type;
            } else if call || macro_call {
                tok = Token::Function;
            } else if is_all_caps(w) {
                tok = Token::Constant;
            } else if cfg.cap_type
                && w.as_bytes()[0].is_ascii_uppercase()
                && w.bytes().any(|c| c.is_ascii_lowercase())
            {
                tok = Token::Type;
            }
            after_fn = next_fn;
            after_type = next_type;
            e.emit(s, i, tok);
            continue;
        }
        after_fn = false;
        after_type = false;
        i += 1;
    }
}

/// Whether `<` at byte `i` starts a JSX tag rather than a comparison or generic.
fn jsx_tag_start(line: &str, i: usize) -> bool {
    let b = line.as_bytes();
    let Some(&next) = b.get(i + 1) else {
        return false;
    };
    if next == b'/' {
        return b.get(i + 2).is_some_and(u8::is_ascii_alphabetic);
    }
    if !next.is_ascii_alphabetic() {
        return false;
    }
    let before = line[..i].trim_end();
    before.is_empty()
        || before.ends_with(['(', '=', ',', '>', ':', '?', '&', '|', '{', '[', ';'])
        || before.ends_with("return")
}

/// Highlight a YAML/TOML key at the start of the line; returns where scanning continues.
fn key_prefix<F: FnMut(Range<usize>, Token)>(keys: Keys, line: &str, e: &mut Emit<F>) -> usize {
    let b = line.as_bytes();
    let n = b.len();
    let mut p = 0;
    while p < n && (b[p] == b' ' || b[p] == b'\t') {
        p += 1;
    }
    if p >= n {
        return 0;
    }
    if keys == Keys::Toml {
        if b[p] == b'[' {
            let end = line.trim_end().len();
            if line[p..end].find('#').is_none() {
                e.emit(p, end, Token::Tag);
                return end;
            }
            return 0;
        }
        let mut j = p;
        while j < n && !matches!(b[j], b'=' | b'#' | b'"' | b'\'') {
            j += 1;
        }
        if j < n && b[j] == b'=' {
            let end = line[p..j].trim_end().len() + p;
            e.emit(p, end, Token::Tag);
            return end;
        }
        return 0;
    }
    while p + 1 < n && b[p] == b'-' && b[p + 1] == b' ' {
        p += 2;
        while p < n && b[p] == b' ' {
            p += 1;
        }
    }
    let mut j = p;
    while j < n && !matches!(b[j], b'#' | b'[' | b']' | b'{' | b'}' | b',' | b'"' | b'\'') {
        if b[j] == b':' && (j + 1 == n || b[j + 1] == b' ') && j > p {
            e.emit(p, j, Token::Tag);
            return j;
        }
        j += 1;
    }
    0
}

fn markdown<F: FnMut(Range<usize>, Token)>(state: &mut State, line: &str, e: &mut Emit<F>) {
    let n = line.len();
    let t = line.trim_start();
    let indent = n - t.len();
    let fence = t.starts_with("```") || t.starts_with("~~~");
    if state.0 == FENCE {
        if fence {
            *state = State(0);
            e.emit(indent, n, Token::Constant);
        }
        return;
    }
    if fence {
        *state = State(FENCE);
        e.emit(indent, n, Token::Constant);
        return;
    }
    let hashes = t.bytes().take_while(|&c| c == b'#').count();
    if indent <= 3
        && (1..=6).contains(&hashes)
        && (t.len() == hashes || t.as_bytes()[hashes] == b' ')
    {
        e.emit(indent, n, Token::Heading);
        return;
    }
    let b = line.as_bytes();
    let mut i = 0;
    while i < n {
        if b[i] == b'`' {
            if let Some(p) = line[i + 1..].find('`') {
                let end = i + 1 + p + 1;
                e.emit(i, end, Token::Constant);
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

fn css<F: FnMut(Range<usize>, Token)>(state: &mut State, line: &str, e: &mut Emit<F>) {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    if state.0 == BLOCK {
        match line.find("*/") {
            Some(p) => {
                e.emit(0, p + 2, Token::Comment);
                i = p + 2;
                *state = State(0);
            }
            None => {
                e.emit(0, n, Token::Comment);
                return;
            }
        }
    }
    let css_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c >= 0x80;
    while i < n {
        let c = b[i];
        if b[i..].starts_with(b"/*") {
            match line[i + 2..].find("*/") {
                Some(p) => {
                    let end = i + 2 + p + 2;
                    e.emit(i, end, Token::Comment);
                    i = end;
                    continue;
                }
                None => {
                    e.emit(i, n, Token::Comment);
                    *state = State(BLOCK);
                    return;
                }
            }
        }
        if c == b'"' || c == b'\'' {
            e.emit(i, i + 1, Token::String);
            let (end, _) = string_body(line, i + 1, c, e);
            i = end;
            continue;
        }
        let signed_number =
            matches!(c, b'-' | b'+' | b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit);
        if c.is_ascii_digit() || signed_number {
            let mut end = number_end(
                b,
                if c == b'.' {
                    i
                } else {
                    i + usize::from(!c.is_ascii_digit())
                },
            );
            if end < n && b[end] == b'%' {
                end += 1;
            }
            e.emit(i, end, Token::Constant);
            i = end;
            continue;
        }
        if c == b'#' {
            let mut j = i + 1;
            while j < n && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            if matches!(j - i - 1, 3 | 4 | 6 | 8) && (j == n || !css_ident(b[j])) {
                e.emit(i, j, Token::Constant);
            }
            i = j.max(i + 1);
            continue;
        }
        if c == b'@' || c == b'!' {
            let mut j = i + 1;
            while j < n && css_ident(b[j]) {
                j += 1;
            }
            e.emit(i, j, Token::Keyword);
            i = j;
            continue;
        }
        if css_ident(c) {
            let s = i;
            while i < n && css_ident(b[i]) {
                i += 1;
            }
            let next = next_nonspace(b, i);
            if next == Some(b':') {
                let colon = line[i..].find(':').map_or(i, |p| i + p);
                if !line[colon..].contains('{') {
                    e.emit(s, i, Token::Constant);
                }
            } else if next == Some(b'(') && b[i] == b'(' {
                e.emit(s, i, Token::Function);
            }
            continue;
        }
        i += 1;
    }
}

fn html<F: FnMut(Range<usize>, Token)>(state: &mut State, line: &str, e: &mut Emit<F>) {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    if state.0 == HTML_COMMENT {
        match line.find("-->") {
            Some(p) => {
                e.emit(0, p + 3, Token::Comment);
                i = p + 3;
                *state = State(0);
            }
            None => {
                e.emit(0, n, Token::Comment);
                return;
            }
        }
    }
    let mut in_tag = state.0 == HTML_TAG;
    while i < n {
        if !in_tag {
            let Some(p) = line[i..].find('<') else {
                break;
            };
            let lt = i + p;
            if b[lt..].starts_with(b"<!--") {
                match line[lt + 4..].find("-->") {
                    Some(q) => {
                        let end = lt + 4 + q + 3;
                        e.emit(lt, end, Token::Comment);
                        i = end;
                    }
                    None => {
                        e.emit(lt, n, Token::Comment);
                        *state = State(HTML_COMMENT);
                        return;
                    }
                }
                continue;
            }
            let mut j = lt + 1;
            if b.get(j) == Some(&b'!') {
                j += 1;
                let s = j;
                while j < n && b[j].is_ascii_alphabetic() {
                    j += 1;
                }
                e.emit(lt + 1, s, Token::Text);
                e.emit(s, j, Token::Keyword);
                i = j;
                in_tag = true;
                continue;
            }
            if b.get(j) == Some(&b'/') {
                j += 1;
            }
            let s = j;
            while j < n && (b[j].is_ascii_alphanumeric() || matches!(b[j], b'-' | b':')) {
                j += 1;
            }
            if j > s && b[s].is_ascii_alphabetic() {
                e.emit(s, j, Token::Tag);
                in_tag = true;
            }
            i = j.max(lt + 1);
            continue;
        }
        let c = b[i];
        if c == b'>' {
            in_tag = false;
            i += 1;
        } else if c == b'"' || c == b'\'' {
            e.emit(i, i + 1, Token::String);
            let (end, _) = string_body(line, i + 1, c, e);
            i = end;
        } else if b[i].is_ascii_alphabetic() || matches!(c, b'@' | b':' | b'_') {
            let s = i;
            while i < n
                && (b[i].is_ascii_alphanumeric()
                    || matches!(b[i], b'-' | b'_' | b':' | b'.' | b'@'))
            {
                i += 1;
            }
            e.emit(s, i, Token::Constant);
        } else {
            i += 1;
        }
    }
    *state = State(if in_tag { HTML_TAG } else { 0 });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(lang: Lang, state: &mut State, line: &str) -> Vec<(String, Token)> {
        let mut out = Vec::new();
        let mut pos = 0;
        highlight(lang, state, line, |r, t| {
            assert_eq!(r.start, pos, "gap in {line:?}");
            assert!(line.is_char_boundary(r.start) && line.is_char_boundary(r.end));
            pos = r.end;
            out.push((line[r].to_string(), t));
        });
        assert_eq!(pos, line.len(), "line not covered: {line:?}");
        out
    }
    fn has(lang: Lang, line: &str, text: &str, tok: Token) {
        let all = tokens(lang, &mut State::default(), line);
        assert!(
            all.iter().any(|(s, t)| s == text && *t == tok),
            "{text:?} as {tok:?} not in {all:?}"
        );
    }

    fn plain(lang: Lang, line: &str, text: &str) {
        let start = line.find(text).unwrap();
        let mut pos = 0;
        for (s, t) in tokens(lang, &mut State::default(), line) {
            let (a, b) = (pos, pos + s.len());
            pos = b;
            if a < start + text.len() && b > start {
                assert_eq!(t, Token::Text, "{s:?} in {line:?}");
            }
        }
    }

    #[test]
    fn language_from_extension_is_case_insensitive() {
        assert_eq!(lang_for_path("src/App.TSX"), Lang::JsTs);
        assert_eq!(lang_for_path("a\\b\\main.rs"), Lang::Rust);
        assert_eq!(lang_for_path("x.hpp"), Lang::C);
        assert_eq!(lang_for_path("Makefile"), Lang::Plain);
        assert_eq!(lang_for_path("a.unknown"), Lang::Plain);
        assert_eq!(lang_for_path("notes.markdown"), Lang::Markdown);
    }

    #[test]
    fn js_ts() {
        has(Lang::JsTs, "const x = 42;", "const", Token::Keyword);
        has(Lang::JsTs, "const x = 42;", "42", Token::Constant);
        has(Lang::JsTs, "let s = 'a\\n';", "\\n", Token::Keyword);
        has(Lang::JsTs, "foo(1) // hi", "// hi", Token::Comment);
        has(Lang::JsTs, "foo(1)", "foo", Token::Function);
        has(Lang::JsTs, "x: Promise<string>", "Promise", Token::Type);
        has(
            Lang::JsTs,
            "return <div className=\"a\">",
            "div",
            Token::Tag,
        );
        has(
            Lang::JsTs,
            "return <div className=\"a\">",
            "className",
            Token::Constant,
        );
        plain(Lang::JsTs, "if (a < b) {}", "a < b");
        has(Lang::JsTs, "x = undefined", "undefined", Token::Constant);
    }

    #[test]
    fn js_template_literal_spans_lines() {
        let mut s = State::default();
        tokens(Lang::JsTs, &mut s, "const t = `abc");
        assert_ne!(s, State::default());
        let t = tokens(Lang::JsTs, &mut s, "def` + 1");
        assert_eq!(t[0], ("def`".to_string(), Token::String));
        assert_eq!(s, State::default());
    }

    #[test]
    fn python() {
        has(Lang::Python, "def run(x):", "def", Token::Keyword);
        has(Lang::Python, "def run(x):", "run", Token::Function);
        has(Lang::Python, "x = 0x1F", "0x1F", Token::Constant);
        has(Lang::Python, "y = 'a' # note", "# note", Token::Comment);
        has(Lang::Python, "return None", "None", Token::Constant);
        has(Lang::Python, "class foo:", "foo", Token::Type);
        let mut s = State::default();
        tokens(Lang::Python, &mut s, "doc = \"\"\"start");
        let t = tokens(Lang::Python, &mut s, "still string");
        assert_eq!(t, vec![("still string".to_string(), Token::String)]);
        tokens(Lang::Python, &mut s, "end\"\"\"");
        assert_eq!(s, State::default());
    }

    #[test]
    fn html() {
        has(Lang::Html, "<a href=\"x\">hi</a>", "a", Token::Tag);
        has(Lang::Html, "<a href=\"x\">hi</a>", "href", Token::Constant);
        has(Lang::Html, "<a href=\"x\">hi</a>", "\"x\"", Token::String);
        has(Lang::Html, "<!-- c -->", "<!-- c -->", Token::Comment);
        has(Lang::Html, "<!DOCTYPE html>", "DOCTYPE", Token::Keyword);
        let mut s = State::default();
        tokens(Lang::Html, &mut s, "<div");
        let t = tokens(Lang::Html, &mut s, "  id=\"a\">");
        assert!(t.contains(&("id".to_string(), Token::Constant)));
        assert_eq!(s, State::default());
        tokens(Lang::Html, &mut s, "<!-- open");
        assert_eq!(
            tokens(Lang::Html, &mut s, "x --> <b>")[0],
            ("x -->".to_string(), Token::Comment)
        );
    }

    #[test]
    fn css() {
        has(Lang::Css, "  color: #fff;", "color", Token::Constant);
        has(Lang::Css, "  color: #fff;", "#fff", Token::Constant);
        has(Lang::Css, "margin: 10px;", "10px", Token::Constant);
        plain(Lang::Css, "a:hover {", "a:hover");
        has(Lang::Css, "@media screen {", "@media", Token::Keyword);
        has(Lang::Css, "content: \"x\";", "\"x\"", Token::String);
        has(Lang::Css, "/* c */", "/* c */", Token::Comment);
    }

    #[test]
    fn json() {
        has(Lang::Json, "  \"name\": \"dark\",", "\"name\"", Token::Tag);
        has(
            Lang::Json,
            "  \"name\": \"dark\",",
            "\"dark\"",
            Token::String,
        );
        has(Lang::Json, "  \"n\": 1.5e3,", "1.5e3", Token::Constant);
        has(Lang::Json, "  \"ok\": true", "true", Token::Constant);
    }

    #[test]
    fn rust() {
        has(Lang::Rust, "pub fn main() {", "pub", Token::Keyword);
        has(Lang::Rust, "pub fn main() {", "main", Token::Function);
        has(Lang::Rust, "let s = \"a\";", "\"a\"", Token::String);
        has(Lang::Rust, "let n = 1_000u32;", "1_000u32", Token::Constant);
        has(Lang::Rust, "x // c", "// c", Token::Comment);
        has(Lang::Rust, "println!(\"x\")", "println", Token::Function);
        has(Lang::Rust, "let v: Vec<u8>", "Vec", Token::Type);
        has(Lang::Rust, "fn f<'a>(x: &'a str)", "'a", Token::Type);
        has(Lang::Rust, "let c = 'x';", "'x'", Token::String);
        has(Lang::Rust, "const MAX: usize = 1;", "MAX", Token::Constant);
    }

    #[test]
    fn block_comment_spans_lines() {
        let mut s = State::default();
        let a = tokens(Lang::Rust, &mut s, "let a = 1; /* start");
        assert_eq!(a.last().unwrap(), &("/* start".to_string(), Token::Comment));
        let b = tokens(Lang::Rust, &mut s, "middle");
        assert_eq!(b, vec![("middle".to_string(), Token::Comment)]);
        let c = tokens(Lang::Rust, &mut s, "end */ let b");
        assert_eq!(c[0], ("end */".to_string(), Token::Comment));
        assert_eq!(c[1].1, Token::Text);
        assert!(c.contains(&("let".to_string(), Token::Keyword)));
        assert_eq!(s, State::default());
    }

    #[test]
    fn go() {
        has(Lang::Go, "func Run(x int) {", "func", Token::Keyword);
        has(Lang::Go, "func Run(x int) {", "Run", Token::Function);
        has(Lang::Go, "func Run(x int) {", "int", Token::Type);
        has(Lang::Go, "s := `raw`", "`raw`", Token::String);
        has(Lang::Go, "x := nil // c", "nil", Token::Constant);
    }

    #[test]
    fn java() {
        has(Lang::Java, "public class Foo {", "public", Token::Keyword);
        has(Lang::Java, "public class Foo {", "Foo", Token::Type);
        has(Lang::Java, "int x = 5;", "int", Token::Type);
        has(Lang::Java, "run(1); // c", "run", Token::Function);
        has(Lang::Java, "s = \"a\";", "\"a\"", Token::String);
    }

    #[test]
    fn c_family() {
        has(Lang::C, "#include <stdio.h>", "#include", Token::Keyword);
        has(Lang::C, "static int x = 0xFF;", "static", Token::Keyword);
        has(Lang::C, "static int x = 0xFF;", "0xFF", Token::Constant);
        has(Lang::C, "printf(\"hi\\n\");", "printf", Token::Function);
        has(Lang::C, "x = 1; /* c */", "/* c */", Token::Comment);
    }

    #[test]
    fn csharp() {
        has(
            Lang::CSharp,
            "public static void Main()",
            "static",
            Token::Keyword,
        );
        has(
            Lang::CSharp,
            "public static void Main()",
            "void",
            Token::Type,
        );
        has(Lang::CSharp, "var s = \"a\";", "\"a\"", Token::String);
        has(Lang::CSharp, "Run(3); // c", "// c", Token::Comment);
    }

    #[test]
    fn php() {
        has(Lang::Php, "function run($a) {", "function", Token::Keyword);
        has(Lang::Php, "function run($a) {", "run", Token::Function);
        has(Lang::Php, "$x = 'a'; # c", "# c", Token::Comment);
        has(Lang::Php, "$x = 12;", "12", Token::Constant);
    }

    #[test]
    fn shell() {
        has(Lang::Shell, "if [ -f x ]; then", "if", Token::Keyword);
        has(Lang::Shell, "echo \"hi\" # c", "# c", Token::Comment);
        has(Lang::Shell, "echo \"hi\"", "\"hi\"", Token::String);
        plain(Lang::Shell, "echo ${#x}", "${#x}");
        has(Lang::Shell, "run() {", "run", Token::Function);
    }

    #[test]
    fn yaml() {
        has(Lang::Yaml, "name: dark", "name", Token::Tag);
        has(Lang::Yaml, "  - port: 8080", "port", Token::Tag);
        has(Lang::Yaml, "  - port: 8080", "8080", Token::Constant);
        has(Lang::Yaml, "k: \"v\" # c", "\"v\"", Token::String);
        has(Lang::Yaml, "k: v # c", "# c", Token::Comment);
        has(Lang::Yaml, "on: true", "true", Token::Constant);
    }

    #[test]
    fn toml() {
        has(Lang::Toml, "[package]", "[package]", Token::Tag);
        has(Lang::Toml, "name = \"dark\"", "name", Token::Tag);
        has(Lang::Toml, "name = \"dark\"", "\"dark\"", Token::String);
        has(Lang::Toml, "n = 3 # c", "3", Token::Constant);
        has(Lang::Toml, "# c", "# c", Token::Comment);
    }

    #[test]
    fn markdown() {
        has(Lang::Markdown, "## Title", "## Title", Token::Heading);
        has(
            Lang::Markdown,
            "use `cargo` now",
            "`cargo`",
            Token::Constant,
        );
        plain(Lang::Markdown, "#hashtag", "#hashtag");
        let mut s = State::default();
        tokens(Lang::Markdown, &mut s, "```rust");
        let t = tokens(Lang::Markdown, &mut s, "# not heading `x`");
        assert_eq!(t, vec![("# not heading `x`".to_string(), Token::Text)]);
        tokens(Lang::Markdown, &mut s, "```");
        assert_eq!(s, State::default());
    }

    #[test]
    fn plain_and_unicode_lines_stay_contiguous() {
        assert_eq!(
            tokens(Lang::Plain, &mut State::default(), "anything"),
            vec![("anything".to_string(), Token::Text)]
        );
        for lang in [Lang::Rust, Lang::JsTs, Lang::Python, Lang::Html, Lang::Css] {
            tokens(
                lang,
                &mut State::default(),
                "let ünï = \"日本\\é\"; // ✓ <b>é</b> 3px",
            );
        }
    }
}
