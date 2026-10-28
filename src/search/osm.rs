//! Free, key-less business discovery from OpenStreetMap via the Overpass API.
//! OSM lists businesses with their own `website` tag, so it finds company sites that
//! public search pages bury under directories. Found sites still go through the crawler,
//! which keeps only contacts published on the company's own pages.

use super::web::SearchHit;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

// Public instances; each one is often busy, so the next is tried on failure.
// https://wiki.openstreetmap.org/wiki/Overpass_API#Public_Overpass_API_instances
const ENDPOINTS: [&str; 3] = [
    "https://overpass-api.de/api/interpreter",
    "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
    "https://overpass.private.coffee/api/interpreter",
];

/// Websites of OSM businesses inside `area` whose name or category matches `sector`.
pub(super) async fn business_websites(
    client: &Client,
    area: u64,
    sector: &str,
) -> Result<Vec<SearchHit>, String> {
    let pattern = sector_pattern(sector).ok_or("no searchable words in the category")?;
    // Single-key filters only: a key-regex across many tags times out on the public servers.
    let query = format!(
        r#"[out:json][timeout:90];area(id:{area})->.a;(nwr(area.a)["website"]["name"~"{pattern}",i];nwr(area.a)["contact:website"]["name"~"{pattern}",i];);out tags;"#
    );
    let form = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("data", &query)
        .finish();
    let mut errors = Vec::new();
    // Public servers shed load with 502/504 for short periods: one more round after a pause.
    for (round, endpoint) in ENDPOINTS.iter().chain(ENDPOINTS.iter()).enumerate() {
        if round == ENDPOINTS.len() {
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
        let response = client
            .post(*endpoint)
            .header("User-Agent", "PoiskContact/0.1 (desktop contact search)")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form.clone())
            .timeout(Duration::from_secs(120))
            .send()
            .await;
        let body = match response {
            Ok(r) if r.status().is_success() => r.text().await.map_err(|e| e.to_string()),
            Ok(r) => Err(format!("HTTP {}", r.status().as_u16())),
            Err(e) => Err(e.to_string()),
        };
        match body.and_then(|b| parse_websites(&b)) {
            Ok(hits) => return Ok(hits),
            Err(e) => errors.push(format!("{endpoint}: {e}")),
        }
    }
    Err(format!("OpenStreetMap unavailable ({})", errors.join("; ")))
}

/// Case-insensitive regex of word stems: "фотостудия" -> "фотосту", so inflected names match.
// ponytail: crude suffix trim instead of a stemmer; swap in a real stemmer if recall is poor.
fn sector_pattern(sector: &str) -> Option<String> {
    let stems: Vec<String> = sector
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .map(|w| {
            let chars: Vec<char> = w.to_lowercase().chars().collect();
            let keep = if chars.len() > 6 {
                chars.len() - 3
            } else {
                chars.len()
            };
            chars[..keep].iter().collect()
        })
        .collect();
    (!stems.is_empty()).then(|| stems.join("|"))
}

fn parse_websites(body: &str) -> Result<Vec<SearchHit>, String> {
    let json: Value =
        serde_json::from_str(body).map_err(|_| "not a JSON response (server busy)")?;
    let mut hits: Vec<SearchHit> = Vec::new();
    for tags in json["elements"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| &e["tags"])
    {
        let Some(site) = ["website", "contact:website", "url"]
            .iter()
            .find_map(|k| tags[*k].as_str())
        else {
            continue;
        };
        // A tag may hold several sites separated by ';'.
        let site = site.split(';').next().unwrap_or("").trim();
        if site.is_empty() {
            continue;
        }
        let url = if site.starts_with("http") {
            site.to_string()
        } else {
            format!("https://{site}")
        };
        if !hits.iter().any(|h| h.url == url) {
            hits.push(SearchHit { url });
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_stem_pattern_and_reads_websites() {
        assert_eq!(sector_pattern("фотостудия").as_deref(), Some("фотосту"));
        assert_eq!(
            sector_pattern("Photo studio").as_deref(),
            Some("photo|studio")
        );
        assert_eq!(sector_pattern("  - "), None);

        let body = r#"{"elements":[
            {"tags":{"name":"A","website":"https://a.ru/"}},
            {"tags":{"name":"B","contact:website":"b.ru;https://vk.com/b"}},
            {"tags":{"name":"A dup","website":"https://a.ru/"}},
            {"tags":{"name":"No site"}}]}"#;
        let urls: Vec<_> = parse_websites(body)
            .unwrap()
            .into_iter()
            .map(|h| h.url)
            .collect();
        assert_eq!(urls, ["https://a.ru/", "https://b.ru"]);
        assert!(parse_websites("<html>504</html>").is_err());
    }
}
