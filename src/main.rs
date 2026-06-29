use anyhow::Result;
use clap::Parser;

mod cli;
mod codex;
mod provider;
mod relay;
mod toml_value;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::run(cli)
}
