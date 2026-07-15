//! `trackly-mcp` — a Model Context Protocol server for Trackly.
//!
//! Speaks JSON-RPC 2.0 over stdio (newline-delimited messages), the MCP stdio
//! transport. An agent connects and calls tools to push its plan and mark progress;
//! every tool is a thin call into `trackly-core`. Diagnostics go to stderr so stdout
//! stays a clean protocol channel.

mod tools;

use std::io::{BufRead, Write};

use serde_json::{json, Value};

/// MCP protocol version we implement (echoed back if the client doesn't specify one).
const PROTOCOL_VERSION: &str = "2025-06-18";

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    eprintln!("trackly-mcp {} — ready on stdio", env!("CARGO_PKG_VERSION"));

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("trackly-mcp: stdin read error: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                send(
                    &mut out,
                    error_response(&Value::Null, -32700, &format!("parse error: {e}")),
                );
                continue;
            }
        };
        if let Some(resp) = handle(&req) {
            send(&mut out, resp);
        }
    }
}

/// Dispatch one request. Returns `Some(response)` for requests (those with an `id`),
/// or `None` for notifications (which get no reply).
fn handle(req: &Value) -> Option<Value> {
    let method = req
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    // Notifications (no `id`) are processed but never answered.
    let id = req.get("id").cloned()?;

    let result = match method {
        "initialize" => Ok(initialize_result(req)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_schemas() })),
        "tools/call" => return Some(handle_tool_call(&id, req)),
        other => Err((-32601, format!("method not found: {other}"))),
    };

    Some(match result {
        Ok(value) => success_response(&id, value),
        Err((code, msg)) => error_response(&id, code, &msg),
    })
}

fn initialize_result(req: &Value) -> Value {
    // Mirror the client's requested protocol version when present.
    let version = req
        .get("params")
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "trackly", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Track this repo's progress. Call set_plan when you form a plan, \
    update_task as you finish each task, and generate_report to produce a shareable report."
    })
}

/// Run a `tools/call` and wrap the outcome. Tool failures are returned as an
/// `isError` result (per MCP), not a protocol-level error.
fn handle_tool_call(id: &Value, req: &Value) -> Value {
    let params = req.get("params").cloned().unwrap_or(json!({}));
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let empty = json!({});
    let args = params.get("arguments").unwrap_or(&empty);

    let outcome = match name {
        "set_plan" => tools::set_plan(args),
        "add_task" => tools::add_task(args),
        "update_task" => tools::update_task(args),
        "get_status" => tools::get_status(args),
        "generate_report" => tools::generate_report(args),
        other => Err(format!("unknown tool: {other}")),
    };

    match outcome {
        Ok(text) => success_response(id, json!({ "content": [text_content(&text)] })),
        Err(msg) => success_response(
            id,
            json!({ "content": [text_content(&msg)], "isError": true }),
        ),
    }
}

fn text_content(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn success_response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Write one JSON-RPC message as a single line, then flush.
fn send(out: &mut impl Write, msg: Value) {
    if let Ok(s) = serde_json::to_string(&msg) {
        let _ = writeln!(out, "{s}");
        let _ = out.flush();
    }
}

/// JSON-Schema definitions for every exposed tool.
fn tool_schemas() -> Value {
    json!([
        {
            "name": "set_plan",
            "description": "Record the plan for this repo, replacing any existing plan. Call this once you've formed a plan, before starting work.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Project title for the report." },
                    "tasks": {
                        "type": "array",
                        "description": "The plan's tasks, in order.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string", "description": "What the task is." },
                                "group": { "type": "string", "description": "Optional phase/section this task belongs to." },
                                "status": { "type": "string", "enum": ["open", "in_progress", "partial", "done", "blocked"], "description": "Initial status (default open)." },
                                "weight": { "type": "integer", "minimum": 1, "description": "Relative size for scoring (default 1)." }
                            },
                            "required": ["title"]
                        }
                    }
                },
                "required": ["tasks"]
            }
        },
        {
            "name": "add_task",
            "description": "Append a single task to the plan (creates the plan if none exists yet).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "group": { "type": "string" },
                    "weight": { "type": "integer", "minimum": 1 }
                },
                "required": ["title"]
            }
        },
        {
            "name": "update_task",
            "description": "Set a task's status as you work. Attach a commit hash or note as evidence when you finish something.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Task id, e.g. t3." },
                    "status": { "type": "string", "enum": ["open", "in_progress", "partial", "done", "blocked"] },
                    "evidence": { "type": "string", "description": "Optional commit hash / file / note." }
                },
                "required": ["id", "status"]
            }
        },
        {
            "name": "get_status",
            "description": "Return the current progress scoreboard as text (overall %, counts, and tasks by group).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "generate_report",
            "description": "Write a print-ready HTML progress report and return its path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subtitle": { "type": "string", "description": "Optional org/project line for the header." },
                    "out": { "type": "string", "description": "Output path (default trackly-report.html in the repo)." }
                }
            }
        }
    ])
}
