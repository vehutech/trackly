//! On-disk state: the `.trackly/` directory that sits in a repo like `.git/`.
//!
//! Layout:
//! - `.trackly/plan.json`      — the current plan (source of truth)
//! - `.trackly/history.jsonl`  — append-only snapshots, one JSON object per line

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::{Plan, Snapshot};
use crate::score;

/// Directory name Trackly stores its state in.
pub const DIR: &str = ".trackly";

/// Handle to a `.trackly` store rooted at some directory.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// A store at `<base>/.trackly`.
    pub fn at(base: impl AsRef<Path>) -> Self {
        Store {
            root: base.as_ref().join(DIR),
        }
    }

    /// Walk up from `start` looking for an existing `.trackly` directory,
    /// like git discovering `.git`. Returns `None` if none is found.
    pub fn discover(start: impl AsRef<Path>) -> Option<Store> {
        let mut dir = start.as_ref().to_path_buf();
        loop {
            if dir.join(DIR).is_dir() {
                return Some(Store {
                    root: dir.join(DIR),
                });
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    pub fn exists(&self) -> bool {
        self.root.is_dir()
    }

    /// The repo root that contains this `.trackly` directory.
    pub fn repo_root(&self) -> &Path {
        self.root.parent().unwrap_or(&self.root)
    }

    fn plan_path(&self) -> PathBuf {
        self.root.join("plan.json")
    }

    fn history_path(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }

    /// Create the `.trackly` directory (idempotent).
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("creating {}", self.root.display()))?;
        Ok(())
    }

    /// Load the plan, or `None` if it hasn't been written yet.
    pub fn load_plan(&self) -> Result<Option<Plan>> {
        let path = self.plan_path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let plan = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(plan))
    }

    /// Persist the plan and append a fresh snapshot to history.
    pub fn save_plan(&self, plan: &Plan) -> Result<()> {
        self.init()?;
        let json = serde_json::to_string_pretty(plan)?;
        fs::write(self.plan_path(), json)
            .with_context(|| format!("writing {}", self.plan_path().display()))?;
        self.append_snapshot(&score::snapshot(plan))?;
        Ok(())
    }

    fn append_snapshot(&self, snap: &Snapshot) -> Result<()> {
        use std::io::Write;
        let line = serde_json::to_string(snap)?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.history_path())
            .with_context(|| format!("opening {}", self.history_path().display()))?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    /// Read all snapshots in chronological order.
    pub fn history(&self) -> Result<Vec<Snapshot>> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path)?;
        let mut out = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(s) = serde_json::from_str::<Snapshot>(line) {
                out.push(s);
            }
        }
        Ok(out)
    }
}
