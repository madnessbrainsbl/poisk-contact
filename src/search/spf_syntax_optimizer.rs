use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpfOptimizationReport {
    pub raw_spf: String,
    pub is_valid: bool,
    pub hygiene_score: u8,
    pub has_dangerous_plus_all: bool,
    pub has_deprecated_ptr: bool,
    pub has_misplaced_all: bool,
    pub duplicate_mechanisms_count: usize,
    pub issues_found: Vec<String>,
    pub optimized_spf: String,
    pub recommendations: Vec<String>,
}

pub fn optimize_spf_record_offline(raw: &str) -> SpfOptimizationReport {
    let clean = raw.trim().trim_matches('"');
    if !clean.starts_with("v=spf1") {
        return SpfOptimizationReport {
            raw_spf: clean.to_string(),
            is_valid: false,
            hygiene_score: 0,
            has_dangerous_plus_all: false,
            has_deprecated_ptr: false,
            has_misplaced_all: false,
            duplicate_mechanisms_count: 0,
            issues_found: vec![
                "Input does not start with standard 'v=spf1' identifier.".to_string(),
            ],
            optimized_spf: "".to_string(),
            recommendations: vec![
                "Generate a valid SPF record starting with 'v=spf1'.".to_string(),
            ],
        };
    }

    let tokens: Vec<&str> = clean.split_whitespace().collect();
    let mut score = 100u8;
    let mut issues = Vec::new();
    let mut recs = Vec::new();

    let mut has_plus_all = false;
    let mut has_ptr = false;
    let mut has_misplaced_all = false;

    let mut all_token: Option<&str> = None;
    let mut body_tokens: Vec<&str> = Vec::new();

    for (idx, &t) in tokens.iter().enumerate() {
        if idx == 0 {
            continue; // v=spf1
        }

        let lower = t.to_lowercase();
        if lower == "+all" {
            has_plus_all = true;
            all_token = Some("+all");
            score = score.saturating_sub(60);
            issues.push(
                "CRITICAL: '+all' mechanism authorizes entire internet to send as your domain."
                    .to_string(),
            );
            recs.push(
                "Replace '+all' with soft-fail '~all' or strict hard-fail '-all' immediately."
                    .to_string(),
            );
        } else if lower == "~all" || lower == "-all" || lower == "?all" {
            all_token = Some(t);
            if idx != tokens.len() - 1 {
                has_misplaced_all = true;
                score = score.saturating_sub(25);
                issues.push(format!("Misplaced '{}' terminator in middle of record. Subsequent mechanisms are ignored by MTAs.", t));
                recs.push(
                    "Move the 'all' terminator strictly to the very end of the SPF string."
                        .to_string(),
                );
            }
        } else if lower.starts_with("ptr") {
            has_ptr = true;
            score = score.saturating_sub(20);
            issues.push("Deprecated 'ptr' mechanism found (RFC 7208 Section 5.5). Increases DNS query load.".to_string());
            recs.push(
                "Remove 'ptr' mechanism and replace with direct 'ip4:' or vendor 'include:'."
                    .to_string(),
            );
        } else {
            body_tokens.push(t);
        }
    }

    if all_token.is_none() {
        score = score.saturating_sub(20);
        issues.push("Missing default 'all' terminator (~all or -all).".to_string());
        recs.push("Append '~all' or '-all' at the end of the SPF record.".to_string());
    }

    // Deduplicate body mechanisms
    let initial_count = body_tokens.len();
    let mut unique_tokens = Vec::new();
    for &t in &body_tokens {
        if !unique_tokens.contains(&t) {
            unique_tokens.push(t);
        }
    }
    let duplicate_count = initial_count - unique_tokens.len();
    if duplicate_count > 0 {
        score = score.saturating_sub(10);
        issues.push(format!(
            "Detected {} duplicate mechanism(s).",
            duplicate_count
        ));
        recs.push(
            "Eliminate redundant duplicate IP/include blocks to reduce packet byte size."
                .to_string(),
        );
    }

    // Reconstruct optimized SPF
    let final_terminator = match all_token {
        Some("+all") => "~all", // Fix dangerous +all automatically
        Some(term) => term,
        None => "~all",
    };

    let mut optimized_parts = vec!["v=spf1".to_string()];
    for t in unique_tokens {
        if !t.to_lowercase().starts_with("ptr") {
            optimized_parts.push(t.to_string());
        }
    }
    optimized_parts.push(final_terminator.to_string());
    let optimized_spf = optimized_parts.join(" ");

    SpfOptimizationReport {
        raw_spf: clean.to_string(),
        is_valid: !has_plus_all && all_token.is_some(),
        hygiene_score: score,
        has_dangerous_plus_all: has_plus_all,
        has_deprecated_ptr: has_ptr,
        has_misplaced_all,
        duplicate_mechanisms_count: duplicate_count,
        issues_found: issues,
        optimized_spf,
        recommendations: recs,
    }
}

