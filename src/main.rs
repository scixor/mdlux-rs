use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = mdlux::cli::Cli::parse();
    mdlux::run(cli)
}
