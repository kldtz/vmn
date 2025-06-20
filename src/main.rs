use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use std::path::PathBuf;
use vmn::add::add;
use vmn::init::init;
use vmn::review::review;
use vmn::stats::stats;

#[derive(clap::ValueEnum, Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Command {
    Init,
    Add,
    Review,
    Stats,
}

/// Spaced-repetition CLI VergissMeinNicht.
#[derive(Parser)]
struct Cli {
    /// What to do
    command: Command,
    /// Path to card box (CSV file)
    path: PathBuf,
    /// Silent
    #[clap(short, long, default_value = "false")]
    silent: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Init => init(&args.path, args.silent),
        Command::Add => add(&args.path, args.silent),
        Command::Review => review(&args.path),
        Command::Stats => stats(&args.path),
    }
}
