//! The `trackly` command.
//!
//! Trackly lives in a repo like git. An agent pushes its plan, marks tasks as it
//! works, and you get a terminal scoreboard (`status`) and an HTML report (`report`).

mod ui;

use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::Deserialize;

use trackly_core::git::{self, CommitInfo, RepoInfo};
use trackly_core::model::{Evidence, Plan, Status, Task};
use trackly_core::report::{render_html, ReportMeta};
use trackly_core::{find_seed_doc, observe, parse, Store};

#[derive(Parser)]
#[command(
    name = "trackly",
    version,
    about = "Agent-native progress tracking that lives in a repo like git."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a `.trackly/` store, seeding a plan from an existing doc if one is found.
    Init,
    /// Manage the plan (the agent's intended work).
    #[command(subcommand)]
    Plan(PlanCmd),
    /// Update a task's status.
    #[command(subcommand)]
    Task(TaskCmd),
    /// Print the current scoreboard to the terminal.
    Status,
    /// Write an HTML progress report (open it and print-to-PDF).
    Report {
        /// Output path for the HTML file.
        #[arg(long, default_value = "trackly-report.html")]
        out: PathBuf,
        /// Optional project/org line for the header, e.g. "Geomed Pharmacy".
        #[arg(long)]
        subtitle: Option<String>,
    },
    /// Record a commit as evidence against tasks it references (used by the git hook).
    Observe {
        /// Commit to read (default HEAD).
        #[arg(long, default_value = "HEAD")]
        commit: String,
    },
    /// Manage the git post-commit hook that runs `observe` automatically.
    #[command(subcommand)]
    Hook(HookCmd),
}

#[derive(Subcommand)]
enum HookCmd {
    /// Install the post-commit hook into this repo's `.git/hooks`.
    Install,
    /// Remove Trackly's post-commit hook.
    Uninstall,
}

#[derive(Subcommand)]
enum PlanCmd {
    /// Replace the whole plan from a markdown checklist or JSON file (`-` = stdin).
    Set {
        /// Path to a `.md`/`.txt` checklist or a `.json` plan. Use `-` for stdin.
        file: String,
        /// Force the plan title (otherwise taken from the doc heading or repo name).
        #[arg(long)]
        title: Option<String>,
    },
    /// Append a single task to the plan.
    Add {
        /// The task description.
        title: String,
        /// Group / phase this task belongs to.
        #[arg(long)]
        group: Option<String>,
        /// Relative weight for scoring (default 1).
        #[arg(long, default_value_t = 1)]
        weight: u32,
    },
}

#[derive(Subcommand)]
enum TaskCmd {
    /// Mark a task done.
    Done(TaskArgs),
    /// Mark a task half-done (partial).
    Partial(TaskArgs),
    /// Mark a task in progress.
    Start(TaskArgs),
    /// Mark a task blocked.
    Block(TaskArgs),
    /// Reset a task to open.
    Open(TaskArgs),
}

#[derive(clap::Args)]
struct TaskArgs {
    /// Task id, e.g. `t3`.
    id: String,
    /// Attach a note / commit hash / file as evidence.
    #[arg(long)]
    evidence: Option<String>,
}

