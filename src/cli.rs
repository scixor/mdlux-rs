use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FeatureMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Parser)]
#[command(name = "mdlux")]
#[command(about = "Render Markdown for terminals")]
pub struct Cli {
    pub file: Option<PathBuf>,

    #[arg(long)]
    pub width: Option<usize>,

    #[arg(long, default_value = "ansi")]
    pub theme: String,

    #[arg(long, value_enum, default_value_t = FeatureMode::Auto)]
    pub kitty: FeatureMode,

    #[arg(long, value_enum, default_value_t = FeatureMode::Auto)]
    pub images: FeatureMode,

    #[arg(long)]
    pub no_highlight: bool,

    #[arg(long)]
    pub plain: bool,

    #[arg(long)]
    pub list_themes: bool,
}
