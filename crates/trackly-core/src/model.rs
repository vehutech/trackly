//! The Trackly data model: a Plan is a list of Tasks the agent intends to do,
//! each carrying a status and the evidence that moved it there.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Execution status of a single task.
///
/// Scoring is line-weighted with half-credit partials (see [`crate::score`]):
/// `Done` = full credit, `Partial` = half, everything else = none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Not started.
    Open,
    /// The agent is actively working on it.
    InProgress,
    /// Meaningfully advanced but not finished (half credit).
    Partial,
    /// Finished.
    Done,
    /// Cannot proceed — waiting on a decision or dependency.
    Blocked,
}

impl Status {
    /// Fraction of credit this status contributes to completion.
    pub fn credit(self) -> f64 {
        match self {
            Status::Done => 1.0,
            Status::Partial => 0.5,
            Status::Open | Status::InProgress | Status::Blocked => 0.0,
        }
    }

    /// A short glyph used in the terminal and HTML report.
    pub fn glyph(self) -> &'static str {
        match self {
            Status::Done => "●",
            Status::Partial => "◐",
            Status::InProgress => "◔",
            Status::Open => "○",
            Status::Blocked => "✕",
        }
    }

    /// Human label used in reports and legends.
    pub fn label(self) -> &'static str {
        match self {
            Status::Done => "Done",
            Status::Partial => "Partial",
            Status::InProgress => "In progress",
            Status::Open => "Open",
            Status::Blocked => "Blocked",
        }
    }
}

/// A piece of evidence that a task moved — a commit, a file, or a free note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Free-text note, commit hash, or file path.
    pub note: String,
    /// When the evidence was recorded.
    pub at: DateTime<Utc>,
}

impl Evidence {
    pub fn new(note: impl Into<String>) -> Self {
        Evidence {
            note: note.into(),
            at: Utc::now(),
        }
    }
}

/// A single unit of work in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Stable short id (e.g. `t1`), used to address the task from the CLI.
    pub id: String,
    /// One-line description of the work.
    pub title: String,
    /// Optional grouping — a phase, sprint, or section heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Current execution status.
    pub status: Status,
    /// Relative weight for line-weighted scoring. Defaults to 1.
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Accumulated evidence, newest last.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_weight() -> u32 {
    1
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Task {
            id: id.into(),
            title: title.into(),
            group: None,
            status: Status::Open,
            weight: 1,
            evidence: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// The whole plan: the agent's intended work for this repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Report title, e.g. "Accounting Module".
    pub title: String,
    /// Where the plan came from — an agent name, a doc path, or "manual".
    #[serde(default)]
    pub source: Option<String>,
    pub tasks: Vec<Task>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Plan {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Plan {
            title: title.into(),
            source: None,
            tasks: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Find a task by id.
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Find a task by id, mutably.
    pub fn task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Allocate the next free `tN` id.
    pub fn next_id(&self) -> String {
        let mut n = self.tasks.len() + 1;
        loop {
            let candidate = format!("t{n}");
            if self.task(&candidate).is_none() {
                return candidate;
            }
            n += 1;
        }
    }

    /// Group names in first-seen order; ungrouped tasks fall under `None`.
    pub fn group_order(&self) -> Vec<Option<String>> {
        let mut seen = Vec::new();
        for t in &self.tasks {
            if !seen.contains(&t.group) {
                seen.push(t.group.clone());
            }
        }
        seen
    }
}

/// A point-in-time measurement, appended to history on every mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub at: DateTime<Utc>,
    pub percent: f64,
    pub done: usize,
    pub partial: usize,
    pub open: usize,
    pub total: usize,
}