impl TaskCmd {
    fn parts(&self) -> (Status, &TaskArgs) {
        match self {
            TaskCmd::Done(a) => (Status::Done, a),
            TaskCmd::Partial(a) => (Status::Partial, a),
            TaskCmd::Start(a) => (Status::InProgress, a),
            TaskCmd::Block(a) => (Status::Blocked, a),
            TaskCmd::Open(a) => (Status::Open, a),
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {e:#}", ui::red("error:"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => cmd_init(),
        Command::Plan(PlanCmd::Set { file, title }) => cmd_plan_set(&file, title),
        Command::Plan(PlanCmd::Add {
            title,
            group,
            weight,
        }) => cmd_plan_add(title, group, weight),
        Command::Task(t) => {
            let (status, args) = t.parts();
            cmd_task(status, args)
        }
        Command::Status => cmd_status(),
        Command::Report { out, subtitle } => cmd_report(out, subtitle),
        Command::Observe { commit } => cmd_observe(&commit),
        Command::Hook(HookCmd::Install) => cmd_hook_install(),
        Command::Hook(HookCmd::Uninstall) => cmd_hook_uninstall(),
    }
}

/// Open the store for the current repo, erroring if `init` hasn't been run.
fn open_store() -> Result<Store> {
    let cwd = std::env::current_dir()?;
    Store::discover(&cwd)
        .with_context(|| "no .trackly store found — run `trackly init` first".to_string())
}

fn cmd_init() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = Store::at(&cwd);
    if store.exists() && store.load_plan()?.is_some() {
        println!("{} .trackly already initialised here.", ui::dim("·"));
        return Ok(());
    }
    store.init()?;

    let repo_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    match find_seed_doc(&cwd) {
        Some(doc) => {
            let text = std::fs::read_to_string(&doc)
                .with_context(|| format!("reading {}", doc.display()))?;
            let mut plan = parse::parse_markdown(&text, &repo_name);
            plan.source = Some(doc.strip_prefix(&cwd).unwrap_or(&doc).display().to_string());
            store.save_plan(&plan)?;
            println!(
                "{} initialised .trackly and seeded {} task(s) from {}",
                ui::green("✓"),
                plan.tasks.len(),
                ui::bold(&plan.source.clone().unwrap_or_default())
            );
            println!(
                "  run {} to see the scoreboard.",
                ui::bold("trackly status")
            );
        }
        None => {
            let plan = Plan::new(repo_name);
            store.save_plan(&plan)?;
            println!("{} initialised .trackly.", ui::green("✓"));
            println!(
                "  {} no plan doc found (looked for plan.md, tasks.md, goals.md, …).",
                ui::yellow("!")
            );
            println!(
                "  add tasks with {} — or point the agent at {}.",
                ui::bold("trackly plan add \"…\""),
                ui::bold("trackly plan set <file>")
            );
        }
    }
    Ok(())
}

/// A permissive JSON shape agents can emit: `{ "title": ..., "tasks": [ ... ] }`
/// or a bare array of task objects.
#[derive(Deserialize)]
#[serde(untagged)]
enum JsonInput {
    Full {
        title: Option<String>,
        tasks: Vec<JsonTask>,
    },
    Bare(Vec<JsonTask>),
}

#[derive(Deserialize)]
struct JsonTask {
    title: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    status: Option<Status>,
    #[serde(default)]
    weight: Option<u32>,
}

fn cmd_plan_set(file: &str, title: Option<String>) -> Result<()> {
    let store = open_store_or_init()?;
    let repo_name = repo_name(&store);

    let (content, from_stdin) = if file == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        (buf, true)
    } else {
        (
            std::fs::read_to_string(file).with_context(|| format!("reading {file}"))?,
            false,
        )
    };

    // Prefer JSON when the content parses as our input shape; else treat as markdown.
    let mut plan = match serde_json::from_str::<JsonInput>(content.trim()) {
        Ok(input) => plan_from_json(input, &repo_name),
        Err(_) => parse::parse_markdown(&content, &repo_name),
    };
    if let Some(t) = title {
        plan.title = t;
    }
    plan.source = Some(if from_stdin {
        "stdin".to_string()
    } else {
        file.to_string()
    });
    plan.updated_at = Utc::now();

    store.save_plan(&plan)?;
    println!(
        "{} plan set: {} task(s) from {}",
        ui::green("✓"),
        plan.tasks.len(),
        ui::bold(plan.source.as_deref().unwrap_or("?"))
    );
    Ok(())
}

fn plan_from_json(input: JsonInput, fallback_title: &str) -> Plan {
    let (title, tasks) = match input {
        JsonInput::Full { title, tasks } => (title, tasks),
        JsonInput::Bare(tasks) => (None, tasks),
    };
    let mut plan = Plan::new(title.unwrap_or_else(|| fallback_title.to_string()));
    for (i, jt) in tasks.into_iter().enumerate() {
        let mut task = Task::new(format!("t{}", i + 1), jt.title);
        task.group = jt.group;
        task.status = jt.status.unwrap_or(Status::Open);
        task.weight = jt.weight.unwrap_or(1);
        plan.tasks.push(task);
    }
    plan
}

fn cmd_plan_add(title: String, group: Option<String>, weight: u32) -> Result<()> {
    let store = open_store_or_init()?;
    let mut plan = load_plan(&store)?;
    let id = plan.next_id();
    let mut task = Task::new(id.clone(), title);
    task.group = group;
    task.weight = weight;
    plan.tasks.push(task);
    plan.updated_at = Utc::now();
    store.save_plan(&plan)?;
    println!("{} added {}", ui::green("✓"), ui::bold(&id));
    Ok(())
}

fn cmd_task(status: Status, args: &TaskArgs) -> Result<()> {
    let store = open_store()?;
    let mut plan = load_plan(&store)?;
    let now = Utc::now();
    {
        let Some(task) = plan.task_mut(&args.id) else {
            bail!("no task with id `{}`", args.id);
        };
        task.status = status;
        task.updated_at = now;
        if let Some(note) = &args.evidence {
            task.evidence.push(Evidence::new(note.clone()));
        }
        println!(
            "{} {} → {} {}",
            ui::green("✓"),
            ui::bold(&args.id),
            status.glyph(),
            ui::status_colored(status, status.label())
        );
    }
    plan.updated_at = now;
    store.save_plan(&plan)?;
    Ok(())
}

