use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs;
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
          outcome TEXT
        );",
    )
    .map_err(|error| error.to_string())?;
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
        .prepare("SELECT id,business_name,trade,area,phone,website,gap_reason,confidence,eligible,opener,outcome FROM leads ORDER BY id")
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
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string())).collect()
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
        .invoke_handler(tauri::generate_handler![list_leads, save_outcome])
        .run(tauri::generate_context!())
        .expect("error while running LeadFinder");
}
