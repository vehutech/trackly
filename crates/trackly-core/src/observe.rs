//! Turn a git commit into task progress. When a commit message references task ids
//! (e.g. `t3`, `closes t7`), Trackly attaches the commit as evidence and advances
//! those tasks — reconstructing *when* work happened straight from git history.
//!
//! This is deliberately conservative: it only touches tasks the commit explicitly
//! names, and it's idempotent (re-observing the same commit changes nothing).

use crate::git::CommitInfo;
use crate::model::{Evidence, Plan, Status};

/// Words that, when present in a commit message, mark referenced tasks as done.
const DONE_WORDS: &[&str] = &[
    "close",
    "closes",
    "closed",
    "fix",
    "fixes",
    "fixed",
    "done",
    "complete",
    "completed",
    "resolve",
    "resolves",
    "resolved",
    "finish",
    "finished",
    "ship",
    "shipped",
];

/// Apply a commit to the plan. Returns a human-readable line per task changed.
///
/// For each referenced, existing task not already crediting this commit:
/// - the commit (`<short> <subject>`) is attached as evidence;
/// - if the message contains a completion word, the task becomes `Done`;
/// - otherwise an `Open` task becomes `InProgress` (work has started on it).
pub fn apply_commit(plan: &mut Plan, commit: &CommitInfo) -> Vec<String> {
    let ids = referenced_task_ids(&commit.message);
    if ids.is_empty() {
        return Vec::new();
    }
    let completes = mentions_completion(&commit.message);
    let subject = commit.subject().to_string();
    let mut changes = Vec::new();

    for id in ids {
        let Some(task) = plan.task_mut(&id) else {
            continue;
        };
        // Idempotent: skip if this commit is already recorded on the task.
        if task
            .evidence
            .iter()
            .any(|e| e.note.starts_with(&commit.short))
        {
            continue;
        }

        task.evidence.push(Evidence::new(format!(
            "{} {}",
            commit.short,
            truncate(&subject, 72)
        )));
        task.updated_at = commit_time();

        let new_status = if completes {
            Some(Status::Done)
        } else if task.status == Status::Open {
            Some(Status::InProgress)
        } else {
            None
        };
        if let Some(s) = new_status {
            task.status = s;
        }
        changes.push(format!(
            "{} → {} ({})",
            id,
            task.status.label(),
            commit.short
        ));
    }
    changes
}

fn commit_time() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// Extract task ids (`t` followed by digits, e.g. `t12`) referenced in `text`,
/// de-duplicated in first-seen order and lowercased.
fn referenced_task_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for token in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.len() < 2 {
            continue;
        }
        let mut chars = token.chars();
        let first = chars.next().unwrap();
        if (first == 't' || first == 'T') && chars.clone().all(|c| c.is_ascii_digit()) {
            let id = format!("t{}", &token[1..]);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Whether the message contains a completion keyword (word-boundary aware).
fn mentions_completion(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| DONE_WORDS.contains(&w.to_lowercase().as_str()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Plan, Status, Task};

    fn commit(msg: &str) -> CommitInfo {
        CommitInfo {
            hash: "a1b2c3d4e5".into(),
            short: "a1b2c3d".into(),
            date: "2026-07-15".into(),
            message: msg.into(),
        }
    }

    fn plan_with(ids: &[(&str, Status)]) -> Plan {
        let mut p = Plan::new("t");
        for (id, st) in ids {
            let mut task = Task::new(*id, format!("task {id}"));
            task.status = *st;
            p.tasks.push(task);
        }
        p
    }

    #[test]
    fn completion_word_marks_done() {
        let mut p = plan_with(&[("t1", Status::Open)]);
        let changes = apply_commit(&mut p, &commit("closes t1: add the thing"));
        assert_eq!(p.task("t1").unwrap().status, Status::Done);
        assert_eq!(p.task("t1").unwrap().evidence.len(), 1);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn bare_reference_starts_progress() {
        let mut p = plan_with(&[("t2", Status::Open)]);
        apply_commit(&mut p, &commit("wip on t2"));
        assert_eq!(p.task("t2").unwrap().status, Status::InProgress);
    }

    #[test]
    fn is_idempotent() {
        let mut p = plan_with(&[("t1", Status::Open)]);
        apply_commit(&mut p, &commit("closes t1"));
        let again = apply_commit(&mut p, &commit("closes t1"));
        assert!(again.is_empty());
        assert_eq!(p.task("t1").unwrap().evidence.len(), 1);
    }

    #[test]
    fn ignores_unknown_and_nonrefs() {
        let mut p = plan_with(&[("t1", Status::Open)]);
        let changes = apply_commit(&mut p, &commit("refactor t99 and tests, total cleanup"));
        // t99 doesn't exist; "tests"/"total" are not task refs.
        assert!(changes.is_empty());
        assert_eq!(p.task("t1").unwrap().status, Status::Open);
    }
}
