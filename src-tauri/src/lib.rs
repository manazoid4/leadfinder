use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lead {
    id: i64,
    business_name: String,
    trade: String,
    area: String,
    phone: String,
    website: Option<String>,
    gap_reason: String,
    confidence: String,
    eligible: bool,
    opener: String,
    outcome: Option<String>,
    verification_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    fast_model: String,
    smart_model: String,
    ollama_ready: bool,
    installed_models: Vec<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTags {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

fn connection(app: &AppHandle) -> Result<Connection, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let db = Connection::open(dir.join("leadfinder.sqlite3")).map_err(|error| error.to_string())?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS leads (
          id INTEGER PRIMARY KEY,
          business_name TEXT NOT NULL,
          trade TEXT NOT NULL,
          area TEXT NOT NULL,
          phone TEXT NOT NULL,
          website TEXT,
          gap_reason TEXT NOT NULL,
          confidence TEXT NOT NULL,
          eligible INTEGER NOT NULL,
          opener TEXT NOT NULL,
          outcome TEXT,
          verification_count INTEGER NOT NULL DEFAULT 1
        );",
    )
    .map_err(|error| error.to_string())?;
    let _ = db.execute("ALTER TABLE leads ADD COLUMN verification_count INTEGER NOT NULL DEFAULT 1", []);
    db.execute(
        "INSERT OR IGNORE INTO leads (id,business_name,trade,area,phone,website,gap_reason,confidence,eligible,opener)
         VALUES (1,'Derby Roofing & Co','Roofing','Derby','01332 555 014',NULL,'NO_WEBSITE','certain',1,
         'My name\'s Maz from Maz Works. I was looking your business up earlier and couldn\'t find a proper website for you. Are most new enquiries coming through phone, social media or referrals at the moment?')",
        [],
    )
    .map_err(|error| error.to_string())?;
    Ok(db)
}

#[tauri::command]
fn list_leads(app: AppHandle) -> Result<Vec<Lead>, String> {
    let db = connection(&app)?;
    let mut statement = db
        .prepare("SELECT id,business_name,trade,area,phone,website,gap_reason,confidence,eligible,opener,outcome,verification_count FROM leads ORDER BY id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Lead {
                id: row.get(0)?,
                business_name: row.get(1)?,
                trade: row.get(2)?,
                area: row.get(3)?,
                phone: row.get(4)?,
                website: row.get(5)?,
                gap_reason: row.get(6)?,
                confidence: row.get(7)?,
                eligible: row.get::<_, i64>(8)? == 1,
                opener: row.get(9)?,
                outcome: row.get(10)?,
                verification_count: row.get(11)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string())).collect()
}

#[tauri::command]
fn model_status() -> Result<ModelStatus, String> {
    const FAST: &str = "phi4-mini:latest";
    const SMART: &str = "lfm2.5-8b:latest";
    let response = reqwest::blocking::get("http://127.0.0.1:11434/api/tags");
    match response {
        Ok(response) => {
            let tags: OllamaTags = response.json().map_err(|error| error.to_string())?;
            let installed_models = tags.models.into_iter().map(|model| model.name).collect::<Vec<_>>();
            let fast_ready = installed_models.iter().any(|model| model == FAST);
            let smart_ready = installed_models.iter().any(|model| model == SMART);
            Ok(ModelStatus {
                fast_model: FAST.to_string(),
                smart_model: SMART.to_string(),
                ollama_ready: fast_ready && smart_ready,
                installed_models,
                message: if fast_ready && smart_ready { "Maz Fast + Maz Smart ready".to_string() } else { "One or more configured local models are missing".to_string() },
            })
        }
        Err(error) => Ok(ModelStatus {
            fast_model: FAST.to_string(),
            smart_model: SMART.to_string(),
            ollama_ready: false,
            installed_models: Vec::new(),
            message: format!("Ollama unavailable: {error}"),
        }),
    }
}

