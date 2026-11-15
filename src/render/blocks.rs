use anyhow::Result;

use crate::kitty::text_size::{TextSize, sized_text};
use crate::markdown::ast::{Block, Inline};

use crate::render::RenderContext;
use crate::render::media::{
    MediaKind, MediaSlot, PreparedImage, encode_image, find_resolution, image_input, load_image,
    measure_image, prepare_image,
};

use super::ansi::apply_style;
use super::inline::render_inlines_to_spans;
use super::table::{RenderedCellMedia, RenderedTable, render_table_media};
use super::wrap::wrap_spans;

pub struct RenderState {
    pub highlighter: Option<crate::highlight::syntect::Highlighter>,
    pub media_slots: Vec<MediaSlot>,
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
            render_paragraph(inlines, ctx, state, out)?;
        }
        Block::CodeBlock { lang, code } => {
            render_code_block(lang.as_deref(), code, ctx, state, out);
            out.push('\n');
        }
        Block::BlockQuote(blocks) => {
            let first_slot = state.media_slots.len();
            let line_offset = out.lines().count();
            let mut quoted = String::new();
            for block in blocks {
                render_block(block, ctx, state, &mut quoted)?;
            }
            preserve_media_lines(&mut quoted, &state.media_slots[first_slot..]);
            offset_nested_slots(&mut state.media_slots[first_slot..], line_offset, 2);
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
            let row_media: Vec<Vec<Vec<RenderedCellMedia>>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| table_cell_media(cell, ctx))
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            let rendered = RenderedTable {
                headers: headers.iter().map(|h| Inline::plain_text(h)).collect(),
                rows: rows
                    .iter()
                    .map(|row| row.iter().map(|cell| Inline::plain_text(cell)).collect())
                    .collect(),
                aligns: aligns.clone(),
                media: row_media,
            };
            let line_offset = out.lines().count();
            let rendered_table = render_table_media(
                &rendered,
                ctx.width,
                ctx.capabilities.ansi,
                ctx.theme.table_header,
                ctx.theme.table_border,
                line_offset,
                &mut state.media_slots,
            );
            out.push_str(&rendered_table);
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
            let is_inline = find_resolution(ctx.media, path)
                .is_some_and(|r| r.display_mode == crate::render::media::MediaDisplayMode::Inline);
            render_image_block(alt, path, ctx, state, out)?;
            out.push('\n');
            if !is_inline {
                out.push('\n');
            }
        }
    }
    Ok(())
}

