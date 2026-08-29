mod cloud;
mod database;
mod ingest;
mod research;

use cloud::{chat_completion, fast_model, mid_model, router_status, CloudBudget, RouterStatus};
use database::{migrate_database, verification_state};
use ingest::{import_raw_leads, parse_csv, ImportSummary, RawLead};
use research::{inspect_site, web_search, SiteSignals};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lead {
    id: i64,
    business_name: String,
    trade: String,
    area: String,
    phone: Option<String>,
    website: Option<String>,
    contact_channel: String,
    status: String,
    next_action: Option<String>,
    opportunity: Option<String>,
    solution: Option<String>,
    template_id: Option<String>,
    demo_url: Option<String>,
    gap_reason: String,
    confidence: String,
    eligible: bool,
    opener: String,
    outcome: Option<String>,
    verification_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    ready: bool,
    schema_version: i64,
    lead_count: i64,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Partner {
    id: i64,
    name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    referred_leads: i64,
    conversions: i64,
    notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeadUpdate {
    status: String,
    next_action: Option<String>,
    opportunity: Option<String>,
    solution: Option<String>,
    template_id: Option<String>,
    demo_url: Option<String>,
}

fn connection(app: &AppHandle) -> Result<Connection, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let mut database =
        Connection::open(dir.join("leadfinder.sqlite3")).map_err(|error| error.to_string())?;
    migrate_database(&mut database)
        .map_err(|error| format!("Database migration failed: {error}"))?;
    Ok(database)
}

#[tauri::command]
fn database_health(app: AppHandle) -> Result<DatabaseHealth, String> {
    let database = connection(&app)?;
    let schema_version = database
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let lead_count = database
        .query_row("SELECT COUNT(*) FROM leads", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    Ok(DatabaseHealth {
        ready: true,
        schema_version,
        lead_count,
        message: "SQLite and migrations ready".to_string(),
    })
}

#[tauri::command]
fn list_leads(app: AppHandle) -> Result<Vec<Lead>, String> {
    let database = connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT
                l.id, l.business_name, l.trade, l.area, l.phone, l.website,
                l.contact_channel, l.status, l.next_action, l.opportunity,
                l.solution, l.template_id, l.demo_url, l.gap_reason,
                l.confidence, l.opener, l.outcome,
                (SELECT COUNT(*) FROM lead_evidence e
                 WHERE e.lead_id = l.id AND e.channel = l.contact_channel
                   AND e.status = 'PASS') AS verification_count
             FROM leads l
             ORDER BY l.id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let verification_count = row.get::<_, i64>(17)?;
            Ok(Lead {
                id: row.get(0)?,
                business_name: row.get(1)?,
                trade: row.get(2)?,
                area: row.get(3)?,
                phone: row.get(4)?,
                website: row.get(5)?,
                contact_channel: row.get(6)?,
                status: row.get(7)?,
                next_action: row.get(8)?,
                opportunity: row.get(9)?,
                solution: row.get(10)?,
                template_id: row.get(11)?,
                demo_url: row.get(12)?,
                gap_reason: row.get(13)?,
                confidence: row.get(14)?,
                opener: row.get(15)?,
                outcome: row.get(16)?,
                eligible: verification_count == 5,
                verification_count,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

#[tauri::command]
fn model_status() -> RouterStatus {
    router_status()
}

fn parse_queries(content: &str) -> Result<Vec<String>, String> {
    let parsed: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| format!("Model returned invalid JSON: {error}"))?;
    let values = parsed
        .get("queries")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "Model response has no queries array".to_string())?;
    let mut seen = HashSet::new();
    let queries = values
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 120)
        .filter(|value| seen.insert(value.to_ascii_lowercase()))
        .take(5)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if queries.len() != 5 {
        return Err(format!(
            "Model response failed schema validation: expected 5 unique queries, got {}",
            queries.len()
        ));
    }
    Ok(queries)
}

fn cached_completion(
    app: &AppHandle,
    budget: &CloudBudget,
    model: &str,
    system: &str,
    input: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let content_hash = format!(
        "{:x}",
        Sha256::digest(format!("{model}\n{system}\n{input}").as_bytes())
    );
    let database = connection(app)?;
    if let Some(response) = database
        .query_row(
            "SELECT response FROM model_cache WHERE content_hash = ?1",
            [&content_hash],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        log::info!("cloud_cache_hit={} model={}", content_hash, model);
        return Ok(response);
    }
    let response = chat_completion(budget, model, system, input, max_tokens)?;
    database
        .execute(
            "INSERT OR REPLACE INTO model_cache (content_hash, model, response) VALUES (?1, ?2, ?3)",
            params![content_hash, model, response],
        )
        .map_err(|error| error.to_string())?;
    Ok(response)
}

#[tauri::command]
fn plan_search(
    app: AppHandle,
    trade: String,
    area: String,
    budget: State<'_, CloudBudget>,
) -> Result<Vec<String>, String> {
    let input = format!("Trade: {trade}\nArea: {area}\nReturn exactly: {{\"queries\":[\"...\"]}}");
    let system = "Create five concise public Google Maps search queries. JSON only. Never invent business names or facts.";
    let first = cached_completion(&app, &budget, &fast_model(), system, &input, 220)?;
    match parse_queries(&first) {
        Ok(queries) => Ok(queries),
        Err(first_error) => {
            let escalation_input = format!(
                "{input}\nThe cheap-tier response failed validation: {first_error}. Return valid JSON only."
            );
            let second = cached_completion(
                &app,
                &budget,
                &mid_model(),
                "Repair one invalid query-plan response. Output exactly five unique strings in a queries array. JSON only.",
                &escalation_input,
                220,
            )?;
            parse_queries(&second)
                .map_err(|error| format!("9router escalation failed schema validation: {error}"))
        }
    }
}

#[tauri::command]
fn import_csv(app: AppHandle, contents: String) -> Result<ImportSummary, String> {
    let parsed = parse_csv(&contents)?;
    let mut database = connection(&app)?;
    let mut summary = import_raw_leads(&mut database, &parsed.leads)?;
    summary.rejected += parsed.errors.len();
    summary.errors.extend(parsed.errors);
    Ok(summary)
}

fn resolve_gosom_executable(resource_dir: &Path) -> Option<PathBuf> {
    [
        resource_dir.join("resources").join("gosom.exe"),
        resource_dir.join("gosom.exe"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("gosom.exe"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

fn resolve_httpx_executable(resource_dir: &Path) -> Option<PathBuf> {
    [
        resource_dir.join("resources").join("httpx.exe"),
        resource_dir.join("httpx.exe"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("httpx.exe"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

#[tauri::command]
fn discover_web(app: AppHandle, query: String) -> Result<ImportSummary, String> {
    let results = web_search(&query)?;
    let leads = results
        .into_iter()
        .map(|result| RawLead {
            business_name: result.name,
            trade: "Web search discovery".to_string(),
            area: "Online".to_string(),
            phone: None,
            website: Some(result.url.clone()),
            source_url: Some(result.url),
            source_place_id: None,
        })
        .collect::<Vec<_>>();
    let mut database = connection(&app)?;
    import_raw_leads(&mut database, &leads)
}

#[tauri::command]
fn research_lead(app: AppHandle, id: i64) -> Result<SiteSignals, String> {
    let database = connection(&app)?;
    let website: String = database
        .query_row("SELECT website FROM leads WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|_| "Lead has no researchable website".to_string())?;
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let executable = resolve_httpx_executable(&resource_dir)
        .ok_or_else(|| "Technology sidecar is not installed at resources/httpx.exe".to_string())?;
    let signals = inspect_site(&executable, &website)?;
    let signals_json = serde_json::to_string(&signals).map_err(|error| error.to_string())?;
    database.execute(
        "INSERT OR IGNORE INTO lead_research (lead_id, source_url, content_hash, signals_json, verdict, reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, signals.url, signals.content_hash, signals_json, signals.verdict, signals.reason],
    ).map_err(|error| error.to_string())?;
    if signals.verdict == "REJECT" {
        database.execute(
            "UPDATE leads SET status = 'Lost', gap_reason = ?1, next_action = 'Do not demo: existing preview app', updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![signals.reason, id],
        ).map_err(|error| error.to_string())?;
    } else if signals.verdict == "QUALIFY" {
        database.execute(
            "UPDATE leads SET status = 'Research', gap_reason = ?1,
             opportunity = 'Live personalisation preview setup', solution = 'Install and configure a proven Shopify preview app',
             template_id = 'shopify-engraving-preview', next_action = 'Confirm public DM path and product evidence',
             updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![signals.reason, id],
        ).map_err(|error| error.to_string())?;
    }
    Ok(signals)
}

#[tauri::command]
fn update_lead(app: AppHandle, id: i64, update: LeadUpdate) -> Result<(), String> {
    const STATUSES: [&str; 9] = [
        "New",
        "Research",
        "Qualified",
        "Demo",
        "Contacted",
        "Replied",
        "Won",
        "Lost",
        "Follow-up",
    ];
    if !STATUSES.contains(&update.status.as_str()) {
        return Err("Unknown pipeline status".to_string());
    }
    let database = connection(&app)?;
    let changed = database.execute(
        "UPDATE leads SET status=?1, next_action=?2, opportunity=?3, solution=?4, template_id=?5, demo_url=?6, updated_at=CURRENT_TIMESTAMP WHERE id=?7",
        params![update.status, update.next_action, update.opportunity, update.solution, update.template_id, update.demo_url, id],
    ).map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Lead not found".to_string());
    }
    Ok(())
}

#[tauri::command]
fn list_partners(app: AppHandle) -> Result<Vec<Partner>, String> {
    let database = connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT p.id, p.name, p.contact_name, p.email, p.phone, p.notes,
         COUNT(r.id), SUM(CASE WHEN r.conversion_status = 'Won' THEN 1 ELSE 0 END)
         FROM partners p LEFT JOIN partner_referrals r ON r.partner_id = p.id
         GROUP BY p.id ORDER BY p.name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(Partner {
                id: row.get(0)?,
                name: row.get(1)?,
                contact_name: row.get(2)?,
                email: row.get(3)?,
                phone: row.get(4)?,
                notes: row.get(5)?,
                referred_leads: row.get(6)?,
                conversions: row.get(7)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

#[tauri::command]
fn create_partner(
    app: AppHandle,
    name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    notes: String,
) -> Result<i64, String> {
    if name.trim().is_empty() {
        return Err("Partner name is required".to_string());
    }
    let database = connection(&app)?;
    database.execute(
        "INSERT INTO partners (name, contact_name, email, phone, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name.trim(), contact_name, email, phone, notes],
    ).map_err(|error| error.to_string())?;
    Ok(database.last_insert_rowid())
}

#[tauri::command]
fn discover_leads(app: AppHandle, queries: Vec<String>) -> Result<ImportSummary, String> {
    if queries.is_empty() || queries.len() > 10 {
        return Err("Discovery needs between 1 and 10 bounded queries".to_string());
    }
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let executable = resolve_gosom_executable(&resource_dir)
        .ok_or_else(|| "Gosom sidecar is not installed at resources/gosom.exe".to_string())?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("discovery");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let input = directory.join(format!("queries-{run_id}.txt"));
    let output = directory.join(format!("results-{run_id}.csv"));
    fs::write(&input, format!("{}\n", queries.join("\n"))).map_err(|error| error.to_string())?;
    let result = Command::new(executable)
        .arg("-input")
        .arg(&input)
        .arg("-results")
        .arg(&output)
        .arg("-fast-mode")
        .arg("-exit-on-inactivity")
        .arg("30s")
        .output()
        .map_err(|error| error.to_string())?;
    if !result.status.success() {
        let error = String::from_utf8_lossy(&result.stderr).trim().to_string();
        return Err(if error.is_empty() {
            format!("Gosom failed with {}", result.status)
        } else {
            error
        });
    }
    if !output.exists() {
        return Err("Gosom completed without a CSV result".to_string());
    }
    import_csv(
        app,
        fs::read_to_string(output).map_err(|error| error.to_string())?,
    )
}

#[tauri::command]
fn smart_review(
    app: AppHandle,
    business_name: String,
    website: Option<String>,
    evidence: String,
    budget: State<'_, CloudBudget>,
) -> Result<String, String> {
    let input = format!(
        "Business: {business_name}\nWebsite: {}\nEvidence signals: {evidence}",
        website.unwrap_or_else(|| "not found".to_string())
    );
    cached_completion(
        &app,
        &budget,
        &mid_model(),
        "Review only supplied evidence signals. Never invent facts, never change qualification, and answer in at most 60 words.",
        &input,
        140,
    )
}

#[tauri::command]
fn save_outcome(app: AppHandle, id: i64, outcome: String) -> Result<(), String> {
    let database = connection(&app)?;
    let channel: String = database
        .query_row(
            "SELECT contact_channel FROM leads WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let state = verification_state(&database, id, &channel).map_err(|error| error.to_string())?;
    if !state.eligible {
        return Err(format!(
            "Lead has {}/5 evidence passes for {channel}; contact outcome cannot be saved",
            state.passed
        ));
    }
    database
        .execute(
            "UPDATE leads SET outcome = ?1, status = 'Contacted', updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![outcome, id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CloudBudget::default())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            database_health,
            list_leads,
            save_outcome,
            model_status,
            plan_search,
            import_csv,
            discover_leads,
            discover_web,
            research_lead,
            update_lead,
            list_partners,
            create_partner,
            smart_review
        ])
        .run(tauri::generate_context!())
        .expect("error while running LeadFinder");
}
