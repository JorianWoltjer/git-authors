use git2::Repository;
use std::{
    collections::HashSet,
    fs::remove_dir_all,
    io::{self, Read},
};

use git_authors::{Err, GITHUB_USER_RE, clone_repo};

#[tokio::main]
async fn main() -> Result<(), Err> {
    // TODO: argparse, allow file input and worker count and verbose output
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let mut results = HashSet::new();

    // TODO: multithread this with tokio spawn and worker count
    for url in input.lines() {
        if url.is_empty() {
            continue;
        }

        let git_urls = if let Some(caps) = GITHUB_USER_RE.captures(url) {
            let user = caps.get(1).unwrap().as_str();
            eprintln!("Getting repositories for GitHub user: {user}");
            git_authors::list_repos_for_user(user).await?
            // TODO: get from org
        } else {
            vec![url.to_string()]
        };

        eprintln!("Found {} git URLs", git_urls.len());
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
    }

    // Print unique authors
    for (name, email) in results {
        println!("{name} <{email}>");
    }

    Ok(())
}