#[tauri::command]
fn plan_search(trade: String, area: String) -> Result<Vec<String>, String> {
    let prompt = format!("Create exactly 5 concise public search queries for finding {trade} businesses in {area}. Return JSON only as {{\"queries\":[\"...\"]}}. Do not invent business names or facts.");
    let body = serde_json::json!({
        "model": "phi4-mini:latest",
        "messages": [{"role": "system", "content": "You are Maz Fast, a query planner. Never claim a business exists."}, {"role": "user", "content": prompt}],
        "format": "json", "stream": false, "keep_alive": "10m", "options": {"temperature": 0.1, "num_predict": 180}
    });
    let response = reqwest::blocking::Client::new().post("http://127.0.0.1:11434/api/chat").json(&body).send().map_err(|error| error.to_string())?;
    if !response.status().is_success() { return Err(format!("Ollama returned {}", response.status())); }
    let result: OllamaChatResponse = response.json().map_err(|error| error.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&result.message.content).map_err(|error| format!("Maz Fast returned invalid JSON: {error}"))?;
    parsed.get("queries").and_then(|value| value.as_array()).map(|queries| queries.iter().filter_map(|value| value.as_str().map(str::to_owned)).take(5).collect()).filter(|queries: &Vec<String>| !queries.is_empty()).ok_or_else(|| "Maz Fast returned no queries".to_string())
}

#[tauri::command]
fn import_csv(app: AppHandle, contents: String) -> Result<usize, String> {
    let db = connection(&app)?;
    let mut imported = 0usize;
    for (index, line) in contents.lines().enumerate() {
        if index == 0 && line.to_ascii_lowercase().contains("business") { continue; }
        let columns = line.split(',').map(|value| value.trim().trim_matches('"')).collect::<Vec<_>>();
        if columns.len() < 4 || columns[0].is_empty() || columns[3].is_empty() { continue; }
        let next_id: i64 = db.query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM leads", [], |row| row.get(0)).map_err(|error| error.to_string())?;
        let website = columns.get(4).filter(|value| !value.is_empty()).copied();
        db.execute("INSERT INTO leads (id,business_name,trade,area,phone,website,gap_reason,confidence,eligible,opener,outcome,verification_count) VALUES (?1,?2,?3,?4,?5,?6,'UNCERTAIN','uncertain',0,'My name\'s Maz from Maz Works. I was looking at your business online earlier and wanted to check something — how do customers normally arrange a quote or visit with you?',NULL,1)", params![next_id, columns[0], columns[1], columns[2], columns[3], website]).map_err(|error| error.to_string())?;
        imported += 1;
    }
    Ok(imported)
}

#[tauri::command]
fn discover_leads(app: AppHandle, queries: Vec<String>) -> Result<usize, String> {
    let resource_dir = app.path().resource_dir().map_err(|error| error.to_string())?;
    let executable = resource_dir.join("gosom.exe");
    if !executable.exists() { return Err("Gosom sidecar is not installed".to_string()); }
    let dir = app.path().app_data_dir().map_err(|error| error.to_string())?.join("discovery");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let input = dir.join("queries.txt"); let output = dir.join("results.csv");
    fs::write(&input, format!("{}\n", queries.join("\n"))).map_err(|error| error.to_string())?;
    let result = Command::new(executable).arg("-input").arg(&input).arg("-results").arg(&output).arg("-fast-mode").arg("-exit-on-inactivity").arg("30s").output().map_err(|error| error.to_string())?;
    if !result.status.success() { return Err(String::from_utf8_lossy(&result.stderr).trim().to_string()); }
    if !output.exists() { return Err("Gosom completed without a CSV result".to_string()); }
    import_csv(app, fs::read_to_string(output).map_err(|error| error.to_string())?)
}

#[tauri::command]
fn smart_review(business_name: String, website: Option<String>, evidence: String) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "lfm2.5-8b:latest",
        "messages": [
            {"role": "system", "content": "You are Maz Smart. Review only the supplied evidence. Never invent facts, never change eligibility, and return at most 60 words."},
            {"role": "user", "content": format!("Business: {business_name}\nWebsite: {}\nEvidence: {evidence}\nReturn a concise advisory review and list any uncertainty.", website.unwrap_or_else(|| "not found".to_string()))}
        ],
        "stream": false, "keep_alive": "10m", "options": {"temperature": 0.1, "num_predict": 120}
    });
    let response = reqwest::blocking::Client::new().post("http://127.0.0.1:11434/api/chat").json(&body).send().map_err(|error| error.to_string())?;
    if !response.status().is_success() { return Err(format!("Ollama returned {}", response.status())); }
    let result: OllamaChatResponse = response.json().map_err(|error| error.to_string())?;
    Ok(result.message.content.chars().take(600).collect())
}

#[tauri::command]
fn save_outcome(app: AppHandle, id: i64, outcome: String) -> Result<(), String> {
    let db = connection(&app)?;
    db.execute("UPDATE leads SET outcome = ?1 WHERE id = ?2", params![outcome, id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![list_leads, save_outcome, model_status, plan_search, import_csv, discover_leads, smart_review])
        .run(tauri::generate_context!())
        .expect("error while running LeadFinder");
}
