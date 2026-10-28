use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpfLookupAudit {
    pub domain_or_raw: String,
    pub raw_record: String,
    pub lookup_count: usize,
    pub max_allowed: usize,
    pub lookup_mechanisms: Vec<String>,
    pub ip_mechanisms: Vec<String>,
    pub compliance_status: String,
    pub is_permerror: bool,
    pub recommendation: String,
}

pub fn parse_and_audit_spf(input: &str) -> SpfLookupAudit {
    let clean = input.trim();
    let tokens: Vec<&str> = clean.split_whitespace().collect();

    let mut lookup_mechanisms = Vec::new();
    let mut ip_mechanisms = Vec::new();

    for token in tokens {
        let lower = token.to_lowercase();
        if lower == "v=spf1" || lower.ends_with("all") {
            continue;
        }

        if lower.starts_with("ip4:") || lower.starts_with("ip6:") {
            ip_mechanisms.push(token.to_string());
        } else if lower.starts_with("include:")
            || lower.starts_with("redirect=")
            || lower == "a"
            || lower.starts_with("a:")
            || lower == "mx"
            || lower.starts_with("mx:")
            || lower == "ptr"
            || lower.starts_with("ptr:")
            || lower.starts_with("exists:")
        {
            lookup_mechanisms.push(token.to_string());
        }
    }

    let lookup_count = lookup_mechanisms.len();
    let max_allowed = 10;
    let is_permerror = lookup_count > max_allowed;

    let (compliance_status, recommendation) = if is_permerror {
        (
            "CRITICAL: RFC 7208 PermError (Limit Exceeded)".to_string(),
            format!(
                "Fatal SPF PermError: Record consumes {} DNS lookups (RFC 7208 ceiling is 10). Receiving MTAs (Google Workspace, Microsoft 365) will fail SPF/DMARC alignment. Flatten vendor includes into direct ip4/ip6 CIDR blocks immediately.",
                lookup_count
            ),
        )
    } else if lookup_count == 10 {
        (
            "WARNING: At Capacity (10/10 Lookups)".to_string(),
            "Record is at exact RFC capacity. Any nested include or additional vendor service will trigger PermError and send emails to Spam. Consider pruning obsolete providers.".to_string(),
        )
    } else if lookup_count >= 7 {
        (
            "WARNING: Elevated Lookups (Near Capacity)".to_string(),
            format!(
                "Record consumes {} of 10 permitted lookups. Safe for current delivery, but approaches RFC limit. Audit active tools before adding new marketing or CRM services.",
                lookup_count
            ),
        )
    } else {
        (
            "OPTIMAL: Compliant with RFC 7208".to_string(),
            format!(
                "Healthy SPF structure with {} lookup(s) (safe margin of {} lookups remaining). High inbox placement reliability.",
                lookup_count,
                10 - lookup_count
            ),
        )
    };

    SpfLookupAudit {
        domain_or_raw: clean.to_string(),
        raw_record: clean.to_string(),
        lookup_count,
        max_allowed,
        lookup_mechanisms,
        ip_mechanisms,
        compliance_status,
        is_permerror,
        recommendation,
    }
}

pub async fn audit_domain_spf_lookups(
    client: &Client,
    domain_or_spf: &str,
) -> Result<SpfLookupAudit, String> {
    let clean = domain_or_spf.trim();

    if clean.starts_with("v=spf1") {
        return Ok(parse_and_audit_spf(clean));
    }

    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
        clean
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_matches('/')
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/dns-json")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("DoH query failed: {}", e))?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
        for ans in answers {
            if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                let unquoted = data.trim_matches('"');
                if unquoted.starts_with("v=spf1") {
                    let mut audit = parse_and_audit_spf(unquoted);
                    audit.domain_or_raw = clean.to_string();
                    return Ok(audit);
                }
            }
        }
    }

    Err(format!(
        "No valid SPF (v=spf1) record found for domain: {}",
        clean
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_compliant_spf() {
        let spf = "v=spf1 include:_spf.google.com ip4:192.168.1.1 ~all";
        let audit = parse_and_audit_spf(spf);
        assert_eq!(audit.lookup_count, 1);
        assert!(!audit.is_permerror);
        assert_eq!(audit.lookup_mechanisms, vec!["include:_spf.google.com"]);
        assert_eq!(audit.ip_mechanisms, vec!["ip4:192.168.1.1"]);
        assert!(audit.compliance_status.contains("OPTIMAL"));
    }

    #[test]
    fn test_permerror_exceeded_spf() {
        let spf = "v=spf1 include:s1.com include:s2.com include:s3.com include:s4.com include:s5.com include:s6.com a mx include:s7.com include:s8.com include:s9.com -all";
        let audit = parse_and_audit_spf(spf);
        assert_eq!(audit.lookup_count, 11);
        assert!(audit.is_permerror);
        assert!(audit.compliance_status.contains("PermError"));
    }

    #[test]
    fn test_near_limit_spf() {
        let spf = "v=spf1 include:s1.com include:s2.com include:s3.com include:s4.com a mx include:s5.com include:s6.com ip4:10.0.0.1 ~all";
        let audit = parse_and_audit_spf(spf);
        assert_eq!(audit.lookup_count, 8);
        assert!(!audit.is_permerror);
        assert!(audit.compliance_status.contains("Elevated Lookups"));
    }
}