fn table_cell_media(inlines: &[Inline], ctx: &RenderContext<'_>) -> Result<Vec<RenderedCellMedia>> {
    if !ctx.capabilities.kitty_graphics {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    collect_table_cell_media(inlines, ctx, &mut out)?;
    Ok(out)
}

fn collect_table_cell_media(
    inlines: &[Inline],
    ctx: &RenderContext<'_>,
    out: &mut Vec<RenderedCellMedia>,
) -> Result<()> {
    for inline in inlines {
        match inline {
            Inline::Image { alt, path, .. } => {
                if let Some(prepared) = prepare_media_image(path, ctx)? {
                    out.push(RenderedCellMedia {
                        source: path.to_string(),
                        alt: alt.to_string(),
                        width_cells: prepared.width_cells,
                        height_cells: prepared.height_cells,
                        sequence: ctx.embed_media.then_some(prepared.sequence),
                    });
                }
            }
            Inline::Emph(inner) | Inline::Strong(inner) | Inline::Strike(inner) => {
                collect_table_cell_media(inner, ctx, out)?;
            }
            Inline::Link { text, .. } => collect_table_cell_media(text, ctx, out)?,
            _ => {}
        }
    }
    Ok(())
}

fn prepare_media_image(path: &str, ctx: &RenderContext<'_>) -> Result<Option<PreparedImage>> {
    let resolution = find_resolution(ctx.media, path);
    let override_width = resolution.and_then(|item| item.width_cells);
    let override_height = resolution.and_then(|item| item.height_cells);
    let result = (|| -> Result<Option<PreparedImage>> {
        let Some(input) = image_input(path, &ctx.source_dir, ctx.media) else {
            return Ok(None);
        };
        let image = load_image(input)?;
        let prepared = if ctx.embed_media {
            prepare_image(
                &image,
                ctx.width,
                ctx.max_media_width_cells,
                ctx.max_media_height_cells,
                ctx.cell_width_px,
                ctx.cell_height_px,
                override_width,
                override_height,
            )?
        } else {
            let (width_cells, height_cells) = measure_image(
                image.width(),
                image.height(),
                ctx.width,
                ctx.max_media_width_cells,
                ctx.max_media_height_cells,
                ctx.cell_width_px,
                ctx.cell_height_px,
                override_width,
                override_height,
            )?;
            PreparedImage {
                sequence: String::new(),
                width_cells,
                height_cells,
            }
        };
        Ok(Some(prepared))
    })();
    if resolution.is_some() {
        result
    } else {
        Ok(result.ok().flatten())
    }
}

fn render_paragraph(
    inlines: &[Inline],
    ctx: &RenderContext<'_>,
    state: &mut RenderState,
    out: &mut String,
) -> Result<()> {
    if !ctx.capabilities.kitty_graphics {
        render_paragraph_text(inlines, ctx, out);
        return Ok(());
    }

    let mut remaining = inlines.to_vec();
    while let Some((before, image, after)) = split_inline_sequence(&remaining) {
        if !before.is_empty() {
            render_paragraph_text(&before, ctx, out);
        }
        let Inline::Image { alt, path, .. } = image else {
            unreachable!("split returns an image")
        };
        render_image_block(&alt, &path, ctx, state, out)?;
        out.push('\n');
        out.push('\n');
        remaining = after;
    }
    if !remaining.is_empty() {
        render_paragraph_text(&remaining, ctx, out);
    }
    Ok(())
}

fn render_paragraph_text(inlines: &[Inline], ctx: &RenderContext<'_>, out: &mut String) {
    let spans = render_inlines_to_spans(inlines, ctx);
    for line in wrap_spans(&spans, ctx.width) {
        out.push_str(&spans_to_ansi_line(&line, ctx.capabilities.ansi));
        out.push('\n');
    }
    out.push('\n');
}

fn split_inline_sequence(inlines: &[Inline]) -> Option<(Vec<Inline>, Inline, Vec<Inline>)> {
    for (index, inline) in inlines.iter().enumerate() {
        let Some((before, image, after)) = split_inline_image(inline) else {
            continue;
        };
        let mut head = inlines[..index].to_vec();
        head.extend(before);
        let mut tail = after.into_iter().collect::<Vec<_>>();
        tail.extend_from_slice(&inlines[index + 1..]);
        return Some((head, image, tail));
    }
    None
}

fn split_inline_image(inline: &Inline) -> Option<(Vec<Inline>, Inline, Vec<Inline>)> {
    if matches!(inline, Inline::Image { .. }) {
        return Some((Vec::new(), inline.clone(), Vec::new()));
    }
    let (before, image, after) = match inline {
        Inline::Emph(inner) => split_wrapped_image(inner, Inline::Emph),
        Inline::Strong(inner) => split_wrapped_image(inner, Inline::Strong),
        Inline::Strike(inner) => split_wrapped_image(inner, Inline::Strike),
        Inline::Link { text, dest, title } => split_wrapped_image(text, |text| Inline::Link {
            text,
            dest: dest.clone(),
            title: title.clone(),
        }),
        _ => None,
    }?;
    Some((
        before.into_iter().collect(),
        image,
        after.into_iter().collect(),
    ))
}

fn split_wrapped_image<F>(
    inlines: &[Inline],
    wrap: F,
) -> Option<(Option<Inline>, Inline, Option<Inline>)>
where
    F: Fn(Vec<Inline>) -> Inline + Copy,
{
    let (before, image, after) = split_inline_sequence(inlines)?;
    let before = (!before.is_empty()).then(|| wrap(before));
    let after = (!after.is_empty()).then(|| wrap(after));
    Some((before, image, after))
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
        let first_slot = state.media_slots.len();
        let line_offset = out.lines().count();
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
        let mut item_text = item_text.trim_end_matches('\n').to_string();
        preserve_media_lines(&mut item_text, &state.media_slots[first_slot..]);
        offset_nested_slots(
            &mut state.media_slots[first_slot..],
            line_offset,
            indent.len() as u32,
        );
        push_indented(out, &format!("{marker} "), &indent, &item_text);
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
    let first_slot = state.media_slots.len();
    let line_offset = out.lines().count();
    let mut body = String::new();
    for block in content {
        render_block(block, ctx, state, &mut body)?;
    }
    let mut body = body.trim_end_matches('\n').to_string();
    let label = format!("[^{name}]: ");
    preserve_media_lines(&mut body, &state.media_slots[first_slot..]);
    offset_nested_slots(
        &mut state.media_slots[first_slot..],
        line_offset,
        label.len() as u32,
    );
    let label_styled = apply_style(&label, ctx.theme.link_url, ctx.capabilities.ansi);
    let indent = " ".repeat(label.len());
    push_indented(out, &label_styled, &indent, &body);
    Ok(())
}

fn preserve_media_lines(body: &mut String, slots: &[MediaSlot]) {
    let required = slots
        .iter()
        .map(|slot| slot.line as usize + slot.height_cells as usize)
        .max()
        .unwrap_or(0);
    while body.lines().count() < required {
        body.push('\n');
    }
}

fn offset_nested_slots(slots: &mut [MediaSlot], line_offset: usize, column_offset: u32) {
    for slot in slots {
        slot.line += line_offset as u32;
        slot.column += column_offset;
    }
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

fn render_image_block(
    alt: &str,
    path: &str,
    ctx: &RenderContext<'_>,
    state: &mut RenderState,
    out: &mut String,
) -> Result<()> {
    let resolution = find_resolution(ctx.media, path);
    if resolution.is_some_and(|r| r.display_mode == crate::render::media::MediaDisplayMode::Inline) {
        out.push_str(&format!("[{alt}]"));
        return Ok(());
    }

    let fallback = |out: &mut String| out.push_str(&format!("[image: {alt} - {path}]"));

    if !ctx.capabilities.kitty_graphics {
        fallback(out);
        return Ok(());
    }
    let resolution = find_resolution(ctx.media, path);
    let has_dimensions =
        resolution.is_some_and(|item| item.width_cells.is_some() && item.height_cells.is_some());
    if ctx.embed_media
        && (ctx.cell_width_px.is_none() || ctx.cell_height_px.is_none())
        && !has_dimensions
    {
        let Some(input) = image_input(path, &ctx.source_dir, ctx.media) else {
            fallback(out);
            return Ok(());
        };
        match load_image(input) {
            Ok(image) => {
                let width = ctx
                    .max_media_width_cells
                    .map(|width| width as usize)
                    .unwrap_or(ctx.width)
                    .min(ctx.width)
                    .clamp(1, 80) as u32;
                out.push_str(&encode_image(&image, width, None)?);
                if !alt.trim().is_empty() {
                    out.push('\n');
                    out.push_str(alt);
                }
                return Ok(());
            }
            Err(error) if resolution.is_some() => return Err(error),
            Err(_) => {
                fallback(out);
                return Ok(());
            }
        }
    }
    let Some(prepared) = prepare_media_image(path, ctx)? else {
        fallback(out);
        return Ok(());
    };

    let line = out.lines().count() as u32;
    state.media_slots.push(MediaSlot {
        source: path.to_string(),
        alt: alt.to_string(),
        kind: MediaKind::Image,
        line,
        column: 0,
        width_cells: prepared.width_cells,
        height_cells: prepared.height_cells,
    });
    if ctx.embed_media {
        out.push_str(&prepared.sequence);
    } else {
        for _ in 0..prepared.height_cells {
            out.push('\n');
        }
    }
    if !alt.trim().is_empty() {
        out.push('\n');
        out.push_str(alt);
    }
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
    use crate::markdown::parser::parse_markdown;

    use crate::render::blocks::{RenderState, render_block};
    use crate::render::media::MediaResolution;
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
            theme: find_theme("ansi").expect("theme exists").to_owned(),
            capabilities: Capabilities::default(),
            source_dir: PathBuf::from("."),
            no_highlight: false,
            code_theme_name: None,
            media: &[],
            embed_media: false,
            cell_width_px: None,
            cell_height_px: None,
            max_media_width_cells: None,
            max_media_height_cells: None,
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
            theme: find_theme("ansi").expect("theme exists").to_owned(),
            capabilities: Capabilities::default(),
            source_dir: PathBuf::from("."),
            no_highlight: false,
            code_theme_name: None,
            media: &[],
            embed_media: false,
            cell_width_px: None,
            cell_height_px: None,
            max_media_width_cells: None,
            max_media_height_cells: None,
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
            theme: find_theme("ansi").expect("theme exists").to_owned(),
            capabilities: Capabilities {
                ansi: true,
                ..Capabilities::default()
            },
            source_dir: PathBuf::from("."),
            no_highlight: false,
            code_theme_name: None,
            media: &[],
            embed_media: false,
            cell_width_px: None,
            cell_height_px: None,
            max_media_width_cells: None,
            max_media_height_cells: None,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("\u{1b}[0m"));
        assert!(out.contains("\nafter\n"));
    }

    #[test]
    fn table_image_is_media_aware() {
        let path = std::env::temp_dir().join("mdlux_table_image.png");
        let img = image::RgbaImage::from_pixel(20, 40, image::Rgba([255, 0, 0, 255]));
        img.save(&path).expect("save png");

        let media = vec![MediaResolution {
            source: path.display().to_string(),
            path: Some(path.clone()),
            ..MediaResolution::default()
        }];
        let ctx = RenderContext {
            width: 40,
            theme: find_theme("ansi").expect("theme exists").to_owned(),
            capabilities: Capabilities {
                ansi: false,
                kitty_graphics: true,
                ..Capabilities::default()
            },
            source_dir: PathBuf::from("."),
            no_highlight: true,
            code_theme_name: None,
            media: &media,
            embed_media: false,
            cell_width_px: Some(10),
            cell_height_px: Some(20),
            max_media_width_cells: Some(5),
            max_media_height_cells: Some(5),
        };
        let blocks = parse_markdown(&format!(
            "| A | B |\n|---|---|\n| ![]({}) | value |\n",
            path.display()
        ));
        let mut out = String::new();
        let mut state = RenderState {
            highlighter: None,
            media_slots: Vec::new(),
        };
        render_block(&blocks[0], &ctx, &mut state, &mut out).expect("table render");
        assert_eq!(state.media_slots.len(), 1);
        let slot = &state.media_slots[0];
        assert_eq!(slot.width_cells, 2);
        assert_eq!(slot.height_cells, 2);
        assert_eq!(slot.column, 2);
        assert!(out.lines().count() >= 7, "table should reserve image lines");
        for line in out.lines() {
            assert!(
                crate::util::width::visible_width(line) <= ctx.width,
                "line too wide: {:?}",
                line
            );
        }
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
            theme: find_theme("ansi").expect("theme exists").to_owned(),
            capabilities: Capabilities {
                kitty_text_size: true,
                ..Capabilities::default()
            },
            source_dir: PathBuf::from("."),
            no_highlight: true,
            code_theme_name: None,
            media: &[],
            embed_media: false,
            cell_width_px: None,
            cell_height_px: None,
            max_media_width_cells: None,
            max_media_height_cells: None,
        };
        let out = render_document(&blocks, &ctx).expect("render should work");
        assert!(out.contains("\u{1b}]66;s=3;mdlux\u{7}\n\n\n\n"));
        assert!(out.contains("Terminal Markdown renderer"));
    }
}
