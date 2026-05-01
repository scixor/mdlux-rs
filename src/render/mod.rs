pub mod ansi;
pub mod blocks;
pub mod inline;
pub mod table;
pub mod wrap;

use std::path::PathBuf;

use anyhow::Result;

use crate::highlight::syntect::Highlighter;
use crate::markdown::ast::Block;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    pub ansi: bool,
    pub kitty_text_size: bool,
    pub kitty_graphics: bool,
    pub kitty_hyperlinks: bool,
}

pub struct RenderContext {
    pub width: usize,
    pub theme: Theme,
    pub capabilities: Capabilities,
    pub source_dir: PathBuf,
    pub no_highlight: bool,
}

pub fn render_document(blocks: &[Block], ctx: &RenderContext) -> Result<String> {
    let mut out = String::new();
    let highlighter = if ctx.capabilities.ansi && !ctx.no_highlight {
        Some(Highlighter::new(ctx.theme.name))
    } else {
        None
    };

    let mut state = blocks::RenderState { highlighter };
    for block in blocks {
        blocks::render_block(block, ctx, &mut state, &mut out)?;
    }
    Ok(out)
}
