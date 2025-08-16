use clap::Parser;
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashSet,
    fs::{self, remove_dir_all},
    io::{self, IsTerminal, Read},
    sync::Arc,
};
use tokio::sync::RwLock;
use tokio::sync::mpsc;

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
            // eprintln!(
            //     "Paste your URL below and press Ctrl+D, pipe output from another command, or use `-f` to read URLs from a file."
            // );
        }
        stdin.read_to_string(&mut input)?;
    }

    // TODO: indicatif progress during this, just a message
    // Turning input URLs into git repository URLs
    let mut git_urls = vec![];
    for url in input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
    {
        // eprintln!("Identifying type for {url}...");
        let repo_source = RepoSource::from_url(url).await?;
        // eprintln!("Listing repositories in {repo_source}...");
        let new_urls = repo_source.list_repos().await?;
        // eprintln!("Found {}", new_urls.len());
        git_urls.extend(new_urls);
    }

    let pb = ProgressBar::new(git_urls.len() as u64).with_style(
        ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len} (ETA {eta})").unwrap(),
    );
    let queue = Arc::new(RwLock::new(git_urls));
    let (tx, mut rx) = mpsc::unbounded_channel();
    // Thread pool for cloning
    // eprintln!("Downloading all histories...");
    for _ in 0..args.threads {
        let queue = queue.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            loop {
                let git_url = queue.write().await.pop();
                if let Some(git_url) = git_url {
                    // eprintln!("Worker {i} picked up {git_url}");
                    let dir = clone_repo(&git_url).await.unwrap();
                    let _ = tx.send(dir);
                } else {
                    // eprintln!("Worker {i} exited");
                    break;
                }
            }
        });
    }
    // Make sure that when all threads exit, all senders are dropped, closing the channel
    drop(tx);

    // Extract authors from cloned repos
    let results = Arc::new(RwLock::new(HashSet::new()));
    while let Some(dir) = rx.recv().await {
        let repo = Repository::open(&dir).unwrap();
        // eprintln!("Cloned into {dir:?}");
        let mut revwalk = repo.revwalk().unwrap();

        // Push all branches and tags to the revwalk
        for reference in repo.references().unwrap() {
            let reference = reference.unwrap();
            if let Some(oid) = reference.target() {
                revwalk.push(oid).unwrap();
            }
        }

        // Iterate through commits
        for commit in revwalk {
            let commit = commit.unwrap();
            let commit = repo.find_commit(commit).unwrap();
            let author = commit.author();
            // TODO: if invalid utf-8, hex escape it
            let name = author.name().unwrap_or_default();
            let email = author.email().unwrap_or_default();
            // eprintln!("Commit: {} by {name} <{email}>", commit.id());
            results
                .write()
                .await
                .insert((name.to_string(), email.to_string()));
        }

        // eprintln!("{}", "-".repeat(80));
        remove_dir_all(dir).unwrap();
        let name = repo
            .find_remote("origin")
            .unwrap()
            .url()
            .unwrap()
            .split('/')
            .last()
            .unwrap()
            .to_string();
        pb.set_message(name);
        pb.inc(1);
    }
    pb.set_message("");

    // Print formatted results
    let mut results = results.read().await.clone().into_iter().collect::<Vec<_>>();
    // results.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
    results.sort();
    for (name, email) in results {
        println!("{name} <{email}>");
    }

    Ok(())
}
