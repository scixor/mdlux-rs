use std::path::Path;

use anyhow::Result;

use crate::kitty::graphics::{KittyImageOptions, render_image};
use crate::kitty::text_size::{TextSize, sized_text};
use crate::markdown::ast::{Block, Inline};
use crate::render::RenderContext;

use super::ansi::apply_style;
use super::inline::render_inlines_to_spans;
use super::table::{RenderedTable, render_table};
use super::wrap::wrap_spans;

pub struct RenderState {
    pub highlighter: Option<crate::highlight::syntect::Highlighter>,
}

pub fn render_block(
    block: &Block,
    ctx: &RenderContext,
    state: &mut RenderState,
    out: &mut String,
) -> Result<()> {
    match block {
        Block::Heading { level, content } => {
            render_heading(*level, content, ctx, out);
            let trailing = match (ctx.capabilities.kitty_text_size, *level) {
                (true, 1) => 4,
                (true, 2) => 3,
                _ => 2,
            };
            for _ in 0..trailing {
                out.push('\n');
            }
        }
        Block::Paragraph(inlines) => {
            let spans = render_inlines_to_spans(inlines, ctx);
            let lines = wrap_spans(&spans, ctx.width);
            for line in lines {
                out.push_str(&spans_to_ansi_line(&line, ctx.capabilities.ansi));
                out.push('\n');
            }
            out.push('\n');
        }
        Block::CodeBlock { lang, code } => {
            render_code_block(lang.as_deref(), code, ctx, state, out);
            out.push('\n');
        }
        Block::BlockQuote(blocks) => {
            let mut quoted = String::new();
            for block in blocks {
                render_block(block, ctx, state, &mut quoted)?;
            }
            for line in quoted.lines() {
                if line.is_empty() {
                    out.push('\n');
                    continue;
                }
                let prefix = apply_style("| ", ctx.theme.quote_marker, ctx.capabilities.ansi);
                out.push_str(&prefix);
                let body = apply_style(line, ctx.theme.quote_text, ctx.capabilities.ansi);
                out.push_str(&body);
                out.push('\n');
            }
            out.push('\n');
        }
        Block::List {
            ordered,
            start,
            items,
        } => {
            render_list(*ordered, *start, items, ctx, state, out)?;
            out.push('\n');
        }
        Block::Table {
            headers,
            rows,
            aligns,
        } => {
            let rendered = RenderedTable {
                headers: headers.iter().map(|h| Inline::plain_text(h)).collect(),
                rows: rows
                    .iter()
                    .map(|r| r.iter().map(|c| Inline::plain_text(c)).collect::<Vec<_>>())
                    .collect(),
                aligns: aligns.clone(),
            };
            out.push_str(&render_table(
                &rendered,
                ctx.width,
                ctx.capabilities.ansi,
                ctx.theme.table_header,
                ctx.theme.table_border,
            ));
            out.push('\n');
            out.push('\n');
        }
        Block::FootnoteDefinition { name, content } => {
            render_footnote_definition(name, content, ctx, state, out)?;
            out.push('\n');
        }
        Block::Rule => {
            let rule_width = ctx.width.clamp(3, 60);
            let rule = "-".repeat(rule_width);
            out.push_str(&apply_style(&rule, ctx.theme.rule, ctx.capabilities.ansi));
            out.push('\n');
            out.push('\n');
        }
        Block::Image { alt, path, .. } => {
            render_image_block(alt, path, ctx, out)?;
            out.push('\n');
            out.push('\n');
        }
    }
    Ok(())
}

fn render_heading(level: u8, inlines: &[Inline], ctx: &RenderContext, out: &mut String) {
    let raw = Inline::plain_text(inlines).trim().to_string();
    let ansi = ctx.capabilities.ansi;

    // s=3 gives 3-row block (H1), s=2 gives 2-row block (H2); H3+ fall back to ANSI.
    // ANSI codes must not be nested inside OSC 66 text payload.
    let text = if ctx.capabilities.kitty_text_size && level <= 2 {
        let size = TextSize::Scale(if level == 1 { 3 } else { 2 });
        sized_text(&raw, size)
    } else {
        let style = match level {
            1 => ctx.theme.heading1,
            2 => ctx.theme.heading2,
            3 => ctx.theme.heading3,
            _ => ctx.theme.heading4,
        };
        apply_style(&raw, style, ansi)
    };
    out.push_str(&text);
}

fn render_code_block(
    lang: Option<&str>,
    code: &str,
    ctx: &RenderContext,
    state: &mut RenderState,
    out: &mut String,
) {
    debug_assert!(ctx.width >= 1, "width must be positive");
    let rendered = match (&state.highlighter, ctx.capabilities.ansi) {
        (Some(highlighter), true) => highlighter.highlight_code(code, lang),
        _ => code.to_string(),
    };

    let label = lang.unwrap_or("code");
    let top = format!("+-- {label} --");
    let bottom = "+--";
    out.push_str(&apply_style(
        &top,
        ctx.theme.table_border,
        ctx.capabilities.ansi,
    ));
    out.push('\n');

    let lines = rendered.lines();
    let once = std::iter::once("").take(rendered.is_empty() as usize);
    for line in lines.chain(once) {
        let pipe = apply_style("| ", ctx.theme.table_border, ctx.capabilities.ansi);
        out.push_str(&pipe);
        out.push_str(line);
        out.push('\n');
    }

    out.push_str(&apply_style(
        bottom,
        ctx.theme.table_border,
        ctx.capabilities.ansi,
    ));
    out.push('\n');
}

