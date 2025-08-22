use clap::Parser;

// Enumerate authors in Git logs of large sets of repositories for OSINT, to find names and emails
#[derive(Parser)]
#[command(name = "gitauthors")]
pub struct Args {
    /// Number of simultaneous threads to clone with
    #[arg(short, long, default_value = "10")]
    pub threads: usize,

    /// URLs of repositories, users or orgnizations
    pub urls: Vec<String>,
}
