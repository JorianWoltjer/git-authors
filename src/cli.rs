use std::path::PathBuf;

use clap::Parser;

// Git author scraper
#[derive(Parser)]
#[command(name = "gitauthors")]
pub struct Args {
    /// Number of simultaneous threads to clone with
    #[arg(global = true, short = 'n', long, default_value = "10")]
    pub workers: usize,

    /// Path to a file URLs [default: stdin]
    #[arg(short, long)]
    pub file: Option<PathBuf>,
}
