pub mod cli;
pub mod input;
use anyhow::Result;
pub fn run(cli: Cli) -> Result<()> {
    let (input, _) = input::read_input(cli.file.as_deref())?;
    print!("{input}");
    Ok(())
}
