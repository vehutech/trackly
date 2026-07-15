//! Terminal presentation: ANSI helpers and the `status` scoreboard.

use chrono::Utc;
use trackly_core::git::RepoInfo;
use trackly_core::model::{Plan, Status};
use trackly_core::score::Stats;

// ---- ANSI helpers ----
// Kept dependency-free; disabled automatically when stdout isn't a terminal.

fn color(code: &str, s: &str) -> String {
    if colors_on() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn colors_on() -> bool {
    // Respect NO_COLOR and only colorize when writing to a tty.
    std::env::var_os("NO_COLOR").is_none() && is_terminal()
}

fn is_terminal() -> bool {
    // Avoid an extra dependency: probe via isatty on the stdout fd.
    #[cfg(unix)]
    {
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        unsafe { isatty(1) == 1 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn bold(s: &str) -> String {
    color("1", s)
}
pub fn dim(s: &str) -> String {
    color("2", s)
}
pub fn red(s: &str) -> String {
    color("31", s)
}
pub fn green(s: &str) -> String {
    color("32", s)
}
pub fn yellow(s: &str) -> String {
    color("33", s)
}

/// Colorize text to match a status.
pub fn status_colored(status: Status, s: &str) -> String {
    let code = match status {
        Status::Done => "32",       // green
        Status::Partial => "33",    // yellow
        Status::InProgress => "34", // blue
        Status::Open => "2",        // dim
        Status::Blocked => "31",    // red
    };
    color(code, s)
}

/// Render a compact percentage bar of fixed width.
fn bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let mut s = String::new();
    s.push_str(&"█".repeat(filled));
    s.push_str(&dim(&"░".repeat(width - filled)));
    s
}

/// Print the full `trackly status` scoreboard.
pub fn print_status(plan: &Plan, repo: &RepoInfo) {
    let s = Stats::of(&plan.tasks);
    let today = Utc::now().format("%Y-%m-%d");

    // Header line.
    let branch = repo
        .branch
        .as_deref()
        .map(|b| format!(" · {b}"))
        .unwrap_or_default();
    println!();
    println!(
        "  {}{}",
        bold(&plan.title),
        dim(&format!("{branch} · {today}"))
    );

    // Headline percent + bar.
    println!(
        "  {}  {}  {}",
        bold(&format!("{:.1}%", s.percent)),
        bar(s.percent, 24),
        dim(&format!("{} items", s.total)),
    );

    // Tile counts.
    println!(
        "  {}   {}   {}   {}",
        green(&format!("{} done", s.done)),
        yellow(&format!("{} partial", s.partial)),
        color_blue(&format!("{} in progress", s.in_progress)),
        dim(&format!("{} open", s.open + s.blocked)),
    );

    if plan.tasks.is_empty() {
        println!();
        println!(
            "  {} no tasks yet — {} or {}",
            yellow("!"),
            bold("trackly plan add \"…\""),
            bold("trackly plan set <file>")
        );
        println!();
        return;
    }

    // Per-group detail.
    for group in plan.group_order() {
        let tasks: Vec<_> = plan.tasks.iter().filter(|t| t.group == group).collect();
        if tasks.is_empty() {
            continue;
        }
        let gs = Stats::of(tasks.iter().copied());
        let name = group.as_deref().unwrap_or("Ungrouped");
        println!();
        println!("  {} {}", bold(name), dim(&format!("{:.0}%", gs.percent)));
        for t in tasks {
            println!(
                "    {} {}  {}",
                status_colored(t.status, t.status.glyph()),
                dim(&t.id),
                t.title,
            );
        }
    }
    println!();
}

fn color_blue(s: &str) -> String {
    color("34", s)
}
