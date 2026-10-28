use crate::models::{ContactRecord, LeadScore};
use crate::search::doh::MxVerifier;
use crate::search::extractor::extract_emails;
use reqwest::Client;
use serde_json::Value;

pub async fn search_hn_whoishiring(
    client: &Client,
    stack: &str,
    limit: usize,
    mx_verifier: &MxVerifier,
) -> Result<Vec<ContactRecord>, String> {
    // 1. Find latest Who is hiring story
    let story_url =
        "https://hn.algolia.com/api/v1/search_by_date?tags=story,author_whoishiring&hitsPerPage=1";
    let story_res = client
        .get(story_url)
        .header("User-Agent", "ContactFinder/1.0")
        .send()
        .await
        .map_err(|e| format!("Hacker News search failed: {}", e))?;

    let story_json: Value = story_res
        .json()
        .await
        .map_err(|e| format!("Invalid Hacker News response: {}", e))?;

    let story_id = story_json
        .get("hits")
        .and_then(|h| h.as_array())
        .and_then(|arr| arr.first())
        .and_then(|s| s.get("objectID"))
        .and_then(|id| id.as_str())
        .unwrap_or("");

    if story_id.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Fetch comments in the hiring thread
    let comments_url = format!(
        "https://hn.algolia.com/api/v1/search?tags=comment,story_{}&hitsPerPage=80",
        story_id
    );

    let comments_res = client
        .get(&comments_url)
        .header("User-Agent", "ContactFinder/1.0")
        .send()
        .await
        .map_err(|e| format!("Hacker News comment request failed: {}", e))?;

    let comments_json: Value = comments_res
        .json()
        .await
        .map_err(|e| format!("Invalid Hacker News comments: {}", e))?;

    let empty_vec = Vec::new();
    let hits = comments_json
        .get("hits")
        .and_then(|h| h.as_array())
        .unwrap_or(&empty_vec);

    let stack_lower = stack.to_lowercase();
    let mut records = Vec::new();

    for hit in hits {
        let comment_text = hit
            .get("comment_text")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if comment_text.is_empty() {
            continue;
        }

        // Check if comment matches stack (or if stack is general)
        if !stack_lower.is_empty() && !comment_text.to_lowercase().contains(&stack_lower) {
            continue;
        }

        let emails = extract_emails(comment_text);
        if emails.is_empty() {
            continue;
        }

        // Try extracting company name from first line: e.g. "Stripe | Frontend Engineer | San Francisco | REMOTE"
        let first_line = comment_text.lines().next().unwrap_or("").trim();
        let parts: Vec<&str> = first_line.split('|').collect();
        let company_name = if !parts.is_empty() && parts[0].len() < 50 {
            clean_html(parts[0])
        } else {
            "HN Hiring Company".to_string()
        };

        let role_title = if parts.len() > 1 {
            clean_html(parts[1])
        } else {
            stack.to_string()
        };

        let location = if parts.len() > 2 {
            clean_html(parts[2])
        } else {
            "Remote / International".to_string()
        };

        let object_id = hit.get("objectID").and_then(|v| v.as_str()).unwrap_or("");
        let hn_url = format!("https://news.ycombinator.com/item?id={}", object_id);

        for email in emails {
            let (mx_ok, mx_rec) = mx_verifier.check_email(client, &email).await;
            let rec = ContactRecord::new(
                &format!("Founder / {}", company_name),
                &email,
                &company_name,
                &role_title,
                &location,
                stack,
                "",
                "",
                "HN Who is Hiring",
                &hn_url,
                LeadScore::Hot,
                mx_ok,
                &mx_rec,
            );
            if !records.iter().any(|r: &ContactRecord| r.email == rec.email) {
                records.push(rec);
            }
        }

        if records.len() >= limit {
            break;
        }
    }

    Ok(records)
}

fn clean_html(input: &str) -> String {
    let no_tags = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(input, "")
        .to_string();
    no_tags
        .replace("&amp;", "&")
        .replace("&#x2F;", "/")
        .replace("&#x27;", "'")
        .trim()
        .to_string()
}
