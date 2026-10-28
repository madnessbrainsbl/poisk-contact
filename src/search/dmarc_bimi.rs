use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmarcPolicy {
    pub version: String,
    pub policy: String, // reject, quarantine, none
    pub subdomain_policy: Option<String>,
    pub percentage: u8, // 0 - 100
    pub aggregate_reports: Vec<String>,
    pub forensic_reports: Vec<String>,
    pub raw_record: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BimiPolicy {
    pub version: String,
    pub logo_url: Option<String>,
    pub vmc_cert_url: Option<String>,
    pub raw_record: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmarcAuditResult {
    pub domain: String,
    pub has_dmarc: bool,
    pub dmarc_policy: Option<DmarcPolicy>,
    pub has_bimi: bool,
    pub bimi_policy: Option<BimiPolicy>,
    pub is_esp_2024_compliant: bool, // Google/Yahoo Feb 2024 requirement
    pub enforcement_grade: String,   // A+, A, B, C, F
    pub security_score: u32,         // 0 - 100
    pub recommendation: String,
}

pub fn parse_dmarc_record(raw: &str) -> Option<DmarcPolicy> {
    let clean = raw.trim().trim_matches('"');
    if !clean.to_lowercase().starts_with("v=dmarc1") {
        return None;
    }

    let mut map: HashMap<String, String> = HashMap::new();
    for token in clean.split(';') {
        let parts: Vec<&str> = token.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].trim().to_lowercase(), parts[1].trim().to_string());
        }
    }

    let version = map
        .get("v")
        .cloned()
        .unwrap_or_else(|| "DMARC1".to_string());
    let policy = map
        .get("p")
        .cloned()
        .unwrap_or_else(|| "none".to_string())
        .to_lowercase();
    let subdomain_policy = map.get("sp").cloned();
    let percentage = map
        .get("pct")
        .and_then(|p| p.parse::<u8>().ok())
        .unwrap_or(100);

    let aggregate_reports = map
        .get("rua")
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let forensic_reports = map
        .get("ruf")
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    Some(DmarcPolicy {
        version,
        policy,
        subdomain_policy,
        percentage,
        aggregate_reports,
        forensic_reports,
        raw_record: clean.to_string(),
    })
}

pub fn parse_bimi_record(raw: &str) -> Option<BimiPolicy> {
    let clean = raw.trim().trim_matches('"');
    if !clean.to_lowercase().starts_with("v=bimi1") {
        return None;
    }

    let mut map: HashMap<String, String> = HashMap::new();
    for token in clean.split(';') {
        let parts: Vec<&str> = token.splitn(2, '=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].trim().to_lowercase(), parts[1].trim().to_string());
        }
    }

    let version = map.get("v").cloned().unwrap_or_else(|| "BIMI1".to_string());
    let logo_url = map.get("l").cloned().filter(|s| !s.is_empty());
    let vmc_cert_url = map.get("a").cloned().filter(|s| !s.is_empty());

    Some(BimiPolicy {
        version,
        logo_url,
        vmc_cert_url,
        raw_record: clean.to_string(),
    })
}

pub fn evaluate_dmarc_and_bimi(
    domain: &str,
    dmarc_record: Option<&str>,
    bimi_record: Option<&str>,
) -> DmarcAuditResult {
    let parsed_dmarc = dmarc_record.and_then(parse_dmarc_record);
    let parsed_bimi = bimi_record.and_then(parse_bimi_record);

    let has_dmarc = parsed_dmarc.is_some();
    let has_bimi = parsed_bimi.is_some();

    let mut score = 0u32;
    let grade;
    let mut compliant = false;
    let mut rec = String::new();

    if let Some(ref d) = parsed_dmarc {
        match d.policy.as_str() {
            "reject" => {
                score += 70;
                compliant = true;
                if d.percentage == 100 {
                    score += 10;
                }
            }
            "quarantine" => {
                score += 55;
                compliant = true;
                if d.percentage == 100 {
                    score += 5;
                }
            }
            "none" => {
                score += 30;
                compliant = false; // Google/Yahoo require p=quarantine or reject for bulk senders
                rec =
                    "DMARC policy is in monitoring mode (p=none). Spoofed emails are not blocked."
                        .to_string();
            }
            _ => {
                score += 20;
            }
        }

        if !d.aggregate_reports.is_empty() {
            score += 10;
        }

        if has_bimi {
            score += 10;
        }

        if score >= 90 {
            grade = "A+".to_string();
            rec = "Enterprise Grade: DMARC policy is strictly enforced with full reporting and BIMI branding.".to_string();
        } else if score >= 75 {
            grade = "A".to_string();
            rec = "Strong: DMARC policy is enforced (quarantine/reject) and meets 2024 ESP deliverability standards.".to_string();
        } else if score >= 50 {
            grade = "B".to_string();
            if rec.is_empty() {
                rec = "Moderate: DMARC published but percentage is below 100% or lacks complete reporting.".to_string();
            }
        } else {
            grade = "C".to_string();
        }
    } else {
        grade = "F".to_string();
        score = 0;
        compliant = false;
        rec = "Critical: No DMARC record published. Domain fails Google/Yahoo 2024 compliance and is vulnerable to direct spoofing.".to_string();
    }

    DmarcAuditResult {
        domain: domain.trim().to_lowercase(),
        has_dmarc,
        dmarc_policy: parsed_dmarc,
        has_bimi,
        bimi_policy: parsed_bimi,
        is_esp_2024_compliant: compliant,
        enforcement_grade: grade,
        security_score: score.min(100),
        recommendation: rec,
    }
}

