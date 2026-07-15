//! Line-weighted scoring with half-credit partials — the same method noted in
//! the reference report: "overall (half-credit partials)".

use crate::model::{Plan, Snapshot, Status, Task};
use chrono::Utc;

/// Aggregate completion stats for a set of tasks.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    /// Number of tasks marked `Done`.
    pub done: usize,
    /// Number of tasks marked `Partial`.
    pub partial: usize,
    /// Number of tasks marked `InProgress`.
    pub in_progress: usize,
    /// Number of tasks marked `Blocked`.
    pub blocked: usize,
    /// Number of tasks marked `Open`.
    pub open: usize,
    /// Total tasks.
    pub total: usize,
    /// Sum of weights across all tasks.
    pub weight_total: u32,
    /// Weighted, half-credit completion as a percentage in `0.0..=100.0`.
    pub percent: f64,
}

impl Stats {
    /// Compute stats over an iterator of tasks.
    pub fn of<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Stats {
        let mut s = Stats::default();
        let mut credited = 0.0_f64;
        for t in tasks {
            s.total += 1;
            s.weight_total += t.weight;
            credited += t.status.credit() * t.weight as f64;
            match t.status {
                Status::Done => s.done += 1,
                Status::Partial => s.partial += 1,
                Status::InProgress => s.in_progress += 1,
                Status::Blocked => s.blocked += 1,
                Status::Open => s.open += 1,
            }
        }
        s.percent = if s.weight_total == 0 {
            0.0
        } else {
            credited / s.weight_total as f64 * 100.0
        };
        s
    }

    /// "Open" for the headline tiles means everything not done or partial —
    /// open + in-progress + blocked. Matches the reference report's three-tile split.
    pub fn open_like(&self) -> usize {
        self.open + self.in_progress + self.blocked
    }
}

/// Compute stats for the whole plan.
pub fn stats(plan: &Plan) -> Stats {
    Stats::of(&plan.tasks)
}

/// Build a snapshot of the plan's current state.
pub fn snapshot(plan: &Plan) -> Snapshot {
    let s = stats(plan);
    Snapshot {
        at: Utc::now(),
        percent: s.percent,
        done: s.done,
        partial: s.partial,
        open: s.open_like(),
        total: s.total,
    }
}