fn cmd_status() -> Result<()> {
    let store = open_store()?;
    let plan = load_plan(&store)?;
    let repo = RepoInfo::gather(store.repo_root());
    ui::print_status(&plan, &repo);
    Ok(())
}

fn cmd_report(out: PathBuf, subtitle: Option<String>) -> Result<()> {
    let store = open_store()?;
    let plan = load_plan(&store)?;
    let meta = ReportMeta {
        subtitle,
        repo: RepoInfo::gather(store.repo_root()),
    };
    let html = render_html(&plan, &meta);
    std::fs::write(&out, html).with_context(|| format!("writing {}", out.display()))?;
    println!(
        "{} wrote {} — open it and Print → Save as PDF.",
        ui::green("✓"),
        ui::bold(&out.display().to_string())
    );
    Ok(())
}

/// Read a commit and apply its task references. Quiet when there's nothing to do,
/// so it's safe to wire into a git hook that runs on every commit.
fn cmd_observe(commit_ref: &str) -> Result<()> {
    // No store or no plan yet → nothing to observe; succeed silently (hook-friendly).
    let Some(store) = Store::discover(&std::env::current_dir()?) else {
        return Ok(());
    };
    let Some(mut plan) = store.load_plan()? else {
        return Ok(());
    };
    let Some(commit) = CommitInfo::lookup(store.repo_root(), commit_ref) else {
        return Ok(());
    };

    let changes = observe::apply_commit(&mut plan, &commit);
    if changes.is_empty() {
        return Ok(());
    }
    plan.updated_at = Utc::now();
    store.save_plan(&plan)?;
    println!(
        "{} trackly: {} from {}",
        ui::green("✓"),
        changes.join(", "),
        ui::dim(&commit.short)
    );
    Ok(())
}

const HOOK_MARKER: &str = "trackly observe";
const HOOK_BODY: &str = "#!/bin/sh\n\
# Trackly post-commit hook — records each commit as evidence against the tasks\n\
# it references. Managed by `trackly hook install`; safe to delete.\n\
command -v trackly >/dev/null 2>&1 || exit 0\n\
trackly observe || true\n\
exit 0\n";

fn cmd_hook_install() -> Result<()> {
    let store = open_store_or_init()?;
    let hooks = git::hooks_dir(store.repo_root())
        .with_context(|| "not a git repository — `git init` first".to_string())?;
    std::fs::create_dir_all(&hooks).with_context(|| format!("creating {}", hooks.display()))?;
    let hook_path = hooks.join("post-commit");

    if hook_path.exists() {
        let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if existing.contains(HOOK_MARKER) {
            println!("{} post-commit hook already installed.", ui::dim("·"));
            return Ok(());
        }
        // Don't clobber a hook we didn't write — tell the user how to add ours.
        println!(
            "{} a post-commit hook already exists at {}.",
            ui::yellow("!"),
            hook_path.display()
        );
        println!("  add this line to it to enable Trackly:");
        println!("    {}", ui::bold("trackly observe || true"));
        return Ok(());
    }

    std::fs::write(&hook_path, HOOK_BODY)
        .with_context(|| format!("writing {}", hook_path.display()))?;
    make_executable(&hook_path)?;
    println!(
        "{} installed post-commit hook at {}",
        ui::green("✓"),
        ui::bold(&hook_path.display().to_string())
    );
    println!(
        "  commits that mention a task id (e.g. {}) now auto-record evidence.",
        ui::bold("\"closes t3\"")
    );
    Ok(())
}

fn cmd_hook_uninstall() -> Result<()> {
    let store = open_store()?;
    let hooks =
        git::hooks_dir(store.repo_root()).with_context(|| "not a git repository".to_string())?;
    let hook_path = hooks.join("post-commit");
    if !hook_path.exists() {
        println!("{} no post-commit hook to remove.", ui::dim("·"));
        return Ok(());
    }
    let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
    if !existing.contains(HOOK_MARKER) {
        bail!(
            "the post-commit hook at {} wasn't installed by Trackly — leaving it untouched",
            hook_path.display()
        );
    }
    std::fs::remove_file(&hook_path)
        .with_context(|| format!("removing {}", hook_path.display()))?;
    println!("{} removed Trackly's post-commit hook.", ui::green("✓"));
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

// ---- helpers ----

fn load_plan(store: &Store) -> Result<Plan> {
    store
        .load_plan()?
        .with_context(|| "no plan yet — run `trackly init` or `trackly plan set`".to_string())
}

/// Like [`open_store`] but transparently initialises if needed (used by plan commands).
fn open_store_or_init() -> Result<Store> {
    let cwd = std::env::current_dir()?;
    if let Some(store) = Store::discover(&cwd) {
        return Ok(store);
    }
    let store = Store::at(&cwd);
    store.init()?;
    Ok(store)
}

fn repo_name(store: &Store) -> String {
    store
        .repo_root()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}
