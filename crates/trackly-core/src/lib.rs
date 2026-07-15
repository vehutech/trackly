//! Trackly core engine.
//!
//! Trackly rides alongside a coding agent: the agent hands over its plan, marks
//! tasks as it works, and Trackly measures weighted completion and renders reports.
//! This crate is UI-agnostic — the CLI, and later the desktop app and MCP server,
//! are thin shells over it.

pub mod git;
pub mod model;
pub mod observe;
pub mod parse;
pub mod report;
pub mod score;
pub mod store;

use std::path::{Path, PathBuf};

pub use model::{Evidence, Plan, Snapshot, Status, Task};
pub use score::Stats;
pub use store::Store;

/// Candidate planning-doc filenames Trackly will seed a plan from, in priority order.
/// Case-insensitive; searched in the repo root and a `docs/` subdirectory.
pub const SEED_DOCS: &[&str] = &[
    "plan.md",
    "tasks.md",
    "todo.md",
    "goals.md",
    "roadmap.md",
    "architecture.md",
    "claude.md",
];

/// Find the first existing planning doc under `root` (root and `root/docs`).
/// Used to seed a plan on `init`, or to nudge the user when nothing is found.
pub fn find_seed_doc(root: &Path) -> Option<PathBuf> {
    let dirs = [root.to_path_buf(), root.join("docs")];
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        // Collect lowercase filename -> path for case-insensitive matching.
        let present: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
            .collect();
        for want in SEED_DOCS {
            if let Some((_, path)) = present.iter().find(|(name, _)| name == want) {
                return Some(path.clone());
            }
        }
    }
    None
}
