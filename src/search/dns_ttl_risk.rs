use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsTtlAnalysis {
    pub domain: String,
    pub min_ttl_seconds: u32,
    pub max_ttl_seconds: u32,
    pub average_ttl_seconds: u32,
    pub is_ephemeral_risk: bool,
    pub risk_level: String,
    pub stability_verdict: String,
    pub records_analyzed: Vec<String>,
}

pub fn evaluate_ttl_records(domain: &str, ttls: Vec<(String, u32)>) -> DnsTtlAnalysis {
    if ttls.is_empty() {
        return DnsTtlAnalysis {
            domain: domain.to_string(),
            min_ttl_seconds: 0,
            max_ttl_seconds: 0,
            average_ttl_seconds: 0,
            is_ephemeral_risk: true,
            risk_level: "High: No DNS Records Published".to_string(),
            stability_verdict: "Domain has no valid MX/NS records resolved via DNS.".to_string(),
            records_analyzed: Vec::new(),
        };
    }

    let min_ttl = ttls.iter().map(|(_, t)| *t).min().unwrap_or(0);
    let max_ttl = ttls.iter().map(|(_, t)| *t).max().unwrap_or(0);
    let sum_ttl: u64 = ttls.iter().map(|(_, t)| *t as u64).sum();
    let avg_ttl = (sum_ttl / ttls.len() as u64) as u32;

    let records_analyzed: Vec<String> = ttls
        .iter()
        .map(|(rec, ttl)| format!("{} (TTL: {}s / ~{:.1}m)", rec, ttl, *ttl as f64 / 60.0))
        .collect();

    let (is_ephemeral_risk, risk_level, stability_verdict) = if min_ttl < 300 {
        (
            true,
            "High: Ephemeral / Fast-Flux Risk (<300s TTL)".to_string(),
            "Ultra-short TTL detected. Domains with sub-5-minute TTLs exhibit high correlation with bulletproof disposable hosting, dynamic spambot infrastructure, and frequent DNS churn.".to_string(),
        )
    } else if min_ttl < 1800 {
        (
            false,
            "Moderate: Dynamic Cloud Hosting (300s - 1800s TTL)".to_string(),
            "Standard dynamic DNS TTL typical of modern cloud infrastructure (AWS Route 53, Cloudflare, GCP).".to_string(),
        )
    } else {
        (
            false,
            "Low: Enterprise Long-Term Stable (>=1800s TTL)".to_string(),
            "Long-lived DNS caching indicates stable, reputable corporate B2B mail infrastructure with minimal spoofing churn.".to_string(),
        )
    };

    DnsTtlAnalysis {
        domain: domain.to_string(),
        min_ttl_seconds: min_ttl,
        max_ttl_seconds: max_ttl,
        average_ttl_seconds: avg_ttl,
        is_ephemeral_risk,
        risk_level,
        stability_verdict,
        records_analyzed,
    }
}

pub async fn audit_dns_ttl_risk(client: &Client, domain: &str) -> Result<DnsTtlAnalysis, String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_matches('/')
        .to_lowercase();
    let mut collected_ttls = Vec::new();

    // 1. Query MX records
    let mx_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=MX",
        clean_domain
    );
    if let Ok(resp) = client
        .get(&mx_url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json.get("Answer").and_then(|v| v.as_array()) {
                for ans in answers {
                    let ttl = ans.get("TTL").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let data = ans.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    if ttl > 0 && !data.is_empty() {
                        collected_ttls.push((format!("MX: {}", data.trim()), ttl));
                    }
                }
            }
        }
    }

    // 2. Query NS records
    let ns_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=NS",
        clean_domain
    );
    if let Ok(resp) = client
        .get(&ns_url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json.get("Answer").and_then(|v| v.as_array()) {
                for ans in answers {
                    let ttl = ans.get("TTL").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let data = ans.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    if ttl > 0 && !data.is_empty() {
                        collected_ttls.push((format!("NS: {}", data.trim()), ttl));
                    }
                }
            }
        }
    }

    Ok(evaluate_ttl_records(&clean_domain, collected_ttls))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_fast_flux_ttl() {
        let ttls = vec![
            ("MX: 10 mail.throwaway.biz".to_string(), 60),
            ("NS: ns1.throwaway.biz".to_string(), 120),
        ];
        let analysis = evaluate_ttl_records("throwaway.biz", ttls);
        assert!(analysis.is_ephemeral_risk);
        assert_eq!(analysis.min_ttl_seconds, 60);
        assert!(analysis.risk_level.contains("Fast-Flux"));
    }

    #[test]
    fn test_enterprise_stable_ttl() {
        let ttls = vec![
            ("MX: 10 aspmx.l.google.com".to_string(), 3600),
            ("NS: ns1.google.com".to_string(), 86400),
        ];
        let analysis = evaluate_ttl_records("google.com", ttls);
        assert!(!analysis.is_ephemeral_risk);
        assert_eq!(analysis.min_ttl_seconds, 3600);
        assert!(analysis.risk_level.contains("Enterprise Long-Term Stable"));
    }
}
