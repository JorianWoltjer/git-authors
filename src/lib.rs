use futures::TryStreamExt;
use regex::Regex;
use std::{fmt::Display, path::PathBuf, sync::LazyLock};
use tempfile::tempdir;
use tokio::process::Command;

pub mod cli;

pub type Err = Box<dyn std::error::Error>;

pub static GITHUB_USER_ORG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://(?:www\.)?github\.com/(?:([^/?#]+)(?:[?#].*)?|orgs/([^/?#]+)(?:.*))$")
        .unwrap()
});

pub enum RepoSource {
    GithubUser(String),
    GithubOrg(String),
    GitRepo(String),
}
impl RepoSource {
    pub async fn from_url(url: &str) -> Result<RepoSource, Err> {
        if let Some(caps) = GITHUB_USER_ORG_RE.captures(url) {
            let name = caps.get(1).unwrap_or_else(|| caps.get(2).unwrap()).as_str();
            let profile = octocrab::instance().users(name).profile().await?;
            Ok(match profile.r#type.as_str() {
                "User" => RepoSource::GithubUser(name.to_string()),
                "Organization" => RepoSource::GithubOrg(name.to_string()),
                _ => Err(format!("Unknown type {:?}", profile.r#type))?,
            })
        } else {
            Ok(Self::GitRepo(url.to_string()))
        }
    }

    pub async fn list_repos(&self) -> Result<Vec<String>, Err> {
        Ok(match self {
            Self::GitRepo(url) => vec![url.to_string()],
            Self::GithubUser(user) => {
                let octocrab = octocrab::instance();
                octocrab
                    .users(user)
                    .repos()
                    .per_page(100)
                    .send()
                    .await?
                    // TODO: use .total_count() to make progress bar
                    .into_stream(&octocrab)
                    .try_filter_map(|repo| async move {
                        Ok((!repo.fork.unwrap()).then_some(repo.clone_url.unwrap().to_string()))
                    })
                    .try_collect()
                    .await?
            }
            Self::GithubOrg(org) => {
                let octocrab = octocrab::instance();
                octocrab
                    .orgs(org)
                    .list_repos()
                    .per_page(100)
                    .send()
                    .await?
                    .into_stream(&octocrab)
                    .try_filter_map(|repo| async move {
                        Ok((!repo.fork.unwrap()).then_some(repo.clone_url.unwrap().to_string()))
                    })
                    .try_collect()
                    .await?
            }
        })
    }
}
impl Display for RepoSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitRepo(url) => write!(f, "Git Repository at {url}"),
            Self::GithubUser(user) => write!(f, "GitHub User {user:?}"),
            Self::GithubOrg(org) => write!(f, "GitHub Organization {org:?}"),
        }
    }
}

pub async fn clone_repo(url: &str) -> Result<PathBuf, Err> {
    let dir = tempdir()?.keep();

    let result = Command::new("git")
        .arg("clone")
        .arg("--filter=blob:none")
        .arg("--no-checkout")
        .arg(url)
        .arg(&dir)
        .output()
        .await?;
    if !result.status.success() {
        let output =
            String::from_utf8_lossy(&result.stdout) + String::from_utf8_lossy(&result.stderr);
        eprint!("{output}");
        return Err("git clone failed".into());
    }

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_url_re_matching() {
        for url in [
            "https://github.com/JorianWoltjer",
            "https://github.com/JorianWoltjer?tab=repositories",
            "https://github.com/JorianWoltjer#",
            "https://github.com/JorianWoltjer#hash",
            "https://github.com/JorianWoltjer?tab=overview&from=2022-12-01&to=2022-12-31",
            "https://github.com/JorianWoltjer?tab=overview&a=?&b=c",
            "https://github.com/orgs/JorianWoltjer",
            "https://github.com/orgs/JorianWoltjer/repositories",
            "https://github.com/orgs/JorianWoltjer/repositories/",
            "https://github.com/orgs/JorianWoltjer/repositories/x",
        ] {
            let user = GITHUB_USER_ORG_RE
                .captures(url)
                .inspect(|caps| {
                    dbg!(&caps);
                })
                .map(|caps| caps.get(1).unwrap_or_else(|| caps.get(2).unwrap()).as_str());
            assert_eq!(user, Some("JorianWoltjer"));
        }
    }

    #[test]
    fn github_url_re_not_matching() {
        for url in [
            "https://example.com",
            "https://github.com",
            "https://github.com/",
            "https://github.com/JorianWoltjer/jorianwoltjer.com",
        ] {
            let user = GITHUB_USER_ORG_RE
                .captures(url)
                .inspect(|caps| {
                    dbg!(&caps);
                })
                .map(|caps| caps.get(1).unwrap_or_else(|| caps.get(2).unwrap()).as_str());
            assert_eq!(user, None);
        }
    }
}
