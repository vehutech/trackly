//! Parse a markdown plan/checklist into a [`Plan`].
//!
//! Recognised structure:
//! - The first level-1 (`#`) heading, or a `title:` line, becomes the plan title.
//! - Level-2/3 headings (`##`, `###`) become task groups.
//! - List items (`-`, `*`, `+`, or `1.`) become tasks. A leading checkbox sets status:
//!   - `[ ]` → open, `[x]`/`[X]` → done, `[/]` → in progress, `[~]` → partial, `[-]` → blocked.
//!   - A list item with no checkbox is treated as an open task.

use crate::model::{Plan, Status, Task};

/// Parse markdown text into a plan. `fallback_title` is used if the doc has no heading.
pub fn parse_markdown(text: &str, fallback_title: &str) -> Plan {
    let mut title: Option<String> = None;
    let mut current_group: Option<String> = None;
    let mut tasks: Vec<Task> = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        // Headings.
        if let Some(h1) = line.strip_prefix("# ") {
            if title.is_none() {
                title = Some(h1.trim().to_string());
            }
            continue;
        }
        if let Some(h) = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))
        {
            current_group = Some(h.trim().to_string());
            continue;
        }

        // List items -> tasks.
        if let Some((status, body)) = parse_list_item(line) {
            if body.is_empty() {
                continue;
            }
            let id = format!("t{}", tasks.len() + 1);
            let mut task = Task::new(id, body);
            task.group = current_group.clone();
            task.status = status;
            tasks.push(task);
        }
    }

    let mut plan = Plan::new(title.unwrap_or_else(|| fallback_title.to_string()));
    plan.tasks = tasks;
    plan
}

/// If `line` is a markdown list item, return its (status, body). Otherwise `None`.
fn parse_list_item(line: &str) -> Option<(Status, String)> {
    // Strip a bullet marker (`- `, `* `, `+ `) or an ordered marker (`1. `, `12) `).
    let rest = if let Some(r) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        r
    } else {
        strip_ordered_marker(line)?
    };

    let rest = rest.trim_start();

    // Optional checkbox.
    if let Some(after) = rest.strip_prefix('[') {
        if let Some((mark, body)) = after.split_once(']') {
            let status = match mark.trim() {
                "" | " " => Status::Open,
                "x" | "X" => Status::Done,
                "/" => Status::InProgress,
                "~" => Status::Partial,
                "-" => Status::Blocked,
                // Unknown checkbox content: keep the whole thing as an open task.
                _ => return Some((Status::Open, rest.trim().to_string())),
            };
            return Some((status, body.trim().to_string()));
        }
    }

    // Plain bullet with no checkbox → open task.
    Some((Status::Open, rest.trim().to_string()))
}

/// Strip an ordered-list marker like `1. ` or `12) `; return the remainder.
fn strip_ordered_marker(line: &str) -> Option<&str> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after = &line[digits.len()..];
    after
        .strip_prefix(". ")
        .or_else(|| after.strip_prefix(") "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_groups_and_statuses() {
        let md = "\
# My Project

## Phase 1
- [x] set up the repo
- [/] wire the database
- [ ] write the tests

## Phase 2
- [~] draft the docs
- [-] deploy (blocked on creds)
- a plain bullet task
";
        let plan = parse_markdown(md, "fallback");
        assert_eq!(plan.title, "My Project");
        assert_eq!(plan.tasks.len(), 6);
        assert_eq!(plan.tasks[0].status, Status::Done);
        assert_eq!(plan.tasks[0].group.as_deref(), Some("Phase 1"));
        assert_eq!(plan.tasks[1].status, Status::InProgress);
        assert_eq!(plan.tasks[2].status, Status::Open);
        assert_eq!(plan.tasks[3].status, Status::Partial);
        assert_eq!(plan.tasks[4].status, Status::Blocked);
        assert_eq!(plan.tasks[5].status, Status::Open);
        assert_eq!(plan.tasks[5].title, "a plain bullet task");
    }

    #[test]
    fn falls_back_to_given_title() {
        let plan = parse_markdown("- [ ] just a task", "seeded");
        assert_eq!(plan.title, "seeded");
        assert_eq!(plan.tasks.len(), 1);
    }
}
