use terminal_size::{Width, terminal_size};
use unicode_width::UnicodeWidthStr;

pub fn terminal_width() -> Option<usize> {
    let (Width(width), _) = terminal_size()?;
    Some(width as usize)
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    let _ = chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(&kind @ (']' | '_')) => {
                    let _ = chars.next();
                    let check_bel = kind == ']';
                    let mut prev = '\0';
                    for c in chars.by_ref() {
                        if (check_bel && c == '\u{7}') || (prev == '\u{1b}' && c == '\\') {
                            break;
                        }
                        prev = c;
                    }
                }
                _ => out.push(ch),
            }
            continue;
        }
        out.push(ch);
    }
    out
}

pub fn visible_width(input: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi(input).as_str())
}
