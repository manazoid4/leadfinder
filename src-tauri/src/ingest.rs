#[derive(Debug, PartialEq)]
pub struct RawLead {
    pub business_name: String,
    pub trade: String,
    pub area: String,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub source_url: Option<String>,
    pub source_place_id: Option<String>,
}

use crate::database::{ensure_evidence_passes, record_evidence_pass};
use csv::StringRecord;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    pub deduplicated: usize,
    pub rejected: usize,
    pub errors: Vec<String>,
}

pub struct ParsedCsv {
    pub leads: Vec<RawLead>,
    pub errors: Vec<String>,
}

fn normalize_header(value: &str) -> String {
    value
        .trim_start_matches('\u{feff}')
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
}

fn index_headers(headers: &StringRecord) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| (normalize_header(header), index))
        .collect()
}

fn field<'a>(
    row: &'a StringRecord,
    headers: &HashMap<String, usize>,
    aliases: &[&str],
) -> Option<&'a str> {
    aliases
        .iter()
        .find_map(|alias| headers.get(*alias).and_then(|index| row.get(*index)))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn parse_csv(contents: &str) -> Result<ParsedCsv, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(contents.as_bytes());
    let headers = index_headers(
        reader
            .headers()
            .map_err(|error| format!("CSV headers are invalid: {error}"))?,
    );
    if ![
        "business_name",
        "business",
        "company",
        "company_name",
        "title",
        "name",
    ]
    .iter()
    .any(|alias| headers.contains_key(*alias))
    {
        return Err("CSV needs a business-name header".to_string());
    }

    let mut parsed = ParsedCsv {
        leads: Vec::new(),
        errors: Vec::new(),
    };
    for (index, result) in reader.records().enumerate() {
        let row_number = index + 2;
        let row = match result {
            Ok(row) => row,
            Err(error) => {
                parsed.errors.push(format!("CSV row {row_number}: {error}"));
                continue;
            }
        };
        let Some(business_name) = field(
            &row,
            &headers,
            &[
                "business_name",
                "business",
                "company",
                "company_name",
                "title",
                "name",
            ],
        ) else {
            parsed
                .errors
                .push(format!("CSV row {row_number}: business name is missing"));
            continue;
        };
        parsed.leads.push(RawLead {
            business_name: business_name.to_string(),
            trade: field(&row, &headers, &["trade", "category", "type"])
                .unwrap_or("")
                .to_string(),
            area: field(
                &row,
                &headers,
                &["area", "address", "city", "complete_address"],
            )
            .unwrap_or("")
            .to_string(),
            phone: field(&row, &headers, &["phone", "telephone", "phone_number"])
                .map(str::to_string),
            website: field(&row, &headers, &["website", "site"]).map(str::to_string),
            source_url: field(
                &row,
                &headers,
                &["link", "maps_link", "source_url", "google_maps_url"],
            )
            .map(str::to_string),
            source_place_id: field(&row, &headers, &["place_id", "data_id", "cid", "source_id"])
                .map(str::to_string),
        });
    }
    Ok(parsed)
}

