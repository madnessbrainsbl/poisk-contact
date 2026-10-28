use crate::models::{ContactRecord, LeadScore};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct GithubProfile {
    pub login: String,
    pub html_url: String,
    pub avatar_url: String,
    pub bio: String,
    pub name: String,
    pub company: String,
    pub blog: String,
}

pub async fn probe_github_user_by_email(client: &Client, email: &str) -> Option<GithubProfile> {
    let clean_email = email.trim().to_lowercase();
    if clean_email.is_empty() || !clean_email.contains('@') {
        return None;
    }

    // Skip common generic / disposable mailboxes
    if clean_email.starts_with("info@")
        || clean_email.starts_with("contact@")
        || clean_email.starts_with("support@")
        || clean_email.starts_with("admin@")
        || clean_email.starts_with("sales@")
    {
        return None;
    }

    let search_url = format!(
        "https://api.github.com/search/users?q={}+in:email",
        urlencoding_simple(&clean_email)
    );

    let resp = match client
        .get(&search_url)
        .header(
            "User-Agent",
            "PoiskContact-OSINT/1.0 (B2B Lead Intelligence)",
        )
        .header("Accept", "application/vnd.github.v3+json")
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    if !resp.status().is_success() {
        return None;
    }

    let val: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return None,
    };

    let total_count = val.get("total_count").and_then(|t| t.as_u64()).unwrap_or(0);
    if total_count == 0 {
        return None;
    }

    let items = val.get("items").and_then(|i| i.as_array())?;
    let first = items.first()?;

    let login = first
        .get("login")
        .and_then(|l| l.as_str())
        .unwrap_or("")
        .to_string();
    if login.is_empty() {
        return None;
    }

    let html_url = first
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();
    let avatar_url = first
        .get("avatar_url")
        .and_then(|a| a.as_str())
        .unwrap_or("")
        .to_string();

    // Optionally probe user profile endpoint for bio & name
    let user_url = format!("https://api.github.com/users/{}", login);
    let mut bio = String::new();
    let mut full_name = String::new();
    let mut company = String::new();
    let mut blog = String::new();

    if let Ok(profile_resp) = client
        .get(&user_url)
        .header(
            "User-Agent",
            "PoiskContact-OSINT/1.0 (B2B Lead Intelligence)",
        )
        .header("Accept", "application/vnd.github.v3+json")
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        if profile_resp.status().is_success() {
            if let Ok(u_val) = profile_resp.json::<Value>().await {
                if let Some(b) = u_val.get("bio").and_then(|v| v.as_str()) {
                    bio = b.trim().chars().take(200).collect();
                }
                if let Some(n) = u_val.get("name").and_then(|v| v.as_str()) {
                    full_name = n.trim().chars().take(100).collect();
                }
                if let Some(c) = u_val.get("company").and_then(|v| v.as_str()) {
                    company = c.trim().chars().take(100).collect();
                }
                if let Some(bl) = u_val.get("blog").and_then(|v| v.as_str()) {
                    blog = bl.trim().to_string();
                }
            }
        }
    }

    Some(GithubProfile {
        login,
        html_url,
        avatar_url,
        bio,
        name: full_name,
        company,
        blog,
    })
}

pub async fn enrich_contact_with_github(client: &Client, contact: &mut ContactRecord) -> bool {
    if contact.email.is_empty() {
        return false;
    }
    if !contact.github_url.is_empty() {
        return true;
    }

    if let Some(prof) = probe_github_user_by_email(client, &contact.email).await {
        contact.github_url = prof.html_url;
        contact.github_bio = prof.bio;
        if contact.avatar_url.is_empty() && !prof.avatar_url.is_empty() {
            contact.avatar_url = prof.avatar_url;
        }
        if contact.name.is_empty() && !prof.name.is_empty() {
            contact.name = prof.name;
        }
        if contact.role.is_empty() {
            contact.role = "Инженер / Разработчик (GitHub)".to_string();
        }
        if !contact.tags.iter().any(|t| t == "GitHub Lead") {
            contact.tags.push("GitHub Lead".to_string());
        }
        // Boost lead score
        contact.score = LeadScore::Hot;
        if contact.lead_score < 95 {
            contact.lead_score = 95;
        }
        return true;
    }

    false
}

fn urlencoding_simple(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode() {
        let email = "dev+test@example.com";
        let encoded = urlencoding_simple(email);
        assert_eq!(encoded, "dev%2Btest%40example.com");
    }
}
