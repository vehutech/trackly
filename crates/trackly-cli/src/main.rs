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

use trackly_core::model::{Evidence, Plan, Status, Task};
use trackly_core::report::{render_html, ReportMeta};
use trackly_core::{find_seed_doc, git::RepoInfo, parse, Store};

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
