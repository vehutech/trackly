# Trackly

**Agent-native progress tracking that lives in a repo like git.**

Trackly rides alongside a coding agent (Claude Code, Cursor, Aider, …). The moment
the agent forms a plan, it hands that plan to Trackly; as the agent works, it marks
tasks done. Trackly measures weighted completion and turns it into a terminal
scoreboard and a printable progress report.

No nagging, no daemon. Trackly is silent until you (or the agent) call it — exactly
like git is silent until you commit.

```
agent forms plan  ──▶  Trackly records it        (source of truth)
agent does a task ──▶  Trackly marks it done     (execution state)
you run report    ──▶  Trackly renders a PDF     (the deliverable)
```

---

## Why

Most repos already contain a plan — `plan.md`, `tasks.md`, `goals.md`, an agent's
todo list. What's missing is a way to **measure** how far along it is and **show**
that to someone. Trackly reads the plan, tracks execution against it, and produces a
clean report you can hand to a stakeholder.

Because it snapshots state every time it's touched, Trackly also builds a history of
*when* things got done — reconstructing the progress timeline from the work itself.

## Install

Trackly is a single native binary. **End users never need Rust installed** — that's
only for building from source today. (Prebuilt binaries and `brew`/`npx` install are
on the roadmap.)

```sh
# From source (requires the Rust toolchain)
git clone https://github.com/vehutech/trackly
cd trackly
cargo build --release -p trackly-cli
# binary at target/release/trackly — put it on your PATH
```

## Quickstart

```sh
cd your-repo
trackly init          # creates .trackly/, seeding a plan from plan.md/tasks.md/… if present
trackly status        # see the scoreboard
trackly report        # write trackly-report.html → open it, Print → Save as PDF
```

If no plan doc is found, `init` tells you how to add one.

## How an agent uses it

An agent that can run shell commands can drive Trackly directly. Two patterns:

**Push the whole plan at once** (markdown checklist or JSON, from a file or stdin):

```sh
# from a markdown checklist
trackly plan set plan.md

# from JSON, piped in
echo '{
  "title": "Payments Service",
  "tasks": [
    {"title": "Card charge endpoint", "group": "Phase 2", "status": "done"},
    {"title": "Refund endpoint",      "group": "Phase 2", "status": "partial"},
    {"title": "Reconciliation report","group": "Phase 3", "weight": 3}
  ]
}' | trackly plan set -
```

**Update tasks as work lands:**

```sh
trackly task start t4
trackly task done  t4 --evidence "abc1234 implemented charge endpoint"
trackly task partial t5
trackly task block   t7 --evidence "waiting on vendor API keys"
```

## Command reference

| Command | What it does |
|---|---|
| `trackly init` | Create `.trackly/`, seeding a plan from an existing doc if one is found. |
| `trackly plan set <file\|->` | Replace the plan from a markdown checklist or JSON (`-` = stdin). |
| `trackly plan add "<title>" [--group G] [--weight W]` | Append one task. |
| `trackly task done\|partial\|start\|block\|open <id> [--evidence <note>]` | Change a task's status. |
| `trackly status` | Print the terminal scoreboard (%, done/partial/open, by group). |
| `trackly report [--out FILE] [--subtitle "Org"]` | Write a print-ready HTML report. |

## Plan format

Trackly reads ordinary markdown checklists — nothing new to learn:

```markdown
# Payments Service

## Phase 1 — Foundations
- [x] Scaffold the service      ← done
- [/] Set up CI                 ← in progress
- [ ] Write the docs            ← open

## Phase 2 — Core flows
- [~] Webhook receiver          ← partial (half credit)
- [-] Fraud check               ← blocked
```

`#` → report title · `##`/`###` → groups · list items → tasks. Checkbox marks:
`[x]` done, `[ ]` open, `[/]` in progress, `[~]` partial, `[-]` blocked.

## Scoring

Completion is **line-weighted with half-credit partials**:

```
percent = Σ(credit(task) × weight) / Σ(weight) × 100
credit:  done = 1.0   partial = 0.5   open/in-progress/blocked = 0
```

Every task defaults to weight 1; bump `--weight` for larger items. Treat the % as a
rough directional signal, not a precise measure of effort.

## What's stored

Trackly keeps a `.trackly/` directory in your repo, mirroring how `.git/` sits there:

- `.trackly/plan.json` — the current plan (source of truth)
- `.trackly/history.jsonl` — an append-only snapshot per change (the trend over time)

Both are human-readable. Commit them if you want the plan and its history versioned.

## Architecture

A Rust workspace so one engine backs every surface:

- **`crates/trackly-core`** — the plan model, store, scoring, markdown parsing, and
  HTML report renderer. UI-agnostic.
- **`crates/trackly-cli`** — the `trackly` command (this is v0.1).
- **`src-tauri` + `src`** — the future desktop app (see roadmap).

## Roadmap

- **v0.1 (now)** — the `trackly` CLI: plan capture, status, HTML/PDF report.
- **Next — MCP server** — a thin wrapper over `trackly-core` so agents call
  `set_plan` / `update_task` natively instead of shelling out.
- **Git as evidence** — an opt-in post-commit hook that auto-snapshots and links
  commit hashes/dates to tasks.
- **The "like GitHub" desktop app** — the Tauri app becomes a machine-wide view that
  discovers all your Trackly repos, shows dashboards, and exports reports.

## License

MIT.
