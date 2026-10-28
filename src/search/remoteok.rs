use crate::models::{ContactRecord, LeadScore};
use crate::search::doh::MxVerifier;
use crate::search::extractor::extract_emails;
use reqwest::Client;
use serde_json::Value;

pub async fn search_remoteok(
    client: &Client,
    stack: &str,
    limit: usize,
    mx_verifier: &MxVerifier,
) -> Result<Vec<ContactRecord>, String> {
    let url = "https://remoteok.com/api";
    let res = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Accept", "application/json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("RemoteOK API error: {}", e))?;

    let json: Value = res
        .json()
        .await
        .map_err(|e| format!("RemoteOK JSON parse error: {}", e))?;

    let jobs = json
        .as_array()
        .ok_or("Invalid remoteok response: expected a job array")?;

    let stack_lower = stack.to_lowercase();
    let mut records = Vec::new();

    for job in jobs {
        if records.len() >= limit {
            break;
        }

        // First object in RemoteOK is disclaimer/legal, skip if no id or position
        let position = job.get("position").and_then(|v| v.as_str());
        if position.is_none() {
            continue;
        }
        let title = position.unwrap();
        let company_name = job
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("Remote Startup");
        let desc = job
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let job_url = job.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let location = job
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("Worldwide Remote");

        let combined = format!("{} {}", title, desc).to_lowercase();
        if !stack_lower.is_empty() && !stack_lower.split('/').any(|s| combined.contains(s.trim())) {
            let tokens: Vec<&str> = stack_lower.split_whitespace().collect();
            if !tokens.is_empty() && !tokens.iter().any(|t| combined.contains(t)) {
                continue;
            }
        }

        let emails = extract_emails(desc);
        let mut email_found = String::new();
        for em in emails {
            if !em.contains("example.") && !em.contains("noreply") {
                email_found = em;
                break;
            }
        }

        let (mx_ok, mx_rec) = if !email_found.is_empty() {
            mx_verifier.check_email(client, &email_found).await
        } else {
            (false, "".to_string())
        };

        let rec = ContactRecord::new(
            &format!("Talent Team / {}", company_name),
            &email_found,
            company_name,
            title,
            location,
            stack,
            "",
            "",
            "RemoteOK Global",
            job_url,
            LeadScore::Hot,
            mx_ok,
            &mx_rec,
        );

        records.push(rec);
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Explicit live API audit; requires network access"]
    async fn test_remoteok_search() -> Result<(), String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let mx = MxVerifier::new();
        let res = search_remoteok(&client, "engineer", 5, &mx).await;
        match res {
            Ok(records) => {
                if records.is_empty() || records.len() > 5 {
                    return Err(format!(
                        "remoteok: expected 1-5 records, got {}",
                        records.len()
                    ));
                }
                if records
                    .iter()
                    .any(|record| record.company.is_empty() || !record.url.starts_with("https://"))
                {
                    return Err("remoteok: invalid public job records".into());
                }
                println!("LIVE REMOTEOK: {} public jobs", records.len());
                Ok(())
            }
            Err(error) => {
                eprintln!("remoteok live audit unavailable: {error}");
                Err(error)
            }
        }
    }
}