pub async fn audit_dmarc_and_bimi(client: &Client, domain: &str) -> DmarcAuditResult {
    let clean_domain = domain.trim().to_lowercase();
    let dmarc_host = format!("_dmarc.{}", clean_domain);
    let bimi_host = format!("default._bimi.{}", clean_domain);

    let dmarc_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
        dmarc_host
    );
    let bimi_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
        bimi_host
    );

    let mut dmarc_txt: Option<String> = None;
    if let Ok(resp) = client
        .get(&dmarc_url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
                for ans in answers {
                    let rtype = ans.get("type").and_then(|t| t.as_i64()).unwrap_or(0);
                    if rtype == 16 {
                        if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                            let clean_data = data.trim_matches('"');
                            if clean_data.to_lowercase().starts_with("v=dmarc1") {
                                dmarc_txt = Some(clean_data.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut bimi_txt: Option<String> = None;
    if let Ok(resp) = client
        .get(&bimi_url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
                for ans in answers {
                    let rtype = ans.get("type").and_then(|t| t.as_i64()).unwrap_or(0);
                    if rtype == 16 {
                        if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                            let clean_data = data.trim_matches('"');
                            if clean_data.to_lowercase().starts_with("v=bimi1") {
                                bimi_txt = Some(clean_data.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    evaluate_dmarc_and_bimi(&clean_domain, dmarc_txt.as_deref(), bimi_txt.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_strict_dmarc_record() {
        let raw = "\"v=DMARC1; p=reject; pct=100; rua=mailto:dmarc@example.com; sp=reject\"";
        let pol = parse_dmarc_record(raw).unwrap();
        assert_eq!(pol.version, "DMARC1");
        assert_eq!(pol.policy, "reject");
        assert_eq!(pol.percentage, 100);
        assert_eq!(pol.aggregate_reports, vec!["mailto:dmarc@example.com"]);
        assert_eq!(pol.subdomain_policy, Some("reject".to_string()));
    }

    #[test]
    fn test_parse_monitoring_dmarc_record() {
        let raw = "v=DMARC1; p=none; rua=mailto:reports@test.org";
        let pol = parse_dmarc_record(raw).unwrap();
        assert_eq!(pol.policy, "none");
        assert_eq!(pol.percentage, 100);
    }

    #[test]
    fn test_parse_bimi_record() {
        let raw = "\"v=BIMI1; l=https://example.com/logo.svg; a=https://example.com/cert.pem\"";
        let bimi = parse_bimi_record(raw).unwrap();
        assert_eq!(bimi.version, "BIMI1");
        assert_eq!(
            bimi.logo_url,
            Some("https://example.com/logo.svg".to_string())
        );
        assert_eq!(
            bimi.vmc_cert_url,
            Some("https://example.com/cert.pem".to_string())
        );
    }

    #[test]
    fn test_evaluate_enterprise_dmarc_and_bimi() {
        let dmarc = "v=DMARC1; p=reject; pct=100; rua=mailto:rua@domain.com";
        let bimi = "v=BIMI1; l=https://domain.com/bimi.svg";
        let res = evaluate_dmarc_and_bimi("domain.com", Some(dmarc), Some(bimi));
        assert!(res.has_dmarc);
        assert!(res.has_bimi);
        assert!(res.is_esp_2024_compliant);
        assert_eq!(res.enforcement_grade, "A+");
        assert!(res.security_score >= 90);
    }

    #[test]
    fn test_evaluate_missing_dmarc() {
        let res = evaluate_dmarc_and_bimi("unprotected.org", None, None);
        assert!(!res.has_dmarc);
        assert!(!res.is_esp_2024_compliant);
        assert_eq!(res.enforcement_grade, "F");
        assert_eq!(res.security_score, 0);
    }

    #[test]
    fn test_evaluate_monitoring_mode_dmarc() {
        let dmarc = "v=DMARC1; p=none";
        let res = evaluate_dmarc_and_bimi("monitoring.com", Some(dmarc), None);
        assert!(res.has_dmarc);
        assert!(!res.is_esp_2024_compliant);
        assert_eq!(res.enforcement_grade, "C");
        assert!(res.security_score < 50);
    }
}
