use clap::Parser;
use git2::Repository;
use std::{
    collections::HashSet,
    fs::{self, remove_dir_all},
    io::{self, IsTerminal, Read},
    sync::Arc,
};
use tokio::sync::RwLock;

use git_authors::{Err, RepoSource, cli::Args, clone_repo};

#[tokio::main]
async fn main() -> Result<(), Err> {
    let args = Args::parse();
    let mut input = String::new();
    if let Some(path) = args.file {
        input = fs::read_to_string(path)?
    } else {
        let mut stdin = io::stdin();
        if stdin.is_terminal() {
            eprintln!(
                "Paste your URL below and press Ctrl+D, pipe output from another command, or use `-f` to read URLs from a file."
            );
        }
        stdin.read_to_string(&mut input)?;
    }

    let mut git_urls = vec![];
    for url in input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
    {
        let repo_source = RepoSource::from_url(url).await?;
        eprintln!("Listing repositories in {repo_source}...");
        let new_urls = repo_source.list_repos().await?;
        eprintln!("Found {}", new_urls.len());
        git_urls.extend(new_urls);
    }

    // let queue = Arc::new(RwLock::new(git_urls));
    let mut results = HashSet::new();
    // TODO: multithread this with tokio spawn and worker count
    eprintln!("Downloading all histories...");
    // TODO: progress bar
    for git_url in git_urls {
        let dir = clone_repo(&git_url)?;
        eprintln!("Cloned {git_url} to {dir:?}");

        let repo = Repository::open(&dir)?;
        let mut revwalk = repo.revwalk()?;

        // Push all branches and tags to the revwalk
        for reference in repo.references()? {
            let reference = reference?;
            if let Some(oid) = reference.target() {
                revwalk.push(oid)?;
            }
        }

        // Iterate through commits
        for commit in revwalk {
            let commit = commit?;
            let commit = repo.find_commit(commit)?;
            let author = commit.author();
            let name = author.name().unwrap_or_default();
            let email = author.email().unwrap_or_default();
            eprintln!("Commit: {} by {name} <{email}>", commit.id());
            results.insert((name.to_string(), email.to_string()));
        }

        eprintln!("{}", "-".repeat(80));
        remove_dir_all(dir)?;
    }

    // Print unique authors
    for (name, email) in results {
        println!("{name} <{email}>");
    }

    Ok(())
}
