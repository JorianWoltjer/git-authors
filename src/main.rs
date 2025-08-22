use clap::Parser;
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashSet,
    fs::remove_dir_all,
    io::{self, IsTerminal, Read},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::sync::mpsc;

use gitauthors::{Err, RepoSource, cli::Args, clone_repo};

#[tokio::main]
async fn main() -> Result<(), Err> {
    let mut args = Args::parse();
    if args.urls.is_empty() {
        let mut stdin = io::stdin();
        if stdin.is_terminal() {
            eprintln!(
                "[!] Tip: Paste your URL(s) below and press Ctrl+D, or pipe output from another command."
            );
        }
        let mut input = String::new();
        stdin.read_to_string(&mut input)?;
        args.urls.extend(
            input
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>(),
        );
    };

    let prep_pb = ProgressBar::new_spinner()
        .with_style(ProgressStyle::default_spinner().tick_chars("◜◠◝◞◡◟✓"));
    prep_pb.enable_steady_tick(Duration::from_millis(100));

    // Turning input URLs into git repository URLs
    let mut git_urls = vec![];
    for url in &args.urls {
        prep_pb.set_message(format!("Identifying {url}..."));
        let repo_source = RepoSource::from_url(url).await?;
        prep_pb.set_message(format!("Listing repositories of {repo_source}..."));
        let new_urls = repo_source.list_repos().await?;
        git_urls.extend(new_urls);
    }
    prep_pb.finish_with_message(format!(
        "Found {} repositor{} from {} source{}",
        git_urls.len(),
        if git_urls.len() == 1 { "y" } else { "ies" },
        args.urls.len(),
        if args.urls.len() == 1 { "" } else { "s" }
    ));

    let pb = ProgressBar::new(git_urls.len() as u64).with_style(
        ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len} ({per_sec}, ETA {eta})")
            .unwrap(),
    );
    pb.set_message("Cloning all repositories...");
    let queue = Arc::new(RwLock::new(git_urls));
    let (tx, mut rx) = mpsc::unbounded_channel();
    // Thread pool for cloning
    for _ in 0..args.threads {
        let queue = queue.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            loop {
                let git_url = queue.write().await.pop();
                if let Some(git_url) = git_url {
                    // let dir = clone_repo(&git_url).await.unwrap();
                    match clone_repo(&git_url).await {
                        Ok(dir) => {
                            let _ = tx.send(dir);
                        }
                        Err(e) => eprintln!("WARNING: {e}, skipping {git_url}\n"),
                    }
                } else {
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
            let name = String::from_utf8_lossy(author.name_bytes());
            let email = String::from_utf8_lossy(author.email_bytes());
            results
                .write()
                .await
                .insert((name.to_string(), email.to_string()));
        }

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
    pb.finish_with_message("✓");

    // Print formatted results
    let mut results = results.read().await.clone().into_iter().collect::<Vec<_>>();
    results.sort();
    for (name, email) in results {
        println!("{name} <{email}>");
    }

    Ok(())
}
