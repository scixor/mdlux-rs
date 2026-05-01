use crate::kitty::hyperlink::osc8_link;
use crate::markdown::ast::Inline;
use crate::render::RenderContext;
use crate::theme::Style;

#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

pub fn render_inlines_to_spans(inlines: &[Inline], ctx: &RenderContext) -> Vec<Span> {
    let mut out = Vec::new();
    for inline in inlines {
        push_inline(inline, &ctx.theme.text, ctx, &mut out);
    }
    merge_spans(out)
}

pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) => out.push_str(text),
            Inline::Emph(inner) | Inline::Strong(inner) | Inline::Strike(inner) => {
                out.push_str(&plain_text(inner))
            }
            Inline::Link { text, .. } => out.push_str(&plain_text(text)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::FootnoteRef(name) => {
                out.push_str("[^");
                out.push_str(name);
                out.push(']');
            }
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
        }
    }
    out
}

fn push_inline(inline: &Inline, base: &Style, ctx: &RenderContext, out: &mut Vec<Span>) {
    match inline {
        Inline::Text(text) => out.push(Span {
            text: text.clone(),
            style: *base,
        }),
        Inline::Code(code) => out.push(Span {
            text: code.clone(),
            style: merge_style(base, &ctx.theme.inline_code),
        }),
        Inline::SoftBreak => out.push(Span {
            text: " ".to_string(),
            style: *base,
        }),
        Inline::HardBreak => out.push(Span {
            text: "\n".to_string(),
            style: *base,
        }),
        Inline::Emph(inner) => {
            let merged = merge_style(base, &ctx.theme.emphasis);
            for item in inner {
                push_inline(item, &merged, ctx, out);
            }
        }
        Inline::Strong(inner) => {
            let merged = merge_style(base, &ctx.theme.strong);
            for item in inner {
                push_inline(item, &merged, ctx, out);
            }
        }
        Inline::Strike(inner) => {
            let mut strike_style = *base;
            strike_style.dim = true;
            for item in inner {
                push_inline(item, &strike_style, ctx, out);
            }
        }
        Inline::Link { text, dest, .. } => {
            if ctx.capabilities.kitty_hyperlinks {
                let label = plain_text(text);
                out.push(Span {
                    text: osc8_link(&label, dest),
                    style: merge_style(base, &ctx.theme.link_label),
                });
            } else {
                let link_style = merge_style(base, &ctx.theme.link_label);
                for item in text {
                    push_inline(item, &link_style, ctx, out);
                }
                out.push(Span {
                    text: format!(" ({dest})"),
                    style: merge_style(base, &ctx.theme.link_url),
                });
            }
        }
        Inline::Image { alt, .. } => out.push(Span {
            text: format!("[{alt}]"),
            style: merge_style(base, &ctx.theme.link_label),
        }),
        Inline::FootnoteRef(name) => out.push(Span {
            text: format!("[^{name}]"),
            style: merge_style(base, &ctx.theme.link_url),
        }),
    }
}

fn merge_style(base: &Style, overlay: &Style) -> Style {
    Style {
        fg: overlay.fg.or(base.fg),
        bg: overlay.bg.or(base.bg),
        bold: base.bold || overlay.bold,
        italic: base.italic || overlay.italic,
        underline: base.underline || overlay.underline,
        dim: base.dim || overlay.dim,
    }
}

fn merge_spans(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut()
            && last.style == span.style
        {
            last.text.push_str(&span.text);
            continue;
        }
        out.push(span);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::markdown::ast::Inline;
    use crate::render::{Capabilities, RenderContext};
    use crate::theme::builtin::find_theme;

    use super::render_inlines_to_spans;

    #[test]
    fn link_includes_destination() {
        let ctx = RenderContext {
            width: 80,
            theme: find_theme("ansi").expect("theme exists").clone(),
            capabilities: Capabilities {
                ansi: true,
                kitty_text_size: false,
                kitty_graphics: false,
                kitty_hyperlinks: false,
            },
            source_dir: PathBuf::from("."),
            no_highlight: false,
        };
        let spans = render_inlines_to_spans(
            &[Inline::Link {
                text: vec![Inline::Text("x".to_string())],
                dest: "https://example.com".to_string(),
                title: None,
            }],
            &ctx,
        );
        let all = spans.iter().map(|s| s.text.clone()).collect::<String>();
        assert!(all.contains("https://example.com"));
    }

    #[test]
    fn kitty_link_hides_raw_url() {
        let ctx = RenderContext {
            width: 80,
            theme: find_theme("ansi").expect("theme exists").clone(),
            capabilities: Capabilities {
                ansi: true,
                kitty_text_size: true,
                kitty_graphics: false,
                kitty_hyperlinks: true,
            },
            source_dir: PathBuf::from("."),
            no_highlight: false,
        };
        let spans = render_inlines_to_spans(
            &[Inline::Link {
                text: vec![Inline::Text("docs".to_string())],
                dest: "https://example.com".to_string(),
                title: None,
            }],
            &ctx,
        );
        let all = spans.iter().map(|s| s.text.clone()).collect::<String>();
        assert!(all.contains("\u{1b}]8;;https://example.com"));
        assert!(!all.contains(" (https://example.com)"));
    }
}
