//! Optional git metadata for report headers. Git is *evidence*, never a trigger —
//! everything here degrades gracefully when the repo isn't a git repo or git is absent.

use std::path::Path;
use std::process::Command;

/// Lightweight repo context used to label a report.
#[derive(Debug, Clone, Default)]
pub struct RepoInfo {
    /// Current branch, if on one.
    pub branch: Option<String>,
    /// Short HEAD commit hash, if any.
    pub commit: Option<String>,
}

impl RepoInfo {
    /// Gather git metadata for the repo at `dir`. Never fails — missing data is `None`.
    pub fn gather(dir: &Path) -> RepoInfo {
        RepoInfo {
            branch: run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
            commit: run(dir, &["rev-parse", "--short", "HEAD"]),
        }
    }
}

/// Run a git command in `dir`, returning trimmed stdout on success.
fn run(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
