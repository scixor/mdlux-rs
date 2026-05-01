pub mod cli;
pub mod highlight;
pub mod input;
pub mod kitty;
pub mod markdown;
pub mod render;
pub mod theme;
pub mod util;

use std::path::Path;

use anyhow::Result;

use crate::cli::Cli;
use crate::kitty::detect::detect_capabilities;
use crate::markdown::parser::parse_markdown;
use crate::render::{RenderContext, render_document};
use crate::theme::builtin::{find_theme, list_theme_names};

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

    let theme = find_theme(&cli.theme)
        .ok_or_else(|| anyhow::anyhow!("unknown theme: {}", cli.theme))?
        .clone();

    let capabilities = detect_capabilities(&cli);
    let blocks = parse_markdown(&input);

    let ctx = RenderContext {
        width,
        theme,
        capabilities,
        source_dir,
        no_highlight: cli.no_highlight,
    };

    let output = render_document(&blocks, &ctx)?;
    print!("{output}");
    Ok(())
}
