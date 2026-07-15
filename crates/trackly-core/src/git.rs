//! Optional git metadata for report headers. Git is *evidence*, never a trigger —
//! everything here degrades gracefully when the repo isn't a git repo or git is absent.

use std::path::{Path, PathBuf};
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

/// A single commit's metadata, used to attach evidence to tasks.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Full commit hash.
    pub hash: String,
    /// Abbreviated hash.
    pub short: String,
    /// Commit date (YYYY-MM-DD, from committer date).
    pub date: String,
    /// Full commit message (subject + body).
    pub message: String,
}

impl CommitInfo {
    /// The first line of the message.
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("").trim()
    }

    /// Look up a commit (default `HEAD`) in the repo at `dir`. `None` if unavailable.
    pub fn lookup(dir: &Path, refspec: &str) -> Option<CommitInfo> {
        // Unit-separator-delimited so the multi-line message stays intact as the last field.
        let raw = run(
            dir,
            &["show", "-s", "--format=%H%x1f%h%x1f%cs%x1f%B", refspec],
        )?;
        let mut parts = raw.splitn(4, '\u{1f}');
        Some(CommitInfo {
            hash: parts.next()?.trim().to_string(),
            short: parts.next()?.trim().to_string(),
            date: parts.next()?.trim().to_string(),
            message: parts.next().unwrap_or("").trim().to_string(),
        })
    }
}

/// The repo's hooks directory (honors `core.hooksPath`, worktrees, submodules).
pub fn hooks_dir(dir: &Path) -> Option<PathBuf> {
    let path = run(dir, &["rev-parse", "--git-path", "hooks"])?;
    let p = PathBuf::from(path);
    // `--git-path` may return a path relative to the repo root.
    Some(if p.is_absolute() { p } else { dir.join(p) })
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