pub async fn audit_and_optimize_spf(client: &Client, domain_or_raw: &str) -> SpfOptimizationReport {
    let clean = domain_or_raw.trim();
    if clean.starts_with("v=spf1") {
        return optimize_spf_record_offline(clean);
    }

    // Resolve domain TXT via DoH
    let domain = clean
        .to_lowercase()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_matches('/')
        .to_string();

    let txt_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
        domain
    );

    let mut found_spf: Option<String> = None;

    if let Ok(resp) = client
        .get(&txt_url)
        .header("Accept", "application/dns-json")
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json["Answer"].as_array() {
                for ans in answers {
                    if ans["type"].as_u64() == Some(16) {
                        if let Some(data) = ans["data"].as_str() {
                            let unquoted = data.trim().trim_matches('"');
                            if unquoted.starts_with("v=spf1") {
                                found_spf = Some(unquoted.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    match found_spf {
        Some(s) => optimize_spf_record_offline(&s),
        None => SpfOptimizationReport {
            raw_spf: format!("No SPF TXT record discovered for {}", domain),
            is_valid: false,
            hygiene_score: 0,
            has_dangerous_plus_all: false,
            has_deprecated_ptr: false,
            has_misplaced_all: false,
            duplicate_mechanisms_count: 0,
            issues_found: vec![format!(
                "Domain {} lacks a registered SPF (v=spf1) record.",
                domain
            )],
            optimized_spf: "v=spf1 mx ~all".to_string(),
            recommendations: vec![
                "Publish a DNS TXT record with 'v=spf1 mx ~all' to prevent spoofing.".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dangerous_plus_all() {
        let rep = optimize_spf_record_offline("v=spf1 ip4:192.168.1.1 +all");
        assert!(rep.has_dangerous_plus_all);
        assert!(!rep.is_valid);
        assert!(rep.hygiene_score < 50);
        assert_eq!(rep.optimized_spf, "v=spf1 ip4:192.168.1.1 ~all");
    }

    #[test]
    fn test_detect_duplicate_and_misplaced_all() {
        let rep = optimize_spf_record_offline("v=spf1 mx ~all ip4:10.0.0.1 mx");
        assert!(rep.has_misplaced_all);
        assert_eq!(rep.duplicate_mechanisms_count, 1);
        assert_eq!(rep.optimized_spf, "v=spf1 mx ip4:10.0.0.1 ~all");
    }

    #[test]
    fn test_clean_optimized_spf() {
        let rep = optimize_spf_record_offline("v=spf1 include:_spf.google.com ip4:1.2.3.4 ~all");
        assert!(rep.is_valid);
        assert_eq!(rep.hygiene_score, 100);
        assert!(rep.issues_found.is_empty());
        assert_eq!(
            rep.optimized_spf,
            "v=spf1 include:_spf.google.com ip4:1.2.3.4 ~all"
        );
    }
}
