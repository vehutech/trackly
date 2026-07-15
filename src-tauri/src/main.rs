// src-tauri/src/main.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tauri::{Manager, State, Emitter};
use sqlx::{SqlitePool, Row};

// ============= DATA STRUCTURES =============

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub priority: String,
    #[serde(rename = "logoPath")]
    pub logo_path: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "completionPercentage")]
    pub completion_percentage: f64,
    #[serde(rename = "timeSpent")]
    pub time_spent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkSession {
    pub id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "projectName")]
    pub project_name: String,
    pub goal: String,
    #[serde(rename = "workDone")]
    pub work_done: String, // JSON array as string
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: i64,
    #[serde(rename = "isSynced")]
    pub is_synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    pub priority: String,
    #[serde(rename = "completionPercentage")]
    pub completion_percentage: f64,
    #[serde(rename = "timeSpent")]
    pub time_spent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    #[serde(rename = "totalTimeMinutes")]
    pub total_time_minutes: i64,
    #[serde(rename = "goalsSet")]
    pub goals_set: i64,
    #[serde(rename = "goalsAchieved")]
    pub goals_achieved: i64,
    #[serde(rename = "completionRate")]
    pub completion_rate: f64,
    #[serde(rename = "productivityScore")]
    pub productivity_score: f64,
}

struct AppState {
    db: SqlitePool,
    current_session: Mutex<Option<WorkSession>>,
    last_activity: Mutex<SystemTime>,
}

// ============= DATABASE FUNCTIONS =============

async fn init_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            priority TEXT NOT NULL,
            logo_path TEXT,
            created_at TEXT NOT NULL,
            completion_percentage REAL DEFAULT 0.0,
            time_spent REAL DEFAULT 0.0
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS work_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            project_name TEXT NOT NULL,
            goal TEXT NOT NULL,
            work_done TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT,
            duration_minutes INTEGER,
            is_synced INTEGER DEFAULT 0,
            FOREIGN KEY (project_id) REFERENCES projects(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============= TAURI COMMANDS =============

#[tauri::command]
async fn get_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT id, name, priority, logo_path, created_at, completion_percentage, time_spent FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
async fn create_project(
    project: CreateProjectInput,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO projects (id, name, priority, created_at, completion_percentage, time_spent)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&project.name)
    .bind(&project.priority)
    .bind(&created_at)
    .bind(project.completion_percentage)
    .bind(project.time_spent)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Project {
        id,
        name: project.name,
        priority: project.priority,
        logo_path: None,
        created_at,
        completion_percentage: project.completion_percentage,
        time_spent: project.time_spent,
    })
}

