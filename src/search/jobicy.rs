use crate::models::{ContactRecord, LeadScore};
use crate::search::crawler::crawl_website_for_contacts;
use crate::search::doh::MxVerifier;
use crate::search::extractor::extract_emails;
use reqwest::Client;
use serde_json::Value;

pub async fn search_jobicy(
    client: &Client,
    stack: &str,
    limit: usize,
    mx_verifier: &MxVerifier,
) -> Result<Vec<ContactRecord>, String> {
    let url = "https://jobicy.com/api/v2/remote-jobs?count=40";
    let res = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Jobicy request failed: {}", e))?;

    let json: Value = res
        .json()
        .await
        .map_err(|e| format!("Invalid Jobicy response: {}", e))?;

    let jobs = json
        .get("jobs")
        .and_then(|j| j.as_array())
        .ok_or("Invalid jobicy response: expected a job array")?;

    let stack_lower = stack.to_lowercase();
    let mut records = Vec::new();

    for job in jobs {
        let title = job.get("jobTitle").and_then(|v| v.as_str()).unwrap_or("");
        let company_name = job
            .get("companyName")
            .and_then(|v| v.as_str())
            .unwrap_or("Remote Company");
        let desc = job
            .get("jobDescription")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let job_url = job.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let geo = job
            .get("jobGeo")
            .and_then(|v| v.as_str())
            .unwrap_or("Remote Worldwide");

        let combined = format!("{} {}", title, desc).to_lowercase();
        if !stack_lower.is_empty() && !combined.contains(&stack_lower) {
            continue;
        }

        let emails = extract_emails(desc);
        for email in emails {
            let (mx_ok, mx_rec) = mx_verifier.check_email(client, &email).await;
            let rec = ContactRecord::new(
                &format!("HR / {}", company_name),
                &email,
                company_name,
                title,
                geo,
                stack,
                "",
                "",
                "Jobicy Remote",
                job_url,
                LeadScore::Hot,
                mx_ok,
                &mx_rec,
            );
            if !records.iter().any(|r: &ContactRecord| r.email == rec.email) {
                records.push(rec);
            }
        }

        // If no direct email in description, try crawling company URL if present
        if records.len() < limit && !job_url.is_empty() {
            if let Ok(crawled) = crawl_website_for_contacts(client, job_url, geo, stack).await {
                for mut c in crawled {
                    let (mx_ok, mx_rec) = mx_verifier.check_email(client, &c.email).await;
                    c.mx_valid = mx_ok;
                    c.mx_record = mx_rec;
                    c.source = "Jobicy / website".to_string();
                    c.score = LeadScore::Warm;
                    if !records.iter().any(|r: &ContactRecord| r.email == c.email) {
                        records.push(c);
                    }
                }
            }
        }

        if records.len() >= limit {
            break;
        }
    }

    Ok(records)
}