pub fn import_raw_leads(
    connection: &mut Connection,
    leads: &[RawLead],
) -> Result<ImportSummary, String> {
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let mut summary = ImportSummary {
        imported: 0,
        deduplicated: 0,
        rejected: 0,
        errors: Vec::new(),
    };
    for (index, lead) in leads.iter().enumerate() {
        let normalized_name = normalize_name(&lead.business_name);
        if normalized_name.is_empty() {
            summary.rejected += 1;
            summary.errors.push(format!(
                "Row {}: business name is empty after normalization",
                index + 1
            ));
            continue;
        }
        let normalized_phone = lead.phone.as_deref().and_then(normalize_phone);
        let normalized_domain = lead.website.as_deref().and_then(normalize_domain);
        let normalized_area = normalize_name(&lead.area);
        let existing_id = transaction
            .query_row(
                "SELECT id FROM leads
                 WHERE (?1 IS NOT NULL AND normalized_phone = ?1)
                    OR (?2 IS NOT NULL AND normalized_domain = ?2)
                    OR (normalized_name = ?3 AND lower(trim(area)) = ?4)
                    OR (?5 IS NOT NULL AND source_place_id = ?5)
                 ORDER BY id
                 LIMIT 1",
                params![
                    normalized_phone,
                    normalized_domain,
                    normalized_name,
                    normalized_area,
                    lead.source_place_id
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let (lead_id, is_new) = if let Some(id) = existing_id {
            transaction
                .execute(
                    "UPDATE leads SET
                        phone = COALESCE(phone, ?1),
                        normalized_phone = COALESCE(normalized_phone, ?2),
                        website = COALESCE(website, ?3),
                        normalized_domain = COALESCE(normalized_domain, ?4),
                        source_url = COALESCE(source_url, ?5),
                        source_place_id = COALESCE(source_place_id, ?6),
                        updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?7",
                    params![
                        lead.phone,
                        normalized_phone,
                        lead.website,
                        normalized_domain,
                        lead.source_url,
                        lead.source_place_id,
                        id
                    ],
                )
                .map_err(|error| error.to_string())?;
            summary.deduplicated += 1;
            (id, false)
        } else {
            transaction
                .execute(
                    "INSERT INTO leads (
                    business_name, normalized_name, trade, area, phone,
                    normalized_phone, website, normalized_domain, source_place_id,
                    source_url, contact_channel, status, gap_reason, confidence, opener
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    CASE WHEN ?5 IS NULL THEN 'dm' ELSE 'phone' END,
                    'New', 'UNCERTAIN', 'uncertain', ''
                )",
                    params![
                        lead.business_name,
                        normalized_name,
                        lead.trade,
                        lead.area,
                        lead.phone,
                        normalized_phone,
                        lead.website,
                        normalized_domain,
                        lead.source_place_id,
                        lead.source_url
                    ],
                )
                .map_err(|error| error.to_string())?;
            summary.imported += 1;
            (transaction.last_insert_rowid(), true)
        };
        let channel = if lead.phone.is_some() { "phone" } else { "dm" };
        ensure_evidence_passes(&transaction, lead_id, channel)
            .map_err(|error| error.to_string())?;
        if is_new {
            record_evidence_pass(
                &transaction,
                lead_id,
                channel,
                1,
                "CSV/Gosom import",
                &serde_json::json!({
                    "business_name": lead.business_name,
                    "phone": lead.phone,
                    "website": lead.website
                })
                .to_string(),
            )?;
            record_evidence_pass(
                &transaction,
                lead_id,
                channel,
                2,
                "normalized dedupe",
                &serde_json::json!({
                    "normalized_name": normalized_name,
                    "normalized_phone": normalized_phone,
                    "normalized_domain": normalized_domain,
                    "dedupe": "clear"
                })
                .to_string(),
            )?;
        }
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(summary)
}

fn normalize_name(value: &str) -> String {
    value
        .replace('&', " and ")
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_phone(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    let normalized = if digits.starts_with("44") && digits.len() >= 11 {
        format!("0{}", &digits[2..])
    } else {
        digits
    };
    (normalized.len() >= 10).then_some(normalized)
}

fn normalize_domain(value: &str) -> Option<String> {
    let candidate = if value.contains("://") {
        value.to_string()
    } else {
        format!("https://{value}")
    };
    Url::parse(&candidate)
        .ok()?
        .host_str()
        .map(|host| host.to_ascii_lowercase())
        .map(|host| host.strip_prefix("www.").unwrap_or(&host).to_string())
}

#[cfg(test)]
mod tests {
    use super::{import_raw_leads, parse_csv, RawLead};
    use crate::database::migrate_database;
    use rusqlite::Connection;

    #[test]
    fn gosom_csv_maps_headers_and_accepts_phone_less_rows() {
        let contents = concat!(
            "input_id,link,title,category,address,website,phone,data_id\n",
            "0,https://maps.google.test/place/123,\"Smith, Jones & Co\",Trophy shop,",
            "\"1 High Street, Derby\",https://smithjones.example,,place123\n"
        );

        let parsed = parse_csv(contents).expect("valid Gosom CSV parses");

        assert_eq!(
            parsed.leads,
            vec![RawLead {
                business_name: "Smith, Jones & Co".to_string(),
                trade: "Trophy shop".to_string(),
                area: "1 High Street, Derby".to_string(),
                phone: None,
                website: Some("https://smithjones.example".to_string()),
                source_url: Some("https://maps.google.test/place/123".to_string()),
                source_place_id: Some("place123".to_string()),
            }]
        );
        assert!(parsed.errors.is_empty());
    }

    #[test]
    fn malformed_rows_are_visible_without_discarding_valid_rows() {
        let parsed =
            parse_csv("name,website\n,https://missing.example\nGood Gifts,https://good.example\n")
                .expect("headers are valid");
        assert_eq!(parsed.leads.len(), 1);
        assert_eq!(parsed.leads[0].business_name, "Good Gifts");
        assert_eq!(parsed.errors, vec!["CSV row 2: business name is missing"]);
    }

    #[test]
    fn repeated_import_deduplicates_without_overwriting_outcome() {
        let mut connection = Connection::open_in_memory().expect("database opens");
        migrate_database(&mut connection).expect("schema migrates");
        let first = RawLead {
            business_name: "Smith & Jones Gifts Ltd".to_string(),
            trade: "Personalised gifts".to_string(),
            area: "Derby".to_string(),
            phone: Some("+44 1332 555 014".to_string()),
            website: Some("https://www.smithjones.example/products/wallet".to_string()),
            source_url: Some("https://maps.google.test/place/123".to_string()),
            source_place_id: Some("place123".to_string()),
        };
        let duplicate = RawLead {
            business_name: "SMITH AND JONES GIFTS LTD".to_string(),
            trade: "Gift shop".to_string(),
            area: "Derby".to_string(),
            phone: Some("01332 555014".to_string()),
            website: Some("https://smithjones.example".to_string()),
            source_url: first.source_url.clone(),
            source_place_id: first.source_place_id.clone(),
        };

        let first_result = import_raw_leads(&mut connection, &[first]).expect("first import works");
        assert_eq!(first_result.imported, 1);
        connection
            .execute("UPDATE leads SET outcome = 'Interested' WHERE id = 1", [])
            .expect("real outcome saved");
        let second_result =
            import_raw_leads(&mut connection, &[duplicate]).expect("repeat import works");

        assert_eq!(second_result.deduplicated, 1);
        let result: (i64, Option<String>) = connection
            .query_row("SELECT COUNT(*), MAX(outcome) FROM leads", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("lead count reads");
        assert_eq!(result, (1, Some("Interested".to_string())));
    }
}
