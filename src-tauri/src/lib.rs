//! Trackly desktop backend.
//!
//! A machine-wide "like GitHub" view over the same `trackly-core` engine the CLI uses:
//! it discovers repos containing a `.trackly/` store under configured root folders,
//! summarizes their progress, and exports reports. All commands are thin calls into
//! `trackly-core`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use trackly_core::git::RepoInfo;
use trackly_core::report::{render_html, ReportMeta};
use trackly_core::score::Stats;
use trackly_core::Store;

/// How deep to walk each root looking for `.trackly` stores.
const SCAN_DEPTH: usize = 6;

/// Directory names never worth descending into while scanning.
const PRUNE: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    "vendor",
    "Library",
    ".cache",
    ".Trash",
];

// ============ view models (sent to the frontend as camelCase JSON) ============

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    path: String,
    name: String,
    title: String,
    percent: f64,
    done: usize,
    partial: usize,
    open: usize,
    total: usize,
    branch: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskView {
    id: String,
    title: String,
    status: String,
    date: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupView {
    name: String,
    percent: f64,
    tasks: Vec<TaskView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotView {
    at: String,
    percent: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDetail {
    summary: ProjectSummary,
    source: Option<String>,
    groups: Vec<GroupView>,
    history: Vec<SnapshotView>,
}

// ============ config (list of root folders to scan) ============

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("roots.json"))
}

fn read_roots(app: &tauri::AppHandle) -> Vec<String> {
    if let Ok(path) = config_path(app) {
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(roots) = serde_json::from_slice::<Vec<String>>(&bytes) {
                return roots;
            }
        }
    }
    // First run: default to the user's home directory.
    home_dir()
        .map(|h| vec![h.to_string_lossy().to_string()])
        .unwrap_or_default()
}

fn write_roots(app: &tauri::AppHandle, roots: &[String]) -> Result<(), String> {
    let path = config_path(app)?;
    let json = serde_json::to_vec_pretty(roots).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

// ============ scanning ============

/// Recursively find directories that contain a `.trackly/plan.json`.
fn find_projects(root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if out.len() > 500 {
        return; // safety cap
    }
    if root.join(".trackly").join("plan.json").is_file() {
        out.push(root.to_path_buf());
        // A tracked repo won't contain another tracked repo; stop descending.
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Skip pruned and hidden directories (but `.trackly` is matched above).
        if PRUNE.contains(&name.as_ref()) || name.starts_with('.') {
            continue;
        }
        find_projects(&path, depth - 1, out);
    }
}

fn summarize(path: &Path) -> Option<ProjectSummary> {
    let store = Store::at(path);
    let plan = store.load_plan().ok().flatten()?;
    let s = Stats::of(&plan.tasks);
    Some(ProjectSummary {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string()),
        title: plan.title,
        percent: s.percent,
        done: s.done,
        partial: s.partial,
        open: s.open_like(),
        total: s.total,
        branch: RepoInfo::gather(path).branch,
    })
}

// ============ commands ============

#[tauri::command]
fn get_roots(app: tauri::AppHandle) -> Vec<String> {
    read_roots(&app)
}

#[tauri::command]
fn add_root(app: tauri::AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut roots = read_roots(&app);
    if !roots.iter().any(|r| r == &path) {
        roots.push(path);
    }
    write_roots(&app, &roots)?;
    Ok(roots)
}

#[tauri::command]
fn remove_root(app: tauri::AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut roots = read_roots(&app);
    roots.retain(|r| r != &path);
    write_roots(&app, &roots)?;
    Ok(roots)
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle) -> Vec<ProjectSummary> {
    let mut found = Vec::new();
    for root in read_roots(&app) {
        find_projects(Path::new(&root), SCAN_DEPTH, &mut found);
    }
    found.sort();
    found.dedup();
    let mut summaries: Vec<ProjectSummary> = found.iter().filter_map(|p| summarize(p)).collect();
    // Most complete first.
    summaries.sort_by(|a, b| b.percent.partial_cmp(&a.percent).unwrap_or(std::cmp::Ordering::Equal));
    summaries
}

#[tauri::command]
fn get_project(path: String) -> Result<ProjectDetail, String> {
    let dir = PathBuf::from(&path);
    let store = Store::at(&dir);
    let plan = store
        .load_plan()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no plan found".to_string())?;

    let summary = summarize(&dir).ok_or_else(|| "could not read project".to_string())?;

    let groups = plan
        .group_order()
        .into_iter()
        .filter_map(|g| {
            let tasks: Vec<_> = plan.tasks.iter().filter(|t| t.group == g).collect();
            if tasks.is_empty() {
                return None;
            }
            let gs = Stats::of(tasks.iter().copied());
            Some(GroupView {
                name: g.unwrap_or_else(|| "Ungrouped".to_string()),
                percent: gs.percent,
                tasks: tasks
                    .iter()
                    .map(|t| TaskView {
                        id: t.id.clone(),
                        title: t.title.clone(),
                        status: format!("{:?}", t.status).to_lowercase(),
                        date: t.evidence.last().map(|e| e.at.format("%Y-%m-%d").to_string()),
                    })
                    .collect(),
            })
        })
        .collect();

    let history = store
        .history()
        .unwrap_or_default()
        .into_iter()
        .map(|s| SnapshotView {
            at: s.at.format("%Y-%m-%d %H:%M").to_string(),
            percent: s.percent,
        })
        .collect();

    Ok(ProjectDetail {
        summary,
        source: plan.source,
        groups,
        history,
    })
}

#[tauri::command]
fn export_report(path: String, subtitle: Option<String>) -> Result<String, String> {
    let dir = PathBuf::from(&path);
    let store = Store::at(&dir);
    let plan = store
        .load_plan()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no plan found".to_string())?;
    let meta = ReportMeta {
        subtitle,
        repo: RepoInfo::gather(&dir),
    };
    let html = render_html(&plan, &meta);
    let out = dir.join("trackly-report.html");
    fs::write(&out, html).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}

#[tauri::command]
fn open_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

// ============ auto-update ============
// Wired but inert until the updater is configured (see UPDATER.md): without a
// `plugins.updater` config, `check_update` returns Err and the UI shows no banner.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

/// Check the release feed for a newer signed build. `Ok(None)` = up to date;
/// `Err` = updater not configured yet (the UI treats both as "nothing to do").
#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version.clone(),
        notes: u.body.clone(),
    }))
}

/// Download and install the pending update, then restart the app.
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Err("no update available".to_string());
    };
    update
        .download_and_install(|_downloaded, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_roots,
            add_root,
            remove_root,
            list_projects,
            get_project,
            export_report,
            open_path,
            check_update,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Trackly desktop");
}
