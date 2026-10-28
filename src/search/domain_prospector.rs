use crate::models::{ContactRecord, LeadScore};
use reqwest::Client;
use std::collections::HashSet;

pub const COMMON_TEAM_ROUTES: &[&str] = &[
    "/team",
    "/about",
    "/contacts",
    "/leadership",
    "/komanda",
    "/kontakty",
    "/o-kompanii",
    "/about-us",
    "/company",
];

pub fn capitalize_token(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn derive_name_from_localpart(local: &str) -> (String, String) {
    let clean = local.trim().to_lowercase();
    if clean.is_empty() {
        return ("Контакт".to_string(), "".to_string());
    }

    if let Some((first, last)) = clean.split_once('.') {
        (capitalize_token(first), capitalize_token(last))
    } else if let Some((first, last)) = clean.split_once('_') {
        (capitalize_token(first), capitalize_token(last))
    } else if let Some((first, last)) = clean.split_once('-') {
        (capitalize_token(first), capitalize_token(last))
    } else {
        (capitalize_token(&clean), "".to_string())
    }
}

pub async fn prospect_domain_team(
    client: &Client,
    raw_domain: &str,
    max_prospects: usize,
) -> Vec<ContactRecord> {
    let domain = raw_domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();

    if domain.is_empty() {
        return Vec::new();
    }

    let mut found_emails = HashSet::new();
    let mut prospects = Vec::new();

    let email_re = regex::Regex::new(r"(?i)[a-z0-9_.+-]+@[a-z0-9-]+\.[a-z0-9-.]+").unwrap();

    let base_url = format!("https://{}", domain);

    for route in COMMON_TEAM_ROUTES {
        let target_url = format!("{}{}", base_url, route);
        if let Ok(resp) = client
            .get(&target_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    for cap in email_re.find_iter(&body) {
                        let email = cap.as_str().to_lowercase();
                        if !email.ends_with(".png")
                            && !email.ends_with(".jpg")
                            && !email.ends_with(".webp")
                            && !email.ends_with(".svg")
                            && email.contains(&domain)
                        {
                            found_emails.insert(email);
                        }
                    }
                }
            }
        }
        if found_emails.len() >= max_prospects {
            break;
        }
    }

    for email in found_emails.into_iter().take(max_prospects) {
        let local = email.split('@').next().unwrap_or("");
        let (first, last) = derive_name_from_localpart(local);
        let full_name = if last.is_empty() {
            first.clone()
        } else {
            format!("{} {}", first, last)
        };

        let mut rec = ContactRecord::new(
            &full_name,
            &email,
            &domain,
            "Сотрудник / Команда компании",
            "Не указан",
            "",
            "",
            "",
            "Snov.io Domain Prospector",
            &base_url,
            LeadScore::Warm,
            true,
            "",
        );
        rec.b2b_tier = "Tier 1".to_string();
        crate::search::seniority::apply_seniority_to_contact(&mut rec);
        crate::search::disposable::apply_mailbox_tier_to_contact(&mut rec);
        prospects.push(rec);
    }

    prospects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_name_from_localpart() {
        let (f, l) = derive_name_from_localpart("alexey.smirnov");
        assert_eq!(f, "Alexey");
        assert_eq!(l, "Smirnov");

        let (f2, l2) = derive_name_from_localpart("john_doe");
        assert_eq!(f2, "John");
        assert_eq!(l2, "Doe");

        let (f3, l3) = derive_name_from_localpart("ceo");
        assert_eq!(f3, "Ceo");
        assert_eq!(l3, "");
    }

    #[test]
    fn test_capitalize_token() {
        assert_eq!(capitalize_token("rust"), "Rust");
        assert_eq!(capitalize_token(""), "");
    }
}
