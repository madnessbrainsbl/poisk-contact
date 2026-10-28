//! Read-only vacancy leads for ApplyPilot; no contact storage or mail operations.
use poisk_contact::search::{linkedin::search_linkedin_jobs, web::search_web};
use serde_json::{Value, json};
use std::time::Duration;

fn vacancy(record: poisk_contact::models::ContactRecord) -> Value {
    json!({"title": record.role.strip_prefix("Hiring Team / ").unwrap_or(&record.role),
           "company": record.company, "location": record.city, "url": record.url,
           "description": "", "source": "PoiskContact LinkedIn Public"})
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut queries = Vec::new();
    let mut limit = 20usize;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--query" if index + 1 < args.len() => queries.push(args[index + 1].clone()),
            "--limit" if index + 1 < args.len() => limit = args[index + 1].parse()?,
            _ => return Err("Expected --query TEXT and --limit 1..100".into()),
        }
        index += 2;
    }
    if queries.is_empty() || queries.len() > 8 || !(1..=100).contains(&limit) {
        return Err("Require 1..8 queries and limit 1..100".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut jobs = Vec::new();
    let mut links = Vec::new();
    let mut statuses = Vec::new();
    for query in &queries {
        match search_web(&client, query, limit).await {
            Ok(hits) => {
                statuses.push(json!({"source":"PoiskContact web", "state":if hits.is_empty(){"empty"}else{"ok"}, "count":hits.len()}));
                links.extend(
                    hits.into_iter()
                        .map(|hit| json!({"url":hit.url, "source":"PoiskContact web"})),
                );
            }
            Err(error) => {
                statuses.push(json!({"source":"PoiskContact web", "state":"error", "error":error}))
            }
        }
    }
    for query in [
        "healthcare application security",
        "medical device cybersecurity",
    ] {
        match search_linkedin_jobs(&client, "Worldwide", query, "jobs", limit).await {
            Ok(result) => {
                statuses.push(json!({"source":"PoiskContact LinkedIn Public", "state":if result.records.is_empty(){"empty"}else{"ok"}, "count":result.records.len()}));
                jobs.extend(result.records.into_iter().map(vacancy));
            }
            Err(error) => statuses.push(
                json!({"source":"PoiskContact LinkedIn Public", "state":"error", "error":error}),
            ),
        }
    }
    println!(
        "{}",
        json!({"jobs":jobs,"links":links,"statuses":statuses,"observed_at":chrono::Utc::now().to_rfc3339()})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_vacancy_fields_without_contact_identity_or_sending_state() {
        let record = poisk_contact::models::ContactRecord {
            role: "Hiring Team / Security Engineer".into(),
            company: "HealthTech".into(),
            city: "Berlin".into(),
            url: "https://www.linkedin.com/jobs/view/123".into(),
            email: "inferred@example.test".into(),
            phone: "+123456".into(),
            ..Default::default()
        };
        let value = vacancy(record);
        assert_eq!(value["title"], "Security Engineer");
        assert!(
            value.get("email").is_none()
                && value.get("phone").is_none()
                && value.get("status").is_none()
        );
    }
}
