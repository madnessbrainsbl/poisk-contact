use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcrdnsValidationReport {
    pub ip: String,
    pub arpa_domain: String,
    pub ptr_hostname: Option<String>,
    pub forward_ips: Vec<String>,
    pub is_fcrdns_valid: bool,
    pub is_residential_dynamic: bool,
    pub reputation_grade: String,
    pub deliverability_verdict: String,
}

pub fn ip_to_arpa(ip_str: &str) -> Option<String> {
    let clean = ip_str.trim();
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        Some(format!(
            "{}.{}.{}.{}.in-addr.arpa",
            parts[3], parts[2], parts[1], parts[0]
        ))
    } else {
        None
    }
}

const RESIDENTIAL_INDICATORS: &[&str] = &[
    "dynamic",
    "dhcp",
    "pool",
    "broadband",
    "cable",
    "dsl",
    "adsl",
    "dialup",
    "user",
    "client",
    "node",
    "ppp",
    "home",
    "subscriber",
    "cust",
    "res",
];

pub fn evaluate_fcrdns_offline(
    ip: &str,
    ptr_hostname: Option<&str>,
    forward_ips: &[String],
) -> FcrdnsValidationReport {
    let clean_ip = ip.trim().to_string();
    let arpa = ip_to_arpa(&clean_ip).unwrap_or_default();

    let ptr_opt = ptr_hostname.map(|s| s.trim().trim_end_matches('.').to_lowercase());

    let (is_fcrdns_valid, is_residential_dynamic) = match &ptr_opt {
        Some(host) => {
            let valid = forward_ips.iter().any(|fip| fip.trim() == clean_ip);
            let residential = RESIDENTIAL_INDICATORS.iter().any(|&ind| host.contains(ind));
            (valid, residential)
        }
        None => (false, false),
    };

    let (reputation_grade, verdict) = if ptr_opt.is_none() {
        (
            "CRITICAL: Missing PTR Record (No Reverse DNS)".to_string(),
            "IP lacks reverse DNS PTR configuration. Enterprise mail receivers (Google, Microsoft, Yahoo) immediately reject unmapped IP connections.".to_string(),
        )
    } else if is_residential_dynamic {
        (
            "HIGH RISK: Residential / Dynamic DHCP Pool (Spamhaus PBL)".to_string(),
            "PTR hostname matches dynamic consumer ISP naming patterns. Listed on Spamhaus PBL (Policy Block List); blocked by default across all B2B mail servers.".to_string(),
        )
    } else if is_fcrdns_valid {
        (
            "OPTIMAL: Verified Dedicated Enterprise MTA (RFC 8601)".to_string(),
            "Forward-Confirmed Reverse DNS (FCrDNS) full round-trip verified. Authoritative, dedicated mail server reputation suitable for cold outreach.".to_string(),
        )
    } else {
        (
            "FAILED: Broken FCrDNS (PTR & Forward A Mismatch)".to_string(),
            "PTR hostname exists but its Forward DNS A records do not map back to this IP address. Fails RFC 8601 anti-spoofing verification.".to_string(),
        )
    };

    FcrdnsValidationReport {
        ip: clean_ip,
        arpa_domain: arpa,
        ptr_hostname: ptr_opt,
        forward_ips: forward_ips.to_vec(),
        is_fcrdns_valid,
        is_residential_dynamic,
        reputation_grade,
        deliverability_verdict: verdict,
    }
}

pub async fn validate_ip_fcrdns(client: &Client, ip_str: &str) -> FcrdnsValidationReport {
    let clean_ip = ip_str.trim();
    let arpa = match ip_to_arpa(clean_ip) {
        Some(a) => a,
        None => return evaluate_fcrdns_offline(clean_ip, None, &[]),
    };

    // 1. Resolve PTR via Cloudflare DoH
    let ptr_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=PTR",
        arpa
    );
    let mut resolved_hostname: Option<String> = None;

    if let Ok(resp) = client
        .get(&ptr_url)
        .header("Accept", "application/dns-json")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
                for ans in answers {
                    if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                        resolved_hostname =
                            Some(data.trim_matches('"').trim_end_matches('.').to_string());
                        break;
                    }
                }
            }
        }
    }

    let mut forward_ips = Vec::new();

    // 2. Resolve Forward A records for the PTR hostname
    if let Some(host) = &resolved_hostname {
        let a_url = format!("https://cloudflare-dns.com/dns-query?name={}&type=A", host);
        if let Ok(resp) = client
            .get(&a_url)
            .header("Accept", "application/dns-json")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
                    for ans in answers {
                        if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                            forward_ips.push(data.trim_matches('"').to_string());
                        }
                    }
                }
            }
        }
    }

    evaluate_fcrdns_offline(clean_ip, resolved_hostname.as_deref(), &forward_ips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verified_enterprise_fcrdns() {
        let report = evaluate_fcrdns_offline(
            "77.88.21.249",
            Some("forward.yandex.net"),
            &["77.88.21.249".to_string(), "77.88.21.250".to_string()],
        );
        assert!(report.is_fcrdns_valid);
        assert!(!report.is_residential_dynamic);
        assert!(report.reputation_grade.contains("OPTIMAL"));
        assert_eq!(report.ptr_hostname.as_deref(), Some("forward.yandex.net"));
    }

    #[test]
    fn test_residential_dynamic_fcrdns() {
        let report = evaluate_fcrdns_offline(
            "1.2.3.4",
            Some("dsl-pool-123.dynamic.isp.com"),
            &["1.2.3.4".to_string()],
        );
        assert!(report.is_residential_dynamic);
        assert!(
            report
                .reputation_grade
                .contains("Residential / Dynamic DHCP Pool")
        );
    }

    #[test]
    fn test_mismatched_forward_ips() {
        let report = evaluate_fcrdns_offline(
            "10.20.30.40",
            Some("mail.legitcorp.com"),
            &["192.168.1.1".to_string()],
        );
        assert!(!report.is_fcrdns_valid);
        assert!(report.reputation_grade.contains("Broken FCrDNS"));
    }
}
