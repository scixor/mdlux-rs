pub mod cli;
pub mod highlight;
pub mod input;
pub mod kitty;
pub mod markdown;
pub mod render;
pub mod theme;
pub mod util;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::Cli;
use crate::kitty::detect::detect_capabilities;
use crate::markdown::parser::parse_markdown;
use crate::render::{Capabilities, RenderContext, render_document};
use crate::theme::builtin::{default_theme_name, find_theme, list_theme_names};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub width: usize,
    pub theme: String,
    pub ansi: bool,
    pub kitty_text_size: bool,
    pub kitty_graphics: bool,
    pub kitty_hyperlinks: bool,
    pub no_highlight: bool,
    pub source_dir: PathBuf,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: util::width::terminal_width().unwrap_or(80).max(20),
            theme: default_theme_name().to_string(),
            ansi: true,
            kitty_text_size: false,
            kitty_graphics: false,
            kitty_hyperlinks: false,
            no_highlight: false,
            source_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

pub fn render_markdown_str(input: &str, opts: &RenderOptions) -> Result<String> {
    let theme = find_theme(&opts.theme)
        .ok_or_else(|| anyhow::anyhow!("unknown theme: {}", opts.theme))?
        .clone();
    let blocks = parse_markdown(input);
    let ctx = RenderContext {
        width: opts.width.max(20),
        theme,
        capabilities: Capabilities {
            ansi: opts.ansi,
            kitty_text_size: opts.kitty_text_size,
            kitty_graphics: opts.kitty_graphics,
            kitty_hyperlinks: opts.kitty_hyperlinks,
        },
        source_dir: opts.source_dir.clone(),
        no_highlight: opts.no_highlight,
    };
    render_document(&blocks, &ctx)
}

pub fn run(cli: Cli) -> Result<()> {
    if cli.list_themes {
        for name in list_theme_names() {
            println!("{name}");
        }
        return Ok(());
    }

    let (input, source_path) = input::read_input(cli.file.as_deref())?;
    let source_dir = source_path
        .as_deref()
        .and_then(Path::parent)
        .map(|p| p.to_path_buf())
        .unwrap_or(std::env::current_dir()?);

    let width = cli
        .width
        .unwrap_or_else(|| util::width::terminal_width().unwrap_or(80))
        .max(20); // if its less than 20 we can't do anything about it : )

    let capabilities = detect_capabilities(&cli);
    let opts = RenderOptions {
        width,
        theme: cli.theme,
        ansi: capabilities.ansi,
        kitty_text_size: capabilities.kitty_text_size,
        kitty_graphics: capabilities.kitty_graphics,
        kitty_hyperlinks: capabilities.kitty_hyperlinks,
        no_highlight: cli.no_highlight,
        source_dir,
    };

    let output = render_markdown_str(&input, &opts)?;
    print!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RenderOptions, render_markdown_str};

    #[test]
    fn render_markdown_str_renders_basic_text() {
        let opts = RenderOptions {
            ansi: false,
            ..RenderOptions::default()
        };
        let out = render_markdown_str("# Hello\n\nworld\n", &opts).expect("should render");
        assert!(out.contains("Hello"));
        assert!(out.contains("world"));
    }

    #[test]
    fn render_markdown_str_errors_on_unknown_theme() {
        let opts = RenderOptions {
            theme: "missing-theme".to_string(),
            ..RenderOptions::default()
        };
        let err = render_markdown_str("hello", &opts).expect_err("must fail");
        assert!(err.to_string().contains("unknown theme"));
    }
}
