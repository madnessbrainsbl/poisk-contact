use chrono::{DateTime, Utc};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct RdapDomainInfo {
    pub domain: String,
    pub registered_at: Option<String>,
    pub age_days: Option<i64>,
    pub age_years: Option<f64>,
    pub registrar: Option<String>,
    pub is_young_domain: bool,
    pub is_mature_domain: bool,
}

pub async fn lookup_rdap_domain(
    client: &reqwest::Client,
    domain: &str,
) -> Result<RdapDomainInfo, String> {
    let clean = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim_end_matches('/')
        .to_lowercase();

    if clean.is_empty() {
        return Err("Пустое доменное имя".to_string());
    }

    let url = format!("https://rdap.org/domain/{}", clean);
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(6))
        .header("Accept", "application/rdap+json, application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) RDAP/1.0",
        )
        .send()
        .await
        .map_err(|e| format!("RDAP запрос не удался: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("RDAP вернул статус {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Ошибка парсинга RDAP JSON: {}", e))?;

    let mut info = RdapDomainInfo {
        domain: clean.clone(),
        ..Default::default()
    };

    // 1. Extract registration date from events
    if let Some(events) = json.get("events").and_then(|e| e.as_array()) {
        for ev in events {
            let action = ev.get("eventAction").and_then(|a| a.as_str()).unwrap_or("");
            if action == "registration" {
                if let Some(date_str) = ev.get("eventDate").and_then(|d| d.as_str()) {
                    info.registered_at = Some(date_str.to_string());
                    if let Ok(parsed_date) = DateTime::parse_from_rfc3339(date_str) {
                        let now = Utc::now();
                        let days = (now - parsed_date.with_timezone(&Utc)).num_days();
                        if days >= 0 {
                            info.age_days = Some(days);
                            info.age_years = Some(days as f64 / 365.25);
                            info.is_young_domain = days < 60;
                            info.is_mature_domain = days > 1095; // > 3 years
                        }
                    }
                    break;
                }
            }
        }
    }

    // 2. Extract Registrar name
    if let Some(entities) = json.get("entities").and_then(|e| e.as_array()) {
        for ent in entities {
            let roles = ent.get("roles").and_then(|r| r.as_array());
            let is_registrar = roles.map_or(false, |r| {
                r.iter().any(|val| val.as_str() == Some("registrar"))
            });

            if is_registrar {
                // Try vcardArray fn
                if let Some(vcard) = ent.get("vcardArray").and_then(|v| v.as_array()) {
                    if let Some(props) = vcard.get(1).and_then(|p| p.as_array()) {
                        for prop in props {
                            if let Some(p_arr) = prop.as_array() {
                                if p_arr.first().and_then(|f| f.as_str()) == Some("fn") {
                                    if let Some(name) = p_arr.get(3).and_then(|n| n.as_str()) {
                                        info.registrar = Some(name.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                if info.registrar.is_none() {
                    if let Some(handle) = ent.get("handle").and_then(|h| h.as_str()) {
                        info.registrar = Some(handle.to_string());
                    }
                }
                break;
            }
        }
    }

    Ok(info)
}
