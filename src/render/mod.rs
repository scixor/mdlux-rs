pub mod ansi;
pub mod blocks;
pub mod inline;
pub mod media;
pub mod table;
pub mod wrap;

use std::path::PathBuf;

use anyhow::Result;

use crate::highlight::syntect::Highlighter;
use crate::markdown::ast::Block;
use crate::theme::Theme;

pub use media::{
    MediaImageInput, MediaKind, MediaRequest, MediaResolution, MediaSlot, extract_media_requests,
    find_resolution, image_input, load_image, media_kind_from_link, prepare_image,
    requests_without_resolutions,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Capabilities {
    pub ansi: bool,
    pub kitty_text_size: bool,
    pub kitty_graphics: bool,
    pub kitty_hyperlinks: bool,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub width: usize,
    pub theme: Theme,
    pub capabilities: Capabilities,
    pub source_dir: PathBuf,
    pub no_highlight: bool,
    pub code_theme_name: Option<String>,
    pub embed_media: bool,
    pub cell_width_px: Option<u32>,
    pub cell_height_px: Option<u32>,
    pub max_media_width_cells: Option<u32>,
    pub max_media_height_cells: Option<u32>,
    pub media: Vec<MediaResolution>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 80,
            theme: crate::theme::builtin::find_theme("ansi")
                .expect("ansi theme exists")
                .clone(),
            capabilities: Capabilities::default(),
            source_dir: PathBuf::from("."),
            no_highlight: true,
            code_theme_name: None,
            embed_media: false,
            cell_width_px: None,
            cell_height_px: None,
            max_media_width_cells: None,
            max_media_height_cells: None,
            media: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderOutput {
    pub text: String,
    pub media_slots: Vec<MediaSlot>,
    pub media_requests: Vec<MediaRequest>,
    pub line_count: usize,
}

pub struct RenderContext<'media> {
    pub width: usize,
    pub theme: Theme,
    pub capabilities: Capabilities,
    pub source_dir: PathBuf,
    pub no_highlight: bool,
    pub code_theme_name: Option<String>,
    pub embed_media: bool,
    pub cell_width_px: Option<u32>,
    pub cell_height_px: Option<u32>,
    pub max_media_width_cells: Option<u32>,
    pub max_media_height_cells: Option<u32>,
    pub media: &'media [MediaResolution],
}

impl<'options> From<&'options RenderOptions> for RenderContext<'options> {
    fn from(options: &'options RenderOptions) -> Self {
        Self {
            width: options.width,
            theme: options.theme.clone(),
            capabilities: options.capabilities,
            source_dir: options.source_dir.clone(),
            no_highlight: options.no_highlight,
            code_theme_name: options.code_theme_name.clone(),
            embed_media: options.embed_media,
            cell_width_px: options.cell_width_px,
            cell_height_px: options.cell_height_px,
            max_media_width_cells: options.max_media_width_cells,
            max_media_height_cells: options.max_media_height_cells,
            media: &options.media,
        }
    }
}

pub fn render_markdown(source: &str, options: &RenderOptions) -> Result<RenderOutput> {
    let blocks = crate::markdown::parser::parse_markdown(source);
    let ctx = RenderContext::from(options);
    let mut state = blocks::RenderState {
        highlighter: (ctx.capabilities.ansi && !ctx.no_highlight).then(|| {
            Highlighter::new(
                ctx.code_theme_name
                    .as_deref()
                    .unwrap_or(ctx.theme.name_str()),
            )
        }),
        media_slots: Vec::new(),
    };
    let mut out = String::new();
    for block in &blocks {
        blocks::render_block(block, &ctx, &mut state, &mut out)?;
    }

    Ok(RenderOutput {
        line_count: out.lines().count(),
        media_slots: state.media_slots,
        media_requests: requests_without_resolutions(&blocks, ctx.media),
        text: out,
    })
}

pub fn render_document(blocks: &[Block], ctx: &RenderContext<'_>) -> Result<String> {
    let mut out = String::new();
    let highlighter = (ctx.capabilities.ansi && !ctx.no_highlight).then(|| {
        Highlighter::new(
            ctx.code_theme_name
                .as_deref()
                .unwrap_or(ctx.theme.name_str()),
        )
    });
    let mut state = blocks::RenderState {
        highlighter,
        media_slots: Vec::new(),
    };
    for block in blocks {
        blocks::render_block(block, ctx, &mut state, &mut out)?;
    }
    Ok(out)
}
