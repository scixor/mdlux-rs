pub mod cli;
pub mod ffi;
pub mod highlight;
pub mod input;
pub mod kitty;
pub mod markdown;
pub mod render;
pub mod theme;
pub mod util;

use anyhow::Result;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::kitty::detect::detect_capabilities;
use crate::render::{RenderOptions, render_markdown};
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
        .and_then(|path| path.parent().map(|path| path.to_path_buf()))
        .unwrap_or_else(PathBuf::new);

    let capabilities = detect_capabilities(&cli);
    let options = RenderOptions {
        width: cli
            .width
            .unwrap_or_else(|| util::width::terminal_width().unwrap_or(80))
            .max(20),
        theme: find_theme(&cli.theme)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown theme: {}", cli.theme))?,
        capabilities,
        source_dir,
        no_highlight: cli.no_highlight,
        embed_media: capabilities.kitty_graphics,
        ..RenderOptions::default()
    };

    let output = render_markdown(&input, &options)?;
    print!("{}", output.text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Capabilities;

    #[test]
    fn renders_basic_text() {
        let test_input = "# Hello\n\nworld\n";
        let options = RenderOptions {
            width: 80,
            theme: find_theme("ansi").unwrap().clone(),
            capabilities: Capabilities::default(),
            no_highlight: true,
            ..RenderOptions::default()
        };
        let out = render_markdown(test_input, &options).expect("should render");
        assert!(out.text.contains("Hello"));
        assert!(out.text.contains("world"));
    }

    #[test]
    fn errors_on_unknown_theme() {
        let err = find_theme("missing-theme");
        assert!(err.is_none());
    }
}
