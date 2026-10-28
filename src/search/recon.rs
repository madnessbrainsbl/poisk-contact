use regex::Regex;
use std::sync::LazyLock;
use std::time::Duration;

static SECURITY_MAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Contact:\s*(?:mailto:)?([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})").unwrap()
});

static HUMANS_MAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:email|mail|contact):\s*([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})").unwrap()
});

#[derive(Debug, Clone, Default)]
pub struct ReconResult {
    pub security_contacts: Vec<String>,
    pub human_contacts: Vec<String>,
    pub subdomains: Vec<String>,
}

pub async fn run_domain_recon(client: &reqwest::Client, domain: &str) -> ReconResult {
    let clean = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim_end_matches('/')
        .to_lowercase();

    if clean.is_empty() {
        return ReconResult::default();
    }

    let mut result = ReconResult::default();

    // 1. Probe security.txt (RFC 9116)
    let sec_paths = ["/.well-known/security.txt", "/security.txt"];
    for path in sec_paths {
        let url = format!("https://{}{}", clean, path);
        if let Ok(resp) = client
            .get(&url)
            .timeout(Duration::from_secs(4))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    for cap in SECURITY_MAIL_RE.captures_iter(&text) {
                        if let Some(m) = cap.get(1) {
                            let email = m.as_str().to_lowercase();
                            if !result.security_contacts.contains(&email) {
                                result.security_contacts.push(email);
                            }
                        }
                    }
                }
            }
        }
        if !result.security_contacts.is_empty() {
            break;
        }
    }

    // 2. Probe humans.txt
    let humans_url = format!("https://{}/humans.txt", clean);
    if let Ok(resp) = client
        .get(&humans_url)
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                for cap in HUMANS_MAIL_RE.captures_iter(&text) {
                    if let Some(m) = cap.get(1) {
                        let email = m.as_str().to_lowercase();
                        if !result.human_contacts.contains(&email) {
                            result.human_contacts.push(email);
                        }
                    }
                }
            }
        }
    }

    // 3. Certificate Transparency logs via crt.sh
    let crt_url = format!("https://crt.sh/?q=%25.{}&output=json", clean);
    if let Ok(resp) = client
        .get(&crt_url)
        .timeout(Duration::from_secs(5))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) OSINT/1.0",
        )
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = json.as_array() {
                    let interesting_prefixes = [
                        "mail.", "smtp.", "pop.", "imap.", "crm.", "portal.", "staff.", "corp.",
                        "api.", "auth.", "vpn.", "dev.", "admin.",
                    ];
                    for item in arr.iter().take(100) {
                        if let Some(name) = item.get("name_value").and_then(|v| v.as_str()) {
                            for line in name.lines() {
                                let sub = line.trim().trim_start_matches("*.").to_lowercase();
                                if sub.ends_with(&clean) && sub != clean {
                                    for prefix in interesting_prefixes {
                                        if sub.starts_with(prefix)
                                            && !result.subdomains.contains(&sub)
                                        {
                                            result.subdomains.push(sub.clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

pub async fn probe_shadow_subdomain_contacts(
    client: &reqwest::Client,
    base_domain: &str,
    subdomains: &[String],
) -> Vec<crate::models::ContactRecord> {
    use crate::models::{ContactRecord, LeadScore};
    use crate::search::extractor::{extract_emails, extract_phones};

    let target_prefixes = [
        "careers.",
        "jobs.",
        "team.",
        "support.",
        "contact.",
        "contacts.",
        "hr.",
        "dev.",
        "blog.",
        "msk.",
        "spb.",
    ];

    let mut discovered_contacts = Vec::new();
    let mut seen_emails = std::collections::HashSet::new();

    for sub in subdomains {
        let is_target = target_prefixes.iter().any(|prefix| sub.starts_with(prefix));
        if !is_target {
            continue;
        }

        let probe_url = format!("https://{}", sub);
        if let Ok(resp) = client
            .get(&probe_url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let emails = extract_emails(&html);
                    let phones = extract_phones(&html);
                    let primary_phone = phones.first().cloned().unwrap_or_default();

                    for email in emails {
                        if seen_emails.insert(email.clone()) {
                            let mut contact = ContactRecord::new(
                                "",
                                &email,
                                base_domain,
                                &format!(
                                    "Отдел ({})",
                                    sub.split('.').next().unwrap_or("subdomain")
                                ),
                                "",
                                "",
                                &primary_phone,
                                "",
                                &format!("crt.sh/subdomain: {}", sub),
                                &probe_url,
                                LeadScore::Warm,
                                true,
                                "",
                            );
                            contact.tags.push("[Shadow Subdomain]".to_string());
                            discovered_contacts.push(contact);
                        }
                    }
                }
            }
        }
    }

    discovered_contacts
}
