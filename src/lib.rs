use regex::Regex;
use std::{path::PathBuf, process::Command, sync::LazyLock};
use tempfile::tempdir;

pub type Err = Box<dyn std::error::Error>;

pub static GITHUB_USER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https?://(?:www\.)?github\.com/([^/?#]+)(?:[?#].*)?$").unwrap());

pub fn clone_repo(url: &str) -> Result<PathBuf, Err> {
    let dir = tempdir()?.keep();

    Command::new("git")
        .arg("clone")
        .arg("--filter=blob:none")
        .arg("--no-checkout")
        .arg(url)
        .arg(&dir)
        .status()?;

    Ok(dir)
}

pub async fn list_repos_for_user(user: &str) -> Result<Vec<String>, Err> {
    let octocrab = octocrab::instance();
    let mut repos = vec![];
    let mut page = 1;
    loop {
        let page_repos = octocrab
            .users(user)
            .repos()
            .per_page(100)
            .page(page)
            .send()
            .await?;
        let number_of_pages = page_repos.number_of_pages().unwrap_or(1);
        repos.extend(page_repos.items);
        page += 1;

        if page > number_of_pages {
            break;
        }
    }
    Ok(repos
        .into_iter()
        .filter(|repo| !repo.fork.unwrap())
        .map(|repo| repo.clone_url.unwrap().to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_user_matching() {
        for url in [
            "https://github.com/JorianWoltjer",
            "https://github.com/JorianWoltjer?tab=repositories",
            "https://github.com/JorianWoltjer#",
            "https://github.com/JorianWoltjer#hash",
            "https://github.com/JorianWoltjer?tab=overview&from=2022-12-01&to=2022-12-31",
            "https://github.com/JorianWoltjer?tab=overview&a=?&b=c",
        ] {
            let user = GITHUB_USER_RE
                .captures(url)
                .and_then(|caps| caps.get(1).map(|m| m.as_str()));
            assert_eq!(user, Some("JorianWoltjer"));
        }
    }

    #[test]
    fn github_user_not_matching() {
        for url in [
            "https://example.com",
            "https://github.com",
            "https://github.com/",
            "https://github.com/JorianWoltjer/jorianwoltjer.com",
        ] {
            let user = GITHUB_USER_RE
                .captures(url)
                .and_then(|caps| caps.get(1).map(|m| m.as_str()));
            assert_eq!(user, None);
        }
    }
}