fn render_list(
    ordered: bool,
    start: Option<u64>,
    items: &[Vec<Block>],
    ctx: &RenderContext,
    state: &mut RenderState,
    out: &mut String,
) -> Result<()> {
    for (item_idx, item) in items.iter().enumerate() {
        let marker_plain = if ordered {
            let n = start.unwrap_or(1) + item_idx as u64;
            format!("{n}.")
        } else {
            "*".to_string()
        };
        let marker = apply_style(&marker_plain, ctx.theme.list_marker, ctx.capabilities.ansi);
        let indent = " ".repeat(marker_plain.len() + 1);

        let mut item_text = String::new();
        for block in item {
            render_block(block, ctx, state, &mut item_text)?;
        }
        let item_text = item_text.trim_end_matches('\n');
        push_indented(out, &format!("{marker} "), &indent, item_text);
        if item_idx + 1 < items.len() && !item_text.is_empty() {
            out.push('\n');
        }
    }
    Ok(())
}

fn render_footnote_definition(
    name: &str,
    content: &[Block],
    ctx: &RenderContext,
    state: &mut RenderState,
    out: &mut String,
) -> Result<()> {
    let mut body = String::new();
    for block in content {
        render_block(block, ctx, state, &mut body)?;
    }
    let body = body.trim_end_matches('\n');
    let label = format!("[^{name}]: ");
    let label_styled = apply_style(&label, ctx.theme.link_url, ctx.capabilities.ansi);
    let indent = " ".repeat(label.len());
    push_indented(out, &label_styled, &indent, body);
    Ok(())
}

fn push_indented(out: &mut String, label: &str, indent: &str, body: &str) {
    let mut lines = body.lines();
    out.push_str(label);
    if let Some(first) = lines.next() {
        out.push_str(first);
    }
    out.push('\n');
    for line in lines {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(indent);
            out.push_str(line);
            out.push('\n');
        }
    }
}

fn render_image_block(alt: &str, path: &str, ctx: &RenderContext, out: &mut String) -> Result<()> {
    let p = Path::new(path);
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else {
        ctx.source_dir.join(p)
    };

    if ctx.capabilities.kitty_graphics
        && let Ok(seq) = render_image(
            &resolved,
            KittyImageOptions {
                max_width_cells: ctx.width.min(80) as u16,
                max_height_cells: None,
            },
        )
    {
        out.push_str(&seq);
        if !alt.trim().is_empty() {
            out.push('\n');
            out.push_str(alt);
        }
        return Ok(());
    }

    out.push_str(&format!("[image: {} - {}]", alt, path));
    Ok(())
}

fn spans_to_ansi_line(spans: &[super::inline::Span], ansi: bool) -> String {
    let mut line = String::new();
    for span in spans {
        line.push_str(&apply_style(&span.text, span.style, ansi));
    }
    line
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::markdown::ast::{Block, Inline};
    use crate::render::{Capabilities, RenderContext, render_document};
    use crate::theme::builtin::find_theme;

    #[test]
    fn renders_paragraph_and_heading() {
        let blocks = vec![
            Block::Heading {
                level: 1,
                content: vec![Inline::Text("Title".to_string())],
            },
            Block::Paragraph(vec![Inline::Text("hello world".to_string())]),
        ];
        let ctx = RenderContext {
            width: 80,
            theme: *find_theme("ansi").expect("theme exists"),
            capabilities: Capabilities::default(),
            source_dir: PathBuf::from("."),
            no_highlight: false,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("Title"));
        assert!(out.contains("hello world"));
    }

    #[test]
    fn image_fallback_is_readable() {
        let blocks = vec![Block::Image {
            alt: "Logo".to_string(),
            path: "assets/logo.png".to_string(),
            title: None,
        }];
        let ctx = RenderContext {
            width: 80,
            theme: *find_theme("ansi").expect("theme exists"),
            capabilities: Capabilities::default(),
            source_dir: PathBuf::from("."),
            no_highlight: false,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("[image: Logo - assets/logo.png]"));
    }

    #[test]
    fn highlighted_code_does_not_bleed_into_following_text() {
        let blocks = vec![
            Block::CodeBlock {
                lang: Some("rust".to_string()),
                code: "fn main() {}\n".to_string(),
            },
            Block::Paragraph(vec![Inline::Text("after".to_string())]),
        ];
        let ctx = RenderContext {
            width: 80,
            theme: *find_theme("ansi").expect("theme exists"),
            capabilities: Capabilities {
                ansi: true,
                ..Capabilities::default()
            },
            source_dir: PathBuf::from("."),
            no_highlight: false,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("\u{1b}[0m"));
        assert!(out.contains("\nafter\n"));
    }

    #[test]
    fn kitty_h1_has_extra_spacing() {
        let blocks = vec![
            Block::Heading {
                level: 1,
                content: vec![Inline::Text("mdlux".to_string())],
            },
            Block::Paragraph(vec![Inline::Text("Terminal Markdown renderer".to_string())]),
        ];
        let ctx = RenderContext {
            width: 80,
            theme: *find_theme("ansi").expect("theme exists"),
            capabilities: Capabilities {
                kitty_text_size: true,
                ..Capabilities::default()
            },
            source_dir: PathBuf::from("."),
            no_highlight: true,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("\u{1b}]66;s=3;mdlux\u{7}\n\n\n\n"));
        assert!(out.contains("Terminal Markdown renderer"));
    }
}
