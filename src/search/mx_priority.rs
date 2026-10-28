use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MxRecordEntry {
    pub preference: u16,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxPriorityAnalysis {
    pub domain: String,
    pub primary_mx: Option<MxRecordEntry>,
    pub backup_relays: Vec<MxRecordEntry>,
    pub has_redundancy: bool,
    pub provider_family: String,
    pub recommendation: String,
}

pub fn parse_mx_entries_from_doh_json(json: &Value) -> Vec<MxRecordEntry> {
    let mut entries = Vec::new();
    if let Some(answers) = json.get("Answer").and_then(|v| v.as_array()) {
        for ans in answers {
            let ans_type = ans.get("type").and_then(|v| v.as_i64()).unwrap_or(0);
            if ans_type == 15 {
                if let Some(data) = ans.get("data").and_then(|v| v.as_str()) {
                    let parts: Vec<&str> = data.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let pref = parts[0].parse::<u16>().unwrap_or(10);
                        let host = parts[1].trim_end_matches('.').to_lowercase();
                        entries.push(MxRecordEntry {
                            preference: pref,
                            host,
                        });
                    }
                }
            }
        }
    }
    // Sort ascending by preference
    entries.sort_by_key(|e| e.preference);
    entries
}

pub fn detect_provider_family(host: &str) -> &'static str {
    let lower = host.to_lowercase();
    if lower.contains("google.com") || lower.contains("googlemail.com") || lower.contains("aspmx") {
        "Google Workspace / Gmail"
    } else if lower.contains("yandex.net")
        || lower.contains("yandex.ru")
        || lower.contains("yamail")
    {
        "Yandex 360 / Yandex Mail"
    } else if lower.contains("mail.ru") || lower.contains("vk.com") || lower.contains("mxs.mail.ru")
    {
        "VK WorkSpace / Mail.ru"
    } else if lower.contains("outlook.com")
        || lower.contains("microsoft.com")
        || lower.contains("office365")
    {
        "Microsoft 365 / Exchange Online"
    } else if lower.contains("protonmail.ch") || lower.contains("proton.me") {
        "ProtonMail Business"
    } else if lower.contains("zoho.com") || lower.contains("zoho.eu") {
        "Zoho Mail"
    } else if lower.contains("fastmail.com") {
        "Fastmail"
    } else if lower.contains("beget.com")
        || lower.contains("reg.ru")
        || lower.contains("timeweb.ru")
        || lower.contains("cpanel")
    {
        "Shared Hosting / cPanel Webmail"
    } else {
        "Self-Hosted / Enterprise On-Premise MTA"
    }
}

pub fn analyze_mx_priorities(domain: &str, mut entries: Vec<MxRecordEntry>) -> MxPriorityAnalysis {
    entries.sort_by_key(|e| e.preference);

    let primary_mx = entries.first().cloned();
    let backup_relays = if entries.len() > 1 {
        entries[1..].to_vec()
    } else {
        Vec::new()
    };

    let has_redundancy = entries.len() >= 2;
    let provider = primary_mx
        .as_ref()
        .map(|p| detect_provider_family(&p.host))
        .unwrap_or("No MX Records Found")
        .to_string();

    let recommendation = if entries.is_empty() {
        "Critical error: Domain has no published MX records. Incoming mail cannot be received."
            .to_string()
    } else if !has_redundancy {
        format!(
            "Single point of failure: Only 1 MX gateway ({}). Lacks secondary backup relay redundancy.",
            primary_mx.as_ref().map(|p| p.host.as_str()).unwrap_or("")
        )
    } else {
        format!(
            "High availability: Primary authoritative gateway ({}, priority {}) backed by {} redundant relay(s).",
            primary_mx.as_ref().map(|p| p.host.as_str()).unwrap_or(""),
            primary_mx.as_ref().map(|p| p.preference).unwrap_or(0),
            backup_relays.len()
        )
    };

    MxPriorityAnalysis {
        domain: domain.to_string(),
        primary_mx,
        backup_relays,
        has_redundancy,
        provider_family: provider,
        recommendation,
    }
}

pub async fn resolve_and_prioritize_mx(
    client: &Client,
    domain: &str,
) -> Result<MxPriorityAnalysis, String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_matches('/')
        .to_lowercase();
    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=MX",
        clean_domain
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("DoH MX lookup failed: {}", e))?;

    let json = resp
        .json::<Value>()
        .await
        .map_err(|e| format!("Invalid DoH JSON response: {}", e))?;
    let entries = parse_mx_entries_from_doh_json(&json);
    Ok(analyze_mx_priorities(&clean_domain, entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_mx_priority_sorting() {
        let entries = vec![
            MxRecordEntry {
                preference: 20,
                host: "alt2.aspmx.l.google.com".to_string(),
            },
            MxRecordEntry {
                preference: 5,
                host: "aspmx.l.google.com".to_string(),
            },
            MxRecordEntry {
                preference: 10,
                host: "alt1.aspmx.l.google.com".to_string(),
            },
        ];
        let analysis = analyze_mx_priorities("company.com", entries);
        assert!(analysis.has_redundancy);
        let primary = analysis.primary_mx.expect("Primary MX should be isolated");
        assert_eq!(primary.preference, 5);
        assert_eq!(primary.host, "aspmx.l.google.com");
        assert_eq!(analysis.backup_relays.len(), 2);
        assert_eq!(analysis.provider_family, "Google Workspace / Gmail");
        assert!(analysis.recommendation.contains("High availability"));
    }

    #[test]
    fn test_yandex_single_mx_redundancy_check() {
        let entries = vec![MxRecordEntry {
            preference: 10,
            host: "mx.yandex.ru".to_string(),
        }];
        let analysis = analyze_mx_priorities("corp.ru", entries);
        assert!(!analysis.has_redundancy);
        let primary = analysis.primary_mx.expect("Primary MX should exist");
        assert_eq!(primary.preference, 10);
        assert_eq!(primary.host, "mx.yandex.ru");
        assert_eq!(analysis.backup_relays.len(), 0);
        assert_eq!(analysis.provider_family, "Yandex 360 / Yandex Mail");
        assert!(analysis.recommendation.contains("Single point of failure"));
    }

    #[test]
    fn test_parse_doh_json_answers() {
        let json: Value = serde_json::json!({
            "Status": 0,
            "Answer": [
                { "name": "example.com", "type": 15, "data": "10 mail1.example.com." },
                { "name": "example.com", "type": 15, "data": "20 mail2.example.com." },
                { "name": "example.com", "type": 1, "data": "93.184.216.34" }
            ]
        });
        let entries = parse_mx_entries_from_doh_json(&json);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].preference, 10);
        assert_eq!(entries[0].host, "mail1.example.com");
        assert_eq!(entries[1].preference, 20);
        assert_eq!(entries[1].host, "mail2.example.com");
    }
}
