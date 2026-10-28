use crate::models::{ContactRecord, LeadScore};
use reqwest::Client;
use std::time::Duration;

pub async fn mine_pgp_keyserver(
    client: &Client,
    domain: &str,
    limit: usize,
) -> Result<Vec<ContactRecord>, String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();

    if clean_domain.is_empty() {
        return Ok(Vec::new());
    }

    let url = format!(
        "https://keyserver.ubuntu.com/pks/lookup?search={}&op=index&fingerprint=on&options=mr",
        clean_domain
    );

    let resp = match client
        .get(&url)
        .timeout(Duration::from_secs(6))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(Vec::new()),
    };

    let body = match resp.text().await {
        Ok(b) => b,
        Err(_) => return Ok(Vec::new()),
    };

    let mut records = Vec::new();

    // Machine-readable HKP response format:
    // uid:Name <email>:creation_date:expiration_date:flags
    for line in body.lines() {
        if records.len() >= limit {
            break;
        }

        if line.starts_with("uid:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let uid_field = parts[1];
                if let Some(start_email) = uid_field.find('<') {
                    if let Some(end_email) = uid_field.find('>') {
                        if start_email < end_email {
                            let raw_name = &uid_field[..start_email].trim();
                            let email = uid_field[start_email + 1..end_email].trim().to_lowercase();

                            if email.ends_with(&format!("@{}", clean_domain)) {
                                if !records.iter().any(|r: &ContactRecord| r.email == email) {
                                    let name = if raw_name.is_empty() {
                                        "Employee / Key Holder"
                                    } else {
                                        raw_name
                                    };

                                    records.push(ContactRecord::new(
                                        name,
                                        &email,
                                        &clean_domain,
                                        "Engineering / PGP Key Holder",
                                        "Global",
                                        "Engineering / Security",
                                        "",
                                        "",
                                        "Ubuntu PGP Keyserver (HKP)",
                                        &url,
                                        LeadScore::Warm,
                                        true,
                                        "",
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_hkp_line() {
        let sample = "uid:John Doe <john.doe@example.com>:1600000000::";
        let parts: Vec<&str> = sample.split(':').collect();
        assert_eq!(parts[0], "uid");
        let uid = parts[1];
        let start = uid.find('<').unwrap();
        let end = uid.find('>').unwrap();
        let name = &uid[..start].trim();
        let email = &uid[start + 1..end].trim();
        assert_eq!(*name, "John Doe");
        assert_eq!(*email, "john.doe@example.com");
    }
}
