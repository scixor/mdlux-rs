use crate::cli::{Cli, FeatureMode};
use crate::render::Capabilities;

fn detect_kitty_env() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.contains("kitty"))
}

fn resolve_mode(mode: FeatureMode, is_plain: bool, default: bool) -> bool {
    if is_plain {
        return false;
    }
    match mode {
        FeatureMode::Always => true,
        FeatureMode::Never => false,
        FeatureMode::Auto => default,
    }
}

pub fn detect_capabilities(cli: &Cli) -> Capabilities {
    let is_plain = cli.plain;
    let is_kitty = detect_kitty_env();

    let kitty_text_size = resolve_mode(cli.kitty, is_plain, is_kitty);
    let kitty_graphics = resolve_mode(cli.images, is_plain, is_kitty);

    Capabilities {
        ansi: !is_plain,
        kitty_text_size,
        kitty_graphics,
        kitty_hyperlinks: kitty_text_size,
    }
}
