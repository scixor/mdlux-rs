use crate::cli::{Cli, FeatureMode};
use crate::render::Capabilities;

fn detect_kitty_env() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.contains("kitty"))
}

pub fn detect_capabilities(cli: &Cli) -> Capabilities {
    let is_plain = cli.plain;
    let is_kitty = detect_kitty_env();

    let resolve = |mode: FeatureMode| match mode {
        FeatureMode::Always => !is_plain,
        FeatureMode::Never => false,
        FeatureMode::Auto => is_kitty && !is_plain,
    };

    Capabilities {
        ansi: !is_plain,
        kitty_text_size: resolve(cli.text_size),
        kitty_graphics: resolve(cli.images),
        kitty_hyperlinks: resolve(cli.text_size),
    }
}
