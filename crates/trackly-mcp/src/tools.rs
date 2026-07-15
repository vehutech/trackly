//! Tool handlers — the operations an agent invokes over MCP. Each returns human-
//! readable text (or an `Err` string surfaced to the agent as a tool error).
//!
//! Every handler is a thin call into `trackly-core`, the same engine the CLI uses.

use std::path::PathBuf;

use chrono::Utc;
use serde_json::Value;

use trackly_core::git::RepoInfo;
use trackly_core::model::{Evidence, Plan, Status, Task};
use trackly_core::report::{render_html, ReportMeta};
use trackly_core::score::Stats;
use trackly_core::Store;

/// The repo Trackly operates on: `$TRACKLY_REPO`, else the process's working directory.
fn base_dir() -> PathBuf {
    std::env::var_os("TRACKLY_REPO")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Discover an existing `.trackly` store or target one at the base dir.
fn store() -> Store {
    let base = base_dir();
    Store::discover(&base).unwrap_or_else(|| Store::at(&base))
}

fn repo_name(store: &Store) -> String {
    store
        .repo_root()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

fn load_plan(store: &Store) -> Result<Plan, String> {
    store
        .load_plan()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no plan yet — call set_plan or add_task first".to_string())
}

/// Parse a status string from tool arguments into a [`Status`].
fn parse_status(s: &str) -> Result<Status, String> {
    match s.trim().to_lowercase().replace('-', "_").as_str() {
        "open" => Ok(Status::Open),
        "in_progress" | "inprogress" | "start" | "started" => Ok(Status::InProgress),
        "partial" => Ok(Status::Partial),
        "done" | "complete" | "completed" => Ok(Status::Done),
        "blocked" | "block" => Ok(Status::Blocked),
        other => Err(format!(
            "unknown status '{other}' (use: open, in_progress, partial, done, blocked)"
        )),
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

// ---- handlers ----

/// Replace the whole plan. `arguments`: `{ title?, tasks: [{title, group?, status?, weight?}] }`.
pub fn set_plan(args: &Value) -> Result<String, String> {
    let store = store();
    let tasks = args
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or_else(|| "`tasks` must be an array".to_string())?;

    let title = arg_str(args, "title")
        .map(str::to_string)
        .unwrap_or_else(|| repo_name(&store));

    let mut plan = Plan::new(title);
    plan.source = Some("agent (mcp)".to_string());
    for (i, t) in tasks.iter().enumerate() {
        let task_title =
            arg_str(t, "title").ok_or_else(|| format!("task #{} is missing `title`", i + 1))?;
        let mut task = Task::new(format!("t{}", i + 1), task_title);
        task.group = arg_str(t, "group").map(str::to_string);
        if let Some(s) = arg_str(t, "status") {
            task.status = parse_status(s)?;
        }
        if let Some(w) = t.get("weight").and_then(Value::as_u64) {
            task.weight = w.max(1) as u32;
        }
        plan.tasks.push(task);
    }
    let n = plan.tasks.len();
    store.save_plan(&plan).map_err(|e| e.to_string())?;
    Ok(format!(
        "Plan set with {n} task(s). {}",
        summary_line(&plan)
    ))
}

/// Append one task. `arguments`: `{ title, group?, weight? }`.
pub fn add_task(args: &Value) -> Result<String, String> {
    let store = store();
    let title = arg_str(args, "title").ok_or_else(|| "`title` is required".to_string())?;
    let mut plan = store
        .load_plan()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            let mut p = Plan::new(repo_name(&store));
            p.source = Some("agent (mcp)".to_string());
            p
        });

    let id = plan.next_id();
    let mut task = Task::new(id.clone(), title);
    task.group = arg_str(args, "group").map(str::to_string);
    if let Some(w) = args.get("weight").and_then(Value::as_u64) {
        task.weight = w.max(1) as u32;
    }
    plan.tasks.push(task);
    plan.updated_at = Utc::now();
    store.save_plan(&plan).map_err(|e| e.to_string())?;
    Ok(format!("Added {id}: {title}. {}", summary_line(&plan)))
}

/// Change a task's status. `arguments`: `{ id, status, evidence? }`.
pub fn update_task(args: &Value) -> Result<String, String> {
    let store = store();
    let id = arg_str(args, "id").ok_or_else(|| "`id` is required".to_string())?;
    let status =
        parse_status(arg_str(args, "status").ok_or_else(|| "`status` is required".to_string())?)?;
    let mut plan = load_plan(&store)?;
    let now = Utc::now();
    {
        let task = plan
            .task_mut(id)
            .ok_or_else(|| format!("no task with id '{id}'"))?;
        task.status = status;
        task.updated_at = now;
        if let Some(note) = arg_str(args, "evidence") {
            task.evidence.push(Evidence::new(note.to_string()));
        }
    }
    plan.updated_at = now;
    store.save_plan(&plan).map_err(|e| e.to_string())?;
    Ok(format!(
        "{id} → {} {}. {}",
        status.glyph(),
        status.label(),
        summary_line(&plan)
    ))
}

/// Return the current scoreboard as text. `arguments`: `{}`.
pub fn get_status(_args: &Value) -> Result<String, String> {
    let store = store();
    let plan = load_plan(&store)?;
    let mut out = format!("{}\n{}\n", plan.title, summary_line(&plan));
    for group in plan.group_order() {
        let tasks: Vec<_> = plan.tasks.iter().filter(|t| t.group == group).collect();
        if tasks.is_empty() {
            continue;
        }
        let gs = Stats::of(tasks.iter().copied());
        out.push_str(&format!(
            "\n{} ({:.0}%)\n",
            group.as_deref().unwrap_or("Ungrouped"),
            gs.percent
        ));
        for t in tasks {
            out.push_str(&format!("  {} {}  {}\n", t.status.glyph(), t.id, t.title));
        }
    }
    Ok(out)
}

/// Write an HTML report. `arguments`: `{ subtitle?, out? }`. Returns the file path.
pub fn generate_report(args: &Value) -> Result<String, String> {
    let store = store();
    let plan = load_plan(&store)?;
    let meta = ReportMeta {
        subtitle: arg_str(args, "subtitle").map(str::to_string),
        repo: RepoInfo::gather(store.repo_root()),
    };
    let html = render_html(&plan, &meta);

    let out = arg_str(args, "out")
        .map(PathBuf::from)
        .unwrap_or_else(|| store.repo_root().join("trackly-report.html"));
    std::fs::write(&out, html).map_err(|e| format!("writing {}: {e}", out.display()))?;
    Ok(format!(
        "Wrote report to {}. Open it and Print → Save as PDF.",
        out.display()
    ))
}

/// One-line completion summary, e.g. "40.9% — 4 done, 1 partial, 5 open (11 items)".
fn summary_line(plan: &Plan) -> String {
    let s = Stats::of(&plan.tasks);
    format!(
        "{:.1}% — {} done, {} partial, {} open ({} items)",
        s.percent,
        s.done,
        s.partial,
        s.open_like(),
        s.total
    )
}
