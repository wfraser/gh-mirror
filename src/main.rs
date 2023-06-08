use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context};
use clap::Parser;
use serde::Deserialize;
use serde_json::Deserializer;

/// Create a mirror of all repos of a Github user
#[derive(Debug, Parser)]
struct Args {
    /// Github username
    user: String,
}

#[derive(Debug, Deserialize)]
struct Repository {
    name: String,
    ssh_url: String,
}

#[derive(Debug, Deserialize)]
struct Error {
    message: String,
    documentation_url: Option<String>,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Github error: {}", self.message)?;
        if let Some(doc) = &self.documentation_url {
            write!(f, " ({doc})")?;
        }
        Ok(())
    }
}

fn get_repositories(user: &str) -> anyhow::Result<impl Iterator<Item = Repository>> {
    let out = Command::new("gh")
        .arg("api")
        .arg("--paginate")
        .arg(format!("users/{user}/repos"))
        .output()
        .context("failed to run gh api")?;

    if !out.status.success() {
        return serde_json::from_slice::<Error>(&out.stdout)
            .map_or_else(
                |e| Err(e).context("failed to deserialize error"),
                |e| Err(anyhow!(e))
            )
            .context(format!("failed to list repositories for user {user}"));
    }

    Ok(Deserializer::from_slice(&out.stdout)
        .into_iter::<Vec<Repository>>()
        .map(|r| r.context("failed to deserialize repos json"))
        .collect::<anyhow::Result<Vec<Vec<Repository>>>>()?
        .into_iter()
        .flatten()
    )
}

fn git_clone(path: &Path, url: &str) -> anyhow::Result<()> {
    Command::new("git")
        .arg("clone")
        .arg("--mirror")
        .arg("--origin")
        .arg("github")
        .arg(url)
        .arg(path)
        .status()
        .with_context(|| format!("failed to git clone {url}"))?;

    let mut hook = File::create(path.join("hooks").join("pre-receive"))
        .context("failed to create hooks/pre-receive")?;

    hook.write_all(b"#!/bin/sh\n\
    \n\
    echo \"Pushing to this repository is forbidden.\"\n\
    echo \"This is a mirror of a Github repository. Push there instead.\"\n\
    exit 1\n")
        .context("failed to write hooks/pre-receive")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = hook.metadata()?.permissions();
        perms.set_mode(perms.mode() | 0o111); // ugo+x
        hook.set_permissions(perms)
            .context("failed to set permissions on hooks/pre-receive")?;
    }

    Ok(())
}

fn git_update(path: &Path) -> anyhow::Result<()> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("remote")
        .arg("update")
        .arg("--prune")
        .status()
        .with_context(|| format!("failed to git remote update {path:?}"))?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let root = std::env::current_dir().context("failed to get cwd")?;
    for repo in get_repositories(&args.user)? {
        let path = root.join(&repo.name);
        if path.is_dir() {
            println!("repo {} exists; updating", repo.name);
            git_update(&path)?;
        } else {
            println!("cloning {}", repo.name);
            git_clone(&path, &repo.ssh_url)?;
        }
    }
    Ok(())
}
