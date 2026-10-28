use crate::models::{ContactRecord, LeadScore};
use regex::Regex;
use reqwest::Client;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct WellKnownResult {
    pub records: Vec<ContactRecord>,
    pub hiring_links: Vec<String>,
}

pub async fn probe_well_known(client: &Client, domain: &str) -> Result<WellKnownResult, String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');

    if clean_domain.is_empty() {
        return Ok(WellKnownResult::default());
    }

    let mut records = Vec::new();
    let mut hiring_links = Vec::new();

    // 1. RFC 9116 security.txt probing
    let sec_urls = [
        format!("https://{}/.well-known/security.txt", clean_domain),
        format!("https://{}/security.txt", clean_domain),
    ];

    let email_re =
        Regex::new(r#"(?i)mailto:([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+)"#).unwrap();
    let hiring_re = Regex::new(r#"(?im)^\s*Hiring:\s*(https?://\S+)"#).unwrap();

    for url in sec_urls {
        if let Ok(resp) = client
            .get(&url)
            .timeout(Duration::from_secs(4))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    for cap in email_re.captures_iter(&body) {
                        if let Some(m) = cap.get(1) {
                            let email = m.as_str().to_lowercase();
                            if !records.iter().any(|r: &ContactRecord| r.email == email) {
                                records.push(ContactRecord::new(
                                    "Security Team / Head of Security",
                                    &email,
                                    clean_domain,
                                    "Cybersecurity Operations",
                                    "Remote",
                                    "AppSec / DevSecOps",
                                    "",
                                    "",
                                    "RFC 9116 security.txt",
                                    &url,
                                    LeadScore::Hot,
                                    true,
                                    "",
                                ));
                            }
                        }
                    }

                    for cap in hiring_re.captures_iter(&body) {
                        if let Some(m) = cap.get(1) {
                            let link = m.as_str().to_string();
                            if !hiring_links.contains(&link) {
                                hiring_links.push(link);
                            }
                        }
                    }

                    if !records.is_empty() || !hiring_links.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    // 2. humans.txt probing (Creative, Design & Engineering leadership)
    let humans_url = format!("https://{}/humans.txt", clean_domain);
    let name_email_re = Regex::new(r#"(?i)(?:designer|author|developer|creator|lead|founder|art director):\s*([^<(\n]+?)(?:<([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-.]+)|\((https?://\S+)\))"#).unwrap();

    if let Ok(resp) = client
        .get(&humans_url)
        .timeout(Duration::from_secs(4))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                for cap in name_email_re.captures_iter(&body) {
                    let name = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                    let email = cap
                        .get(2)
                        .map(|m| m.as_str().trim().to_lowercase())
                        .unwrap_or_default();
                    let profile_link = cap.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                    if !name.is_empty() && (!email.is_empty() || !profile_link.is_empty()) {
                        if !records.iter().any(|r| r.name == name) {
                            records.push(ContactRecord::new(
                                name,
                                &email,
                                clean_domain,
                                "Creative / Design / Tech Leadership",
                                "Remote",
                                "Design / Web",
                                "",
                                "",
                                "humans.txt",
                                profile_link
                                    .is_empty()
                                    .then(|| humans_url.as_str())
                                    .unwrap_or(profile_link),
                                LeadScore::Warm,
                                !email.is_empty(),
                                "",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(WellKnownResult {
        records,
        hiring_links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_security_txt() {
        let text = "Contact: mailto:security@example.com\nContact: https://example.com/sec\nHiring: https://example.com/jobs/sec";
        let email_re =
            Regex::new(r#"(?i)mailto:([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+)"#).unwrap();
        let hiring_re = Regex::new(r#"(?im)^\s*Hiring:\s*(https?://\S+)"#).unwrap();

        let mut emails = Vec::new();
        for cap in email_re.captures_iter(text) {
            emails.push(cap.get(1).unwrap().as_str().to_string());
        }
        assert_eq!(emails, vec!["security@example.com"]);

        let mut hiring = Vec::new();
        for cap in hiring_re.captures_iter(text) {
            hiring.push(cap.get(1).unwrap().as_str().to_string());
        }
        assert_eq!(hiring, vec!["https://example.com/jobs/sec"]);
    }
}