#[tauri::command]
async fn update_project(
    id: String,
    name: Option<String>,
    completion_percentage: Option<f64>,
    time_spent: Option<f64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(n) = name {
        sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
            .bind(n)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(cp) = completion_percentage {
        sqlx::query("UPDATE projects SET completion_percentage = ? WHERE id = ?")
            .bind(cp)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(ts) = time_spent {
        sqlx::query("UPDATE projects SET time_spent = ? WHERE id = ?")
            .bind(ts)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn delete_project(id: String, state: State<'_, AppState>) -> Result<(), String> {
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn start_session(
    project_id: String,
    goal: String,
    state: State<'_, AppState>,
) -> Result<WorkSession, String> {
    let project = sqlx::query_as::<_, Project>(
        "SELECT id, name, priority, logo_path, created_at, completion_percentage, time_spent FROM projects WHERE id = ?"
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let session = WorkSession {
        id: uuid::Uuid::new_v4().to_string(),
        project_id,
        project_name: project.name,
        goal,
        work_done: "[]".to_string(),
        start_time: chrono::Utc::now().to_rfc3339(),
        end_time: None,
        duration_minutes: 0,
        is_synced: false,
    };

    *state.current_session.lock().unwrap() = Some(session.clone());

    Ok(session)
}

#[tauri::command]
async fn update_session(
    session_id: String,
    work_done: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut current = state.current_session.lock().unwrap();
    
    if let Some(session) = current.as_mut() {
        if session.id == session_id {
            let mut work_list: Vec<String> = serde_json::from_str(&session.work_done)
                .unwrap_or_default();
            work_list.push(work_done);
            session.work_done = serde_json::to_string(&work_list)
                .unwrap_or_else(|_| "[]".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
async fn end_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<WorkSession, String> {
    let mut current = state.current_session.lock().unwrap();
    
    if let Some(mut session) = current.take() {
        if session.id == session_id {
            let end_time = chrono::Utc::now();
            let start = chrono::DateTime::parse_from_rfc3339(&session.start_time)
                .map_err(|e| e.to_string())?;
            
            session.end_time = Some(end_time.to_rfc3339());
            session.duration_minutes = (end_time.timestamp() - start.timestamp()) / 60;

            sqlx::query(
                r#"
                INSERT INTO work_sessions 
                (id, project_id, project_name, goal, work_done, start_time, end_time, duration_minutes, is_synced)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&session.id)
            .bind(&session.project_id)
            .bind(&session.project_name)
            .bind(&session.goal)
            .bind(&session.work_done)
            .bind(&session.start_time)
            .bind(&session.end_time)
            .bind(session.duration_minutes)
            .bind(if session.is_synced { 1 } else { 0 })
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;

            // Update project completion and time
            update_project_stats(&session.project_id, &state.db).await?;

            return Ok(session);
        }
    }

    Err("No active session found".to_string())
}

async fn update_project_stats(project_id: &str, db: &SqlitePool) -> Result<(), String> {
    let sessions: Vec<WorkSession> = sqlx::query_as(
        "SELECT * FROM work_sessions WHERE project_id = ?"
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    if sessions.is_empty() {
        return Ok(());
    }

    let total_time: f64 = sessions.iter()
        .map(|s| s.duration_minutes as f64 / 60.0)
        .sum();

    let completed = sessions.iter()
        .filter(|s| {
            let work_list: Vec<String> = serde_json::from_str(&s.work_done)
                .unwrap_or_default();
            !work_list.is_empty()
        })
        .count();

    let completion = (completed as f64 / sessions.len() as f64) * 100.0;

    sqlx::query(
        "UPDATE projects SET completion_percentage = ?, time_spent = ? WHERE id = ?"
    )
    .bind(completion)
    .bind(total_time)
    .bind(project_id)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_sessions(
    project_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<WorkSession>, String> {
    let sessions = if let Some(pid) = project_id {
        sqlx::query_as::<_, WorkSession>(
            "SELECT * FROM work_sessions WHERE project_id = ? ORDER BY start_time DESC"
        )
        .bind(pid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as::<_, WorkSession>(
            "SELECT * FROM work_sessions ORDER BY start_time DESC"
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(sessions)
}

#[tauri::command]
async fn get_analytics(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Analytics, String> {
    let sessions = get_sessions(Some(project_id), state).await?;

    let total_time: i64 = sessions.iter().map(|s| s.duration_minutes).sum();
    let goals_set = sessions.len() as i64;
    let goals_achieved = sessions.iter()
        .filter(|s| {
            let work_list: Vec<String> = serde_json::from_str(&s.work_done)
                .unwrap_or_default();
            !work_list.is_empty()
        })
        .count() as i64;

    let completion_rate = if goals_set > 0 {
        (goals_achieved as f64 / goals_set as f64) * 100.0
    } else {
        0.0
    };

    let time_score = ((total_time as f64 / 60.0) * 10.0).min(100.0);
    let productivity_score = (time_score * 0.4 + completion_rate * 0.6).min(100.0);

    Ok(Analytics {
        total_time_minutes: total_time,
        goals_set,
        goals_achieved,
        completion_rate,
        productivity_score,
    })
}

#[tauri::command]
async fn sync_to_cloud(state: State<'_, AppState>) -> Result<(), String> {
    // Placeholder for cloud sync logic
    println!("Syncing to cloud...");
    Ok(())
}

#[tauri::command]
async fn check_online() -> Result<bool, String> {
    // Simple online check
    Ok(true)
}

// ============= BACKGROUND MONITORING =============

async fn start_activity_monitor(window: Window) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;
            
            // Emit event to frontend to prompt user
            let _ = window.emit("prompt_user", serde_json::json!({
                "type": "start_session"
            }));
        }
    });
}

// ============= MAIN =============

#[tokio::main]
async fn main() {
    let db_path = "trackly.db";
    let pool = SqlitePool::connect(&format!("sqlite:{}", db_path))
        .await
        .expect("Failed to connect to database");

    init_database(&pool).await.expect("Failed to initialize database");

    let app_state = AppState {
        db: pool,
        current_session: Mutex::new(None),
        last_activity: Mutex::new(SystemTime::now()),
    };

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();  // Changed from get_window
            tauri::async_runtime::spawn(start_activity_monitor(window));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_projects,
            create_project,
            update_project,
            delete_project,
            start_session,
            update_session,
            end_session,
            get_sessions,
            get_analytics,
            sync_to_cloud,
            check_online,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}