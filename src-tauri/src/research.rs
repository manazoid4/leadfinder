use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::process::Command;
use std::time::Duration;
use url::Url;

const REJECT_FINGERPRINTS: [&str; 9] = [
    "customily",
    "kickflip",
    "easify",
    "zakeke",
    "inkybay",
    "tailorkit",
    "product-personalizer",
    "teeinblue",
    "gokickflip",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebResult {
    pub name: String,
    pub url: String,
    pub domain: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSignals {
    pub url: String,
    pub status_code: u16,
    pub title: String,
    pub technologies: Vec<String>,
    pub reject_fingerprints: Vec<String>,
    pub shopify: bool,
    pub content_hash: String,
    pub verdict: String,
    pub reason: String,
}

#[derive(Deserialize)]
struct HttpxOutput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    tech: Vec<String>,
    #[serde(default)]
    status_code: u16,
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("LeadFinder/0.1 research (manual outreach)")
        .build()
        .map_err(|error| error.to_string())
}

fn decode_html(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#x27;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub fn parse_search_results(html: &str) -> Vec<WebResult> {
    let anchor =
        Regex::new(r#"(?s)<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
            .expect("static search result regex compiles");
    let tags = Regex::new(r"<[^>]+>").expect("static tag regex compiles");
    let mut results = Vec::new();
    for captures in anchor.captures_iter(html) {
        let href = decode_html(&captures[1]);
        let destination = Url::parse(&href)
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "uddg")
                    .map(|(_, value)| value.into_owned())
            })
            .unwrap_or(href);
        let Ok(url) = Url::parse(&destination) else {
            continue;
        };
        let Some(host) = url.host_str() else { continue };
        let domain = host.trim_start_matches("www.").to_ascii_lowercase();
        if results
            .iter()
            .any(|result: &WebResult| result.domain == domain)
        {
            continue;
        }
        results.push(WebResult {
            name: decode_html(tags.replace_all(&captures[2], "").trim()),
            url: destination,
            domain,
        });
        if results.len() == 10 {
            break;
        }
    }
    results
}

pub fn web_search(query: &str) -> Result<Vec<WebResult>, String> {
    let query = query.trim();
    if query.len() < 3 || query.len() > 160 {
        return Err("Web search query must be 3-160 characters".to_string());
    }
    let response = http_client()?
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .map_err(|error| format!("Web search unavailable: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Web search returned {}", response.status()));
    }
    let results = parse_search_results(&response.text().map_err(|error| error.to_string())?);
    if results.is_empty() {
        return Err("Web search returned no usable company URLs".to_string());
    }
    Ok(results)
}

pub fn inspect_site(
    httpx_executable: &std::path::Path,
    website: &str,
) -> Result<SiteSignals, String> {
    let parsed = Url::parse(website).map_err(|_| "Website must be an absolute http(s) URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Website must use http or https".to_string());
    }
    let response = http_client()?
        .get(website)
        .send()
        .map_err(|error| format!("Website unavailable: {error}"))?;
    let status_code = response.status().as_u16();
    let bytes = response.bytes().map_err(|error| error.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("Website response exceeds the 2MB deterministic scan limit".to_string());
    }
    let lowercase = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    let reject_fingerprints = REJECT_FINGERPRINTS
        .iter()
        .filter(|fingerprint| lowercase.contains(**fingerprint))
        .map(|fingerprint| (*fingerprint).to_string())
        .collect::<Vec<_>>();
    let content_hash = format!("{:x}", Sha256::digest(&bytes));

    let output = Command::new(httpx_executable)
        .args([
            "-u",
            website,
            "-tech-detect",
            "-json",
            "-silent",
            "-timeout",
            "20",
            "-retries",
            "1",
            "-no-color",
        ])
        .output()
        .map_err(|error| format!("Technology sidecar failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!("Technology sidecar failed with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| "Technology sidecar returned no JSON result".to_string())?;
    let detected: HttpxOutput = serde_json::from_str(line)
        .map_err(|error| format!("Technology sidecar returned malformed JSON: {error}"))?;
    let shopify = detected
        .tech
        .iter()
        .any(|tech| tech.eq_ignore_ascii_case("shopify"))
        || lowercase.contains("cdn.shopify.com")
        || lowercase.contains("myshopify.com");
    let (verdict, reason) = if !reject_fingerprints.is_empty() {
        (
            "REJECT",
            format!(
                "Existing preview app detected: {}",
                reject_fingerprints.join(", ")
            ),
        )
    } else if shopify {
        (
            "QUALIFY",
            "Shopify detected and no supplied preview-app reject fingerprint found".to_string(),
        )
    } else {
        ("UNCERTAIN", "Shopify was not detected; choose a matching local-business template or reject manually".to_string())
    };
    let signals = SiteSignals {
        url: website.to_string(),
        status_code: if detected.status_code == 0 {
            status_code
        } else {
            detected.status_code
        },
        title: detected.title,
        technologies: detected.tech,
        reject_fingerprints,
        shopify,
        content_hash,
        verdict: verdict.to_string(),
        reason,
    };
    let encoded = serde_json::to_vec(&signals).map_err(|error| error.to_string())?;
    if encoded.len() > 2_048 {
        return Err(format!(
            "Extracted signals are {} bytes; maximum is 2048",
            encoded.len()
        ));
    }
    Ok(signals)
}

#[cfg(test)]
mod tests {
    use super::{inspect_site, parse_search_results};
    use std::path::PathBuf;

    #[test]
    fn search_results_are_unwrapped_and_deduplicated_by_domain() {
        let html = r#"<a rel="nofollow" class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fgift.example%2Fwallet">Gift &amp; Co</a><a class="result__a" href="https://gift.example/about">Duplicate</a>"#;
        let results = parse_search_results(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].domain, "gift.example");
        assert_eq!(results[0].name, "Gift & Co");
    }

    #[test]
    #[ignore = "live network and sidecar contract test"]
    fn real_shopify_sites_qualify_or_reject_from_current_evidence() {
        let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("httpx.exe");
        let rejected =
            inspect_site(&sidecar, "https://www.paarsawahid.com").expect("Paarsa can be inspected");
        assert_eq!(rejected.verdict, "REJECT");
        assert!(rejected
            .reject_fingerprints
            .iter()
            .any(|item| item == "easify"));

        let qualified = inspect_site(&sidecar, "https://www.rfidwallets.co.uk")
            .expect("RFID Wallets UK can be inspected");
        assert_eq!(qualified.verdict, "QUALIFY");
        assert!(qualified.shopify);
        assert!(
            serde_json::to_vec(&qualified)
                .expect("signals serialize")
                .len()
                <= 2_048
        );
    }
}
