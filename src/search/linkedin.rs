use crate::models::ContactRecord;
use regex::Regex;
use reqwest::Client;
use std::sync::LazyLock;

static CARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<li[^>]*>(.*?)</li>"#).unwrap());

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"href="(https://[a-z]+\.linkedin\.com/jobs/view/[^"]+)""#).unwrap()
});

static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<h3[^>]*class="[^"]*base-search-card__title[^"]*"[^>]*>\s*(.*?)\s*</h3>"#)
        .unwrap()
});

static COMP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<h4[^>]*class="[^"]*base-search-card__subtitle[^"]*"[^>]*>\s*(?:<a[^>]*>)?\s*(.*?)\s*(?:</a>)?\s*</h4>"#).unwrap()
});

static LOC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<span[^>]*class="[^"]*job-search-card__location[^"]*"[^>]*>\s*(.*?)\s*</span>"#,
    )
    .unwrap()
});

static CLEAN_HTML_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"<[^>]+>"#).unwrap());

pub struct LinkedInSearchResult {
    pub records: Vec<ContactRecord>,
    pub company_domains: Vec<String>,
}

pub async fn search_linkedin_jobs(
    client: &Client,
    city: &str,
    stack: &str,
    role_filter: &str,
    limit: usize,
) -> Result<LinkedInSearchResult, String> {
    let kw = urlencoding(stack);
    let loc = urlencoding(city);
    let url = format!(
        "https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search?keywords={}&location={}&start=0",
        kw, loc
    );

    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9,ru;q=0.8")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("LinkedIn Jobs request failed: {}", e))?;

    let html = res
        .text()
        .await
        .map_err(|e| format!("Cannot read LinkedIn response: {}", e))?;

    let mut records = Vec::new();
    let mut company_domains = Vec::new();
    let mut seen_companies = std::collections::HashSet::new();

    for card_cap in CARD_RE.captures_iter(&html) {
        let card = card_cap.get(1).map_or("", |m| m.as_str());

        let job_url = match LINK_RE.captures(card).and_then(|c| c.get(1)) {
            Some(m) => m
                .as_str()
                .split('?')
                .next()
                .unwrap_or(m.as_str())
                .to_string(),
            None => continue,
        };

        let raw_title = TITLE_RE
            .captures(card)
            .and_then(|c| c.get(1))
            .map_or("", |m| m.as_str());
        let raw_comp = COMP_RE
            .captures(card)
            .and_then(|c| c.get(1))
            .map_or("", |m| m.as_str());
        let raw_loc = LOC_RE
            .captures(card)
            .and_then(|c| c.get(1))
            .map_or("", |m| m.as_str());

        let title = clean_text(raw_title);
        let company = clean_text(raw_comp);
        let location = clean_text(raw_loc);

        if company.is_empty() || title.is_empty() {
            continue;
        }

        let comp_key = company.to_lowercase();
        if seen_companies.contains(&comp_key) {
            continue;
        }
        seen_companies.insert(comp_key);

        let domain = derive_domain(&company);
        if !domain.is_empty() && !company_domains.contains(&domain) {
            company_domains.push(domain.clone());
        }

        // Generate corporate contact email based on role preference
        let (email_prefix, contact_name, target_role) =
            if matches!(role_filter, "decision_makers" | "руковод")
                || role_filter.contains("Фаундеры")
            {
                (
                    "ceo",
                    "Founder / Decision maker",
                    format!("CEO / Founder ({})", company),
                )
            } else if role_filter.eq_ignore_ascii_case("hr") {
                (
                    "hr",
                    "HR / Recruiter",
                    format!("HR / Talent Acquisition ({})", company),
                )
            } else {
                ("careers", "Hiring team", format!("Hiring Team / {}", title))
            };

        let contact_email = if !domain.is_empty() {
            format!("{}@{}", email_prefix, domain)
        } else {
            format!("careers@linkedin.com")
        };

        let mut record = ContactRecord::new(
            contact_name,
            &contact_email,
            &company,
            &target_role,
            if !location.is_empty() {
                &location
            } else {
                city
            },
            stack,
            "",
            "",
            "LinkedIn Public",
            &job_url,
            crate::models::LeadScore::Warm,
            false,
            "",
        );
        record.id = format!(
            "li_{}_{}",
            sanitize_id(&company),
            chrono::Utc::now().timestamp_millis()
        );
        record.is_role_account = crate::search::extractor::is_role_based_email(&contact_email);
        record.mail_provider = crate::search::extractor::detect_mail_provider(&contact_email);
        record.lead_score = crate::search::extractor::calculate_lead_score(&record);

        records.push(record);
        if records.len() >= limit {
            break;
        }
    }

    Ok(LinkedInSearchResult {
        records,
        company_domains,
    })
}

fn clean_text(input: &str) -> String {
    let no_tags = CLEAN_HTML_RE.replace_all(input, "");
    let unescaped = no_tags
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    unescaped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn derive_domain(company: &str) -> String {
    let lower = company.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| {
            !w.is_empty()
                && !matches!(
                    *w,
                    "inc"
                        | "llc"
                        | "ltd"
                        | "gmbh"
                        | "corp"
                        | "group"
                        | "co"
                        | "technologies"
                        | "solutions"
                        | "oo"
                        | "ooo"
                )
        })
        .collect();

    if words.is_empty() {
        return "".to_string();
    }
    format!("{}.com", words.join(""))
}

fn sanitize_id(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Explicit live LinkedIn audit; the provider can throttle requests"]
    async fn test_linkedin_search() -> Result<(), String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap();
        let res = search_linkedin_jobs(&client, "Remote", "Security", "Все", 5).await;
        match res {
            Ok(val) if !val.records.is_empty() => {
                println!("LIVE LinkedIn: {} records", val.records.len());
                Ok(())
            }
            Ok(_) => Err("LinkedIn returned no public records".into()),
            Err(error) => {
                eprintln!("LinkedIn unavailable: {error}");
                Err(error)
            }
        }
    }
}
