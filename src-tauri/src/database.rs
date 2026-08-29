use rusqlite::Connection;
use sha2::{Digest, Sha256};

const CURRENT_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS leads (
        id INTEGER PRIMARY KEY,
        business_name TEXT NOT NULL,
        normalized_name TEXT NOT NULL,
        trade TEXT NOT NULL DEFAULT '',
        area TEXT NOT NULL DEFAULT '',
        phone TEXT,
        normalized_phone TEXT,
        website TEXT,
        normalized_domain TEXT,
        source_place_id TEXT,
        source_url TEXT,
        contact_channel TEXT NOT NULL DEFAULT 'dm',
        status TEXT NOT NULL DEFAULT 'New',
        next_action TEXT,
        opportunity TEXT,
        solution TEXT,
        template_id TEXT,
        demo_url TEXT,
        gap_reason TEXT NOT NULL DEFAULT 'UNCERTAIN',
        confidence TEXT NOT NULL DEFAULT 'uncertain',
        opener TEXT NOT NULL DEFAULT '',
        outcome TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS leads_normalized_phone_idx ON leads(normalized_phone)
        WHERE normalized_phone IS NOT NULL;
    CREATE INDEX IF NOT EXISTS leads_normalized_domain_idx ON leads(normalized_domain)
        WHERE normalized_domain IS NOT NULL;
    CREATE INDEX IF NOT EXISTS leads_normalized_name_idx ON leads(normalized_name);
    CREATE TABLE IF NOT EXISTS lead_evidence (
        id INTEGER PRIMARY KEY,
        lead_id INTEGER NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
        channel TEXT NOT NULL,
        pass_number INTEGER NOT NULL CHECK(pass_number BETWEEN 1 AND 5),
        pass_name TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('PENDING', 'PASS', 'FAIL', 'BLOCKED', 'UNCERTAIN')),
        source TEXT,
        evidence_json TEXT NOT NULL DEFAULT '{}',
        evidence_hash TEXT,
        checked_at TEXT,
        error TEXT,
        UNIQUE(lead_id, channel, pass_number)
    );
    CREATE TABLE IF NOT EXISTS lead_research (
        id INTEGER PRIMARY KEY,
        lead_id INTEGER NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
        source_url TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        signals_json TEXT NOT NULL CHECK(length(signals_json) <= 2048),
        verdict TEXT NOT NULL CHECK(verdict IN ('QUALIFY', 'REJECT', 'UNCERTAIN', 'ERROR')),
        reason TEXT NOT NULL,
        checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE(lead_id, content_hash)
    );
    CREATE TABLE IF NOT EXISTS partners (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        contact_name TEXT,
        email TEXT,
        phone TEXT,
        notes TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS partner_referrals (
        id INTEGER PRIMARY KEY,
        partner_id INTEGER NOT NULL REFERENCES partners(id) ON DELETE CASCADE,
        lead_id INTEGER NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
        conversion_status TEXT NOT NULL DEFAULT 'Referred',
        notes TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE(partner_id, lead_id)
    );
    CREATE TABLE IF NOT EXISTS model_cache (
        content_hash TEXT PRIMARY KEY,
        model TEXT NOT NULL,
        response TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
";

#[derive(Debug, PartialEq)]
pub struct VerificationState {
    pub passed: i64,
    pub eligible: bool,
}

pub fn ensure_evidence_passes(
    connection: &Connection,
    lead_id: i64,
    channel: &str,
) -> rusqlite::Result<()> {
    let names = pass_names(channel);
    for (index, name) in names.iter().enumerate() {
        connection.execute(
            "INSERT OR IGNORE INTO lead_evidence (
                lead_id, channel, pass_number, pass_name, status
             ) VALUES (?1, ?2, ?3, ?4, 'PENDING')",
            rusqlite::params![lead_id, channel, index as i64 + 1, name],
        )?;
    }
    Ok(())
}

pub fn record_evidence_pass(
    connection: &Connection,
    lead_id: i64,
    channel: &str,
    pass_number: i64,
    source: &str,
    evidence_json: &str,
) -> Result<(), String> {
    if !(1..=5).contains(&pass_number) {
        return Err("Evidence pass must be between 1 and 5".to_string());
    }
    if source.trim().is_empty() {
        return Err("Evidence source is required".to_string());
    }
    serde_json::from_str::<serde_json::Value>(evidence_json)
        .map_err(|error| format!("Evidence JSON is invalid: {error}"))?;
    ensure_evidence_passes(connection, lead_id, channel).map_err(|error| error.to_string())?;
    let prior_passes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM lead_evidence
             WHERE lead_id = ?1 AND channel = ?2 AND pass_number < ?3 AND status = 'PASS'",
            rusqlite::params![lead_id, channel, pass_number],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if prior_passes != pass_number - 1 {
        return Err(format!(
            "Evidence pass {pass_number} cannot pass before earlier passes"
        ));
    }
    let evidence_hash = format!("{:x}", Sha256::digest(evidence_json.as_bytes()));
    connection
        .execute(
            "UPDATE lead_evidence SET
                status = 'PASS', source = ?1, evidence_json = ?2,
                evidence_hash = ?3, checked_at = CURRENT_TIMESTAMP, error = NULL
             WHERE lead_id = ?4 AND channel = ?5 AND pass_number = ?6",
            rusqlite::params![
                source,
                evidence_json,
                evidence_hash,
                lead_id,
                channel,
                pass_number
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn verification_state(
    connection: &Connection,
    lead_id: i64,
    channel: &str,
) -> rusqlite::Result<VerificationState> {
    let passed = connection.query_row(
        "SELECT COUNT(*) FROM lead_evidence
         WHERE lead_id = ?1 AND channel = ?2 AND status = 'PASS'",
        rusqlite::params![lead_id, channel],
        |row| row.get(0),
    )?;
    Ok(VerificationState {
        passed,
        eligible: passed == 5,
    })
}

fn pass_names(channel: &str) -> [&'static str; 5] {
    match channel {
        "phone" => [
            "Schema and normalization",
            "Identity and deduplication",
            "Phone corroborated",
            "Opportunity evidence captured",
            "Compliance and deterministic qualification",
        ],
        "email" => [
            "Schema and normalization",
            "Identity and deduplication",
            "Public email path confirmed",
            "Opportunity evidence captured",
            "Deterministic qualification",
        ],
        _ => [
            "Schema and normalization",
            "Identity and deduplication",
            "Public DM path confirmed",
            "Opportunity evidence captured",
            "Deterministic qualification",
        ],
    }
}

fn table_exists(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for current in columns {
        if current? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn migrate_database(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let has_leads = table_exists(connection, "leads")?;
    let legacy_leads = has_leads && !column_exists(connection, "leads", "normalized_name")?;
    let transaction = connection.transaction()?;

    if legacy_leads {
        transaction.execute_batch("ALTER TABLE leads RENAME TO leads_legacy;")?;
    }
    transaction.execute_batch(CURRENT_SCHEMA)?;
    if legacy_leads {
        transaction.execute_batch(
            "INSERT INTO leads (
                id, business_name, normalized_name, trade, area, phone,
                normalized_phone, website, contact_channel, status, gap_reason,
                confidence, opener, outcome
            )
            SELECT
                id,
                business_name,
                lower(trim(business_name)),
                trade,
                area,
                NULLIF(trim(phone), ''),
                NULLIF(replace(replace(replace(replace(replace(lower(trim(phone)), ' ', ''), '-', ''), '(', ''), ')', ''), '+', ''), ''),
                website,
                CASE WHEN phone IS NULL OR trim(phone) = '' THEN 'dm' ELSE 'phone' END,
                CASE WHEN outcome IS NULL OR trim(outcome) = '' THEN 'New' ELSE 'Contacted' END,
                gap_reason,
                confidence,
                opener,
                outcome
            FROM leads_legacy;
            DROP TABLE leads_legacy;",
        )?;
    }
    transaction.pragma_update(None, "user_version", 2)?;
    transaction.commit()
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_evidence_passes, migrate_database, record_evidence_pass, verification_state,
        VerificationState,
    };
    use rusqlite::{params, Connection};

    #[test]
    fn migration_preserves_existing_outcome_and_allows_missing_phone() {
        let mut connection = Connection::open_in_memory().expect("legacy database opens");
        connection
            .execute_batch(
                "CREATE TABLE leads (
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
                );
                INSERT INTO leads (
                    id, business_name, trade, area, phone, website, gap_reason,
                    confidence, eligible, opener, outcome, verification_count
                ) VALUES (
                    41, 'O''Brien Awards', 'Trophies', 'Derby', '01332 555 014',
                    'https://obrien.example', 'UNCERTAIN', 'uncertain', 0,
                    'Existing opener', 'Callback', 3
                );",
            )
            .expect("legacy fixture created");

        migrate_database(&mut connection).expect("migration succeeds");

        let preserved: (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT business_name, phone, outcome FROM leads WHERE id = 41",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("legacy lead remains");
        assert_eq!(
            preserved,
            (
                "O'Brien Awards".to_string(),
                Some("01332 555 014".to_string()),
                Some("Callback".to_string())
            )
        );

        connection
            .execute(
                "INSERT INTO leads (
                    business_name, normalized_name, trade, area, phone, status,
                    gap_reason, confidence, opener
                ) VALUES (?1, ?2, ?3, ?4, NULL, 'New', 'UNCERTAIN', 'uncertain', '')",
                params![
                    "Online Gifts Ltd",
                    "online gifts ltd",
                    "Personalised gifts",
                    "UK"
                ],
            )
            .expect("phone-less lead can be stored");
    }

    #[test]
    fn five_pass_gate_is_persisted_sequential_and_derived() {
        let mut connection = Connection::open_in_memory().expect("database opens");
        migrate_database(&mut connection).expect("schema migrates");
        connection
            .execute(
                "INSERT INTO leads (
                    business_name, normalized_name, trade, area, phone, contact_channel,
                    status, gap_reason, confidence, opener
                ) VALUES ('Online Gifts', 'online gifts', 'Personalised gifts', 'UK', NULL,
                    'dm', 'Research', 'UNCERTAIN', 'uncertain', '')",
                [],
            )
            .expect("lead inserts");
        let lead_id = connection.last_insert_rowid();

        ensure_evidence_passes(&connection, lead_id, "dm").expect("pass scaffold persists");
        assert_eq!(
            verification_state(&connection, lead_id, "dm").expect("state reads"),
            VerificationState {
                passed: 0,
                eligible: false
            }
        );
        assert!(record_evidence_pass(
            &connection,
            lead_id,
            "dm",
            2,
            "import",
            r#"{"dedupe":"clear"}"#
        )
        .is_err());

        for pass_number in 1..=5 {
            record_evidence_pass(
                &connection,
                lead_id,
                "dm",
                pass_number,
                "test evidence",
                &format!(r#"{{"pass":{pass_number}}}"#),
            )
            .expect("next pass records");
        }
        assert_eq!(
            verification_state(&connection, lead_id, "dm").expect("state reads"),
            VerificationState {
                passed: 5,
                eligible: true
            }
        );
    }
}
