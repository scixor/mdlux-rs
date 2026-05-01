use unicode_width::UnicodeWidthChar;

use crate::render::inline::Span;
use crate::util::width::visible_width;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Word,
    Space,
    Newline,
}

#[derive(Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    style: crate::theme::Style,
}

pub fn wrap_spans(spans: &[Span], width: usize) -> Vec<Vec<Span>> {
    debug_assert!(width > 0, "width must be positive");
    let width = width.max(1);
    let tokens = tokenize(spans);
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    let mut line_width = 0usize;

    for token in tokens {
        match token.kind {
            TokenKind::Newline => {
                lines.push(trim_end_spaces(std::mem::take(&mut line)));
                line_width = 0;
            }
            TokenKind::Space => {
                if line.is_empty() {
                    continue;
                }
                let token_w = visible_width(&token.text);
                if line_width + token_w <= width {
                    push_span(
                        &mut line,
                        Span {
                            text: token.text,
                            style: token.style,
                        },
                    );
                    line_width += token_w;
                } else {
                    lines.push(trim_end_spaces(std::mem::take(&mut line)));
                    line_width = 0;
                }
            }
            TokenKind::Word => {
                let token_w = visible_width(&token.text);
                if token.text.contains('\u{1b}') {
                    if line_width + token_w > width && !line.is_empty() {
                        lines.push(trim_end_spaces(std::mem::take(&mut line)));
                        line_width = 0;
                    }
                    push_span(
                        &mut line,
                        Span {
                            text: token.text,
                            style: token.style,
                        },
                    );
                    line_width += token_w;
                    continue;
                }
                if token_w <= width {
                    if line_width + token_w > width && !line.is_empty() {
                        lines.push(trim_end_spaces(std::mem::take(&mut line)));
                        line_width = 0;
                    }
                    push_span(
                        &mut line,
                        Span {
                            text: token.text,
                            style: token.style,
                        },
                    );
                    line_width += token_w;
                } else {
                    for part in split_long_token(&token.text, width) {
                        let part_w = visible_width(&part);
                        if line_width + part_w > width && !line.is_empty() {
                            lines.push(trim_end_spaces(std::mem::take(&mut line)));
                            line_width = 0;
                        }
                        push_span(
                            &mut line,
                            Span {
                                text: part,
                                style: token.style,
                            },
                        );
                        line_width += part_w;
                        if line_width >= width {
                            lines.push(trim_end_spaces(std::mem::take(&mut line)));
                            line_width = 0;
                        }
                    }
                }
            }
        }
    }
    if !line.is_empty() {
        lines.push(trim_end_spaces(line));
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn tokenize(spans: &[Span]) -> Vec<Token> {
    let mut out = Vec::new();
    for span in spans {
        if span.text.contains('\u{1b}') {
            let parts: Vec<&str> = span.text.split('\n').collect();
            for (idx, part) in parts.iter().enumerate() {
                if !part.is_empty() {
                    out.push(Token {
                        kind: TokenKind::Word,
                        text: (*part).to_string(),
                        style: span.style,
                    });
                }
                if idx + 1 < parts.len() {
                    out.push(Token {
                        kind: TokenKind::Newline,
                        text: "\n".to_string(),
                        style: span.style,
                    });
                }
            }
            continue;
        }
        let mut current = String::new();
        let mut mode: Option<TokenKind> = None;
        for ch in span.text.chars() {
            if ch == '\n' {
                flush_token(&mut out, &mut current, &mut mode, span.style);
                out.push(Token {
                    kind: TokenKind::Newline,
                    text: "\n".to_string(),
                    style: span.style,
                });
                continue;
            }
            let kind = if ch.is_whitespace() {
                TokenKind::Space
            } else {
                TokenKind::Word
            };
            if mode != Some(kind) {
                flush_token(&mut out, &mut current, &mut mode, span.style);
                mode = Some(kind);
            }
            current.push(ch);
        }
        flush_token(&mut out, &mut current, &mut mode, span.style);
    }
    out
}

fn flush_token(
    out: &mut Vec<Token>,
    current: &mut String,
    mode: &mut Option<TokenKind>,
    style: crate::theme::Style,
) {
    if let Some(kind) = *mode
        && !current.is_empty()
    {
        out.push(Token {
            kind,
            text: std::mem::take(current),
            style,
        });
    }
    *mode = None;
}

fn split_long_token(text: &str, width: usize) -> Vec<String> {
    debug_assert!(width > 0, "width must be positive");
    let mut out = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_w + w > width && !current.is_empty() {
            out.push(std::mem::take(&mut current));
            current_w = 0;
        }
        current.push(ch);
        current_w += w;
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn trim_end_spaces(mut line: Vec<Span>) -> Vec<Span> {
    if let Some(last) = line.last_mut() {
        let trimmed_len = last.text.trim_end().len();
        if trimmed_len < last.text.len() {
            last.text.truncate(trimmed_len);
        }
        if last.text.is_empty() {
            line.pop();
        }
    }
    line
}

fn push_span(line: &mut Vec<Span>, span: Span) {
    if let Some(last) = line.last_mut()
        && last.style == span.style
    {
        last.text.push_str(&span.text);
        return;
    }
    line.push(span);
}

#[cfg(test)]
mod tests {
    use crate::kitty::hyperlink::osc8_link;
    use crate::theme::Style;

    use super::{Span, wrap_spans};

    #[test]
    fn wraps_long_text() {
        let spans = vec![Span {
            text: "alpha beta gamma delta".to_string(),
            style: Style::default(),
        }];
        let lines = wrap_spans(&spans, 10);
        assert!(lines.len() >= 2);
        for line in lines {
            let text = line.into_iter().map(|s| s.text).collect::<String>();
            assert!(text.len() <= 10 || text.contains(' '));
        }
    }

    #[test]
    fn keeps_osc8_link_sequence_intact() {
        let spans = vec![Span {
            text: osc8_link("docs", "https://example.com"),
            style: Style::default(),
        }];
        let lines = wrap_spans(&spans, 2);
        assert_eq!(lines.len(), 1);
        let text = lines[0].iter().map(|s| s.text.clone()).collect::<String>();
        assert!(text.contains("\u{1b}]8;;https://example.com"));
        assert!(text.contains("docs"));
    }
}
