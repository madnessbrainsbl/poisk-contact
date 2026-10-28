use crate::models::{ContactRecord, LeadScore};
use reqwest::Client;
use std::collections::HashSet;

pub async fn search_telegram(client: &Client, limit: usize) -> Result<Vec<ContactRecord>, String> {
    let mut records = Vec::new();
    let mut seen_keys = HashSet::new();

    let channels = vec![
        "forpython",
        "devjobs",
        "backend_jobs",
        "remotejobss",
        "rust_jobs_feed",
        "django_jobs",
    ];

    let email_re = regex::Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap();
    let tg_re = regex::Regex::new(r"@([a-zA-Z0-9_]{5,32})").unwrap();

    let bad_handles: HashSet<&str> = [
        "share",
        "joinchat",
        "telegram",
        "channel",
        "bot",
        "channel_bot",
        "admin",
        "support",
        "media",
        "info",
        "news",
        "official",
    ]
    .iter()
    .cloned()
    .collect();

    for channel in channels {
        if records.len() >= limit {
            break;
        }

        let url = format!("https://t.me/s/{}", channel);
        let resp = match client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !resp.status().is_success() {
            continue;
        }

        let html = resp.text().await.unwrap_or_default();
        let message_re =
            regex::Regex::new(r#"(?s)<div class="tgme_widget_message_text[^"]*"[^>]*>(.*?)</div>"#)
                .unwrap();

        for cap in message_re.captures_iter(&html) {
            if records.len() >= limit {
                break;
            }

            let raw_msg = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let clean = raw_msg
                .replace("<br/>", "\n")
                .replace("<br>", "\n")
                .replace("<br />", "\n");
            let clean = regex::Regex::new(r"<[^>]+>")
                .unwrap()
                .replace_all(&clean, " ")
                .to_string();
            let clean = clean.trim();

            if clean.len() < 40 {
                continue;
            }

            // Extract recruiter telegram handle
            let mut recruiter_tg = String::new();
            for tg_cap in tg_re.captures_iter(clean) {
                let handle = tg_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                if !bad_handles.contains(&handle.to_lowercase().as_str())
                    && handle.to_lowercase() != channel
                {
                    recruiter_tg = format!("@{}", handle);
                    break;
                }
            }

            // Extract email
            let mut found_email = String::new();
            for em_cap in email_re.captures_iter(clean) {
                let em = em_cap
                    .get(0)
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !em.contains("example.") && !em.contains("noreply") {
                    found_email = em;
                    break;
                }
            }

            if recruiter_tg.is_empty() && found_email.is_empty() {
                continue;
            }

            let key = if !found_email.is_empty() {
                found_email.clone()
            } else {
                recruiter_tg.clone()
            };

            if seen_keys.contains(&key) {
                continue;
            }
            seen_keys.insert(key);

            let first_line = clean.lines().next().unwrap_or("Job opportunity");
            let title: String = first_line.chars().take(60).collect();
            let title = title.trim_matches(&[' ', '#', '*', '•', '-'][..]);

            let name = if !recruiter_tg.is_empty() {
                format!("Recruiter ({})", recruiter_tg)
            } else {
                "HR / Recruiter".to_string()
            };

            let rec = ContactRecord::new(
                &name,
                &found_email,
                &format!("Telegram (@{})", channel),
                &format!("HR / Recruiter ({})", title),
                "Remote",
                title,
                "",
                &recruiter_tg,
                &format!("Telegram (@{})", channel),
                &url,
                LeadScore::Hot,
                false,
                "",
            );

            records.push(rec);
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Explicit live Telegram audit; requires public network access"]
    async fn test_telegram_search() -> Result<(), String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let res = search_telegram(&client, 5).await;
        match res {
            Ok(records) if !records.is_empty() => {
                println!("LIVE Telegram: {} records", records.len());
                Ok(())
            }
            Ok(_) => Err("Telegram returned no public records".into()),
            Err(error) => {
                eprintln!("Telegram unavailable: {error}");
                Err(error)
            }
        }
    }
}
