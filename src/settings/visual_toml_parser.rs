use crate::settings::Settings;

#[derive(Clone, Copy, PartialEq)]
enum Tok {
    Comment,
    Table,
    Key,
    Str,
    Num,
    Bool,
    Punct,
    Plain,
}

struct Span {
    start: usize,
    end: usize,
    kind: Tok,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum St {
    #[default] Normal,
    MlBasic,
    MlLit,
}

fn find_close(s: &str, from: usize, delim: &str, escapes: bool) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = from;
    while i < b.len() {
        if escapes && b[i] == b'\\' { i += 2; continue; }
        if s[i..].starts_with(delim) { return Some(i + delim.len()); }
        i += 1;
    }
    None
}

fn lex_line(line: &str, st: &mut St, out: &mut Vec<Span>) {
    let b = line.as_bytes();
    let mut i = 0;

    if *st != St::Normal {
        let (d, esc) = if *st == St::MlBasic { ("\"\"\"", true) } else { ("'''", false) };
        match find_close(line, 0, d, esc) {
            Some(e) => { out.push(Span { start: 0, end: e, kind: Tok::Str }); i = e; *st = St::Normal; }
            None => { out.push(Span { start: 0, end: line.len(), kind: Tok::Str }); return; }
        }
    }

    let mut expect_key = true;
    let mut braces: Vec<u8> = vec![]; // track inline tables
    while i < b.len() {
        let c = b[i];
        match c {
            b' ' | b'\t' => { i += 1; }
            b'#' => { out.push(Span { start: i, end: b.len(), kind: Tok::Comment }); break; }
            b'[' if expect_key && line[..i].trim().is_empty() => {
                let end = line[i..].find(']').map(|p| i + p + 1).unwrap_or(b.len());
                let end = if line[end..].starts_with(']') { end + 1 } else { end }; // [[array.tables]]
                out.push(Span { start: i, end, kind: Tok::Table });
                i = end;
            }
            b'"' | b'\'' => {
                let q = c as char;
                let triple = line[i..].starts_with(&q.to_string().repeat(3));
                let kind = if expect_key { Tok::Key } else { Tok::Str };
                if triple {
                    let d = q.to_string().repeat(3);
                    match find_close(line, i + 3, &d, q == '"') {
                        Some(e) => { out.push(Span { start: i, end: e, kind }); i = e; }
                        None => {
                            out.push(Span { start: i, end: b.len(), kind });
                            *st = if q == '"' { St::MlBasic } else { St::MlLit };
                            break;
                        }
                    }
                } else {
                    let e = find_close(line, i + 1, &q.to_string(), q == '"').unwrap_or(b.len());
                    out.push(Span { start: i, end: e, kind });
                    i = e;
                }
            }
            b'=' => { out.push(Span { start: i, end: i + 1, kind: Tok::Punct }); expect_key = false; i += 1; }
            b'{' => { braces.push(b'{'); expect_key = true; out.push(Span { start: i, end: i + 1, kind: Tok::Punct }); i += 1; }
            b'}' => { braces.pop(); out.push(Span { start: i, end: i + 1, kind: Tok::Punct }); i += 1; }
            b'[' | b']' | b',' => {
                if c == b',' && !braces.is_empty() { expect_key = true; }
                out.push(Span { start: i, end: i + 1, kind: Tok::Punct });
                i += 1;
            }
            _ => {
                let s = i;
                while i < b.len() && !matches!(b[i], b' ' | b'\t' | b',' | b']' | b'}' | b'#' | b'=') { i += 1; }
                let w = &line[s..i];
                let kind = if expect_key { Tok::Key }
                else if w == "true" || w == "false" { Tok::Bool }
                else if w.starts_with(|ch: char| ch.is_ascii_digit() || "+-".contains(ch)) || w == "inf" || w == "nan" { Tok::Num }
                else { Tok::Plain };
                out.push(Span { start: s, end: i, kind });
            }
        }
    }
}

pub fn check_errors(src: &str) -> Option<SettingsError> {
    // 1. Syntax errors
    if let Err(e) = src.parse::<toml::Table>() {
        return Some(SettingsError::from(&e, src));
    }
    // 2. Schema errors
    if let Err(e) = toml::from_str::<Settings>(src) {
        return Some(SettingsError::from(&e, src));
    }
    None
}

pub struct SettingsError {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub msg: String
}

impl SettingsError {
    fn from(e: &toml::de::Error, src: &str) -> Self {
        let span = e.span().unwrap_or(0..0);
        let line_start = src[..span.start].rfind('\n').map_or(0, |p| p + 1);
        let line = src[..span.start].matches('\n').count();
        let line_end = src[line_start..].find('\n').map_or(src.len(), |p| line_start + p);
        // zero-width span (e.g. "expected =" at EOL): underline the last char instead
        let end = if span.end > span.start { span.end.min(line_end) } else { line_end.max(span.start + 1).min(src.len()) };
        SettingsError {
            line,
            col_start: span.start - line_start,
            col_end: end - line_start,
            msg: e.message().to_string(),
        }
    }
}