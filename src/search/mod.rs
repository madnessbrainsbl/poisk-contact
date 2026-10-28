pub mod arbeitnow;
pub mod bounce_classifier;
pub mod business;
pub mod career_signals;
pub mod case_study_miner;
pub mod catchall_scorer;
pub mod cloud_infra;
pub mod company_tier;
pub mod compliance;
mod contact_blocks;
pub mod crawler;
pub mod cta_friction_scorer;
pub mod disposable;
pub mod dmarc_bimi;
pub mod dns_ttl_risk;
pub mod doh;
pub mod domain_pattern;
pub mod domain_prospector;
pub mod domain_typo;
pub mod duckduckgo;
pub mod ecommerce_detect;
pub mod email_pattern_engine;
pub mod extractor;
pub mod fcrdns;
pub mod fcrdns_validator;
pub mod firmographics;
pub mod framework_profiler;
pub mod github_osint;
pub mod google_maps;
pub mod greylist_detector;
pub mod hn;
mod http;
pub mod icebreaker;
pub mod intent_signals;
pub mod jobicy;
pub mod keyserver;
pub mod link_density;
pub mod linkedin;
mod location;
pub mod lookalike;
pub mod mail_recon;
pub mod mailbox_deliverability_tier;
pub mod mime_body_auditor;
pub mod mx_fingerprint;
pub mod mx_priority;
pub mod mx_redundancy_auditor;
mod osm;
pub mod permutator;
pub mod phone_intel;
pub mod preflight_compliance;
pub mod rdap;
pub mod readability;
pub mod reading_load;
pub mod recon;
pub mod remoteok;
pub mod remotive;
pub mod role_mailbox_risk;
pub mod scrapling;
pub mod security_posture;
pub mod sending_window;
pub mod seniority;
pub mod sentiment_classifier;
pub mod sitemap_crawler;
pub mod smart_hooks;
pub mod smtp_probe;
pub mod spam_category_matrix;
pub mod spam_trigger;
pub mod spamtrap_guard;
pub mod spf_lookup_auditor;
pub mod spf_syntax_optimizer;
pub mod starttls_probe;
pub mod subject_optimizer;
pub mod tax_id;
pub mod tech_detect;
pub mod tech_maturity;
pub mod telegram;
pub mod title_normalizer;
pub mod voip_detect;
pub mod waterfall_merge;
pub mod web;
pub mod well_known;

use crate::models::{ContactRecord, SearchFilter};
use arbeitnow::search_arbeitnow;
use crawler::crawl_with_location;
use doh::MxVerifier;
use google_maps::search_google_maps;
use hn::search_hn_whoishiring;
use jobicy::search_jobicy;
use linkedin::search_linkedin_jobs;
use remoteok::search_remoteok;
use remotive::search_remotive;
use reqwest::Client;
use std::sync::mpsc::Sender;
use std::time::Duration;
use telegram::search_telegram;
use web::search_web;

pub enum SearchProgress {
    Log(String, String), // level, message
    Done(Vec<ContactRecord>),
    Collected(Vec<ContactRecord>),
    Error(String),
}

fn web_queries(city: &str, query: &str) -> [String; 2] {
    let terms = format!("{} {}", query.trim(), city.trim());
    let terms = terms.trim();
    let contacts = if terms
        .chars()
        .any(|c| ('\u{0400}'..='\u{04ff}').contains(&c))
    {
        "контакты"
    } else {
        "contact"
    };
    [format!("{terms} {contacts}"), format!("{terms} email")]
}

fn translated_query(data: &serde_json::Value) -> Option<String> {
    if data["responseStatus"].as_u64() != Some(200) || data["quotaFinished"].as_bool() == Some(true)
    {
        return None;
    }
    data["responseData"]["translatedText"]
        .as_str()
        .map(|text| {
            text.trim()
                .trim_end_matches(['.', '!', '?'])
                .trim()
                .to_string()
        })
        .filter(|text| !text.is_empty())
}

async fn local_query_terms(
    client: &Client,
    location: &location::Location,
    query: &str,
) -> Result<String, String> {
    if location.country_code != "RU"
        || query
            .chars()
            .any(|c| ('\u{0400}'..='\u{04ff}').contains(&c))
    {
        return Ok(query.into());
    }
    if query.len() > 500 {
        return Err(
            "Local-language query translation is limited to 500 bytes; using the original terms."
                .into(),
        );
    }
    let mut url = url::Url::parse("https://api.mymemory.translated.net/get").unwrap();
    url.query_pairs_mut()
        .extend_pairs([("q", query), ("langpair", "en|ru")]);
    let data = http::get_json(client, url, Duration::from_secs(6))
        .await
        .map_err(|e| format!("Local-language query translation unavailable: {e:#}"))?;
    translated_query(&data).ok_or_else(|| "Local-language query translation unavailable or quota exceeded; using the original terms.".into())
}

fn matches_role(role: &str, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    let role_lower = role.to_lowercase();
    match filter.as_str() {
        "" | "all" | "все" => true,
        "decision_makers" | "руковод" => {
            title_normalizer::normalize_job_title(role).is_decision_maker
        }
        "hr" => {
            seniority::classify_title(role).1 == seniority::Department::HumanResources
                || role_lower.contains("recruit")
        }
        _ => {
            role_lower.contains(&filter)
                || filter
                    .split_whitespace()
                    .any(|word| role_lower.contains(word))
        }
    }
}

pub async fn run_hybrid_search(filter: SearchFilter, tx: Sender<SearchProgress>) {
    let client = match Client::builder().timeout(Duration::from_secs(12)).build() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(SearchProgress::Error(format!(
                "HTTP client initialization failed: {}",
                e
            )));
            return;
        }
    };

    let mx_verifier = MxVerifier::new();
    let mut all_contacts: Vec<ContactRecord> = Vec::new();
    let mut discovered_urls: Vec<String> = Vec::new();

    let _ = tx.send(SearchProgress::Log(
        "INFO".to_string(),
        format!(
            "Search started: location '{}', query '{}'...",
            filter.city, filter.stack
        ),
    ));

    // 1. LinkedIn Global Jobs
    if filter.search_linkedin && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching hiring companies on LinkedIn...".to_string(),
        ));

        match search_linkedin_jobs(
            &client,
            &filter.city,
            &filter.stack,
            &filter.role_filter,
            filter.max_items,
        )
        .await
        {
            Ok(result) => {
                let count = result.records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("LinkedIn: found {} companies and positions", count),
                ));
                for mut r in result.records {
                    if !all_contacts
                        .iter()
                        .any(|c| c.email == r.email || c.url == r.url)
                    {
                        let (mx_ok, mx_rec) = mx_verifier.check_email(&client, &r.email).await;
                        r.mx_valid = mx_ok;
                        r.mx_record = mx_rec;
                        all_contacts.push(r);
                    }
                }

                if filter.crawl_websites {
                    for dom in result.company_domains {
                        let dom_url = format!("https://{}", dom);
                        if !discovered_urls.contains(&dom_url) {
                            discovered_urls.push(dom_url);
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 2. Remotive Worldwide Tech Jobs
    if filter.search_remotive && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching international jobs through Remotive...".to_string(),
        ));

        match search_remotive(&client, &filter.stack, filter.max_items, &mx_verifier).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Remotive: found {} jobs and contacts", count),
                ));
                for r in records {
                    if !all_contacts
                        .iter()
                        .any(|c| (!r.email.is_empty() && c.email == r.email) || c.url == r.url)
                    {
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 3. Arbeitnow EU & Remote Jobs
    if filter.search_arbeitnow && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching jobs through Arbeitnow (Europe / remote)...".to_string(),
        ));

        match search_arbeitnow(&client, &filter.stack, filter.max_items, &mx_verifier).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Arbeitnow: collected {} jobs and contacts", count),
                ));
                for r in records {
                    if !all_contacts
                        .iter()
                        .any(|c| (!r.email.is_empty() && c.email == r.email) || c.url == r.url)
                    {
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 4. RemoteOK Global Jobs
    if filter.search_remoteok && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching remote jobs through RemoteOK...".to_string(),
        ));

        match search_remoteok(&client, &filter.stack, filter.max_items, &mx_verifier).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("RemoteOK: collected {} jobs and contacts", count),
                ));
                for r in records {
                    if !all_contacts
                        .iter()
                        .any(|c| (!r.email.is_empty() && c.email == r.email) || c.url == r.url)
                    {
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 5. Jobicy Remote Jobs API
    if filter.search_jobicy && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching Jobicy Remote Jobs...".to_string(),
        ));

        match search_jobicy(&client, &filter.stack, filter.max_items, &mx_verifier).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Jobicy: found {} contacts", count),
                ));
                for r in records {
                    if !all_contacts
                        .iter()
                        .any(|c| (!r.email.is_empty() && c.email == r.email) || c.url == r.url)
                    {
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 6. Hacker News "Who is Hiring"
    if filter.search_hn && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Checking Hacker News Who is Hiring...".to_string(),
        ));

        match search_hn_whoishiring(&client, &filter.stack, filter.max_items, &mx_verifier).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Hacker News: collected {} direct contacts", count),
                ));
                for r in records {
                    if !all_contacts.iter().any(|c| c.email == r.email) {
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 7. Telegram International Tech Channels
    if filter.search_telegram && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Collecting recruiter contacts from Telegram IT channels...".to_string(),
        ));

        match search_telegram(&client, filter.max_items).await {
            Ok(records) => {
                let count = records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Telegram: found {} contacts from job posts", count),
                ));
                for mut r in records {
                    if !all_contacts.iter().any(|c| {
                        (!r.email.is_empty() && c.email == r.email)
                            || (!r.telegram.is_empty() && c.telegram == r.telegram)
                    }) {
                        if !r.email.is_empty() && filter.verify_mx {
                            let (mx_ok, mx_rec) = mx_verifier.check_email(&client, &r.email).await;
                            r.mx_valid = mx_ok;
                            r.mx_record = mx_rec;
                        }
                        all_contacts.push(r);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 8. Google Maps Tech Hubs & Places
    if filter.search_google_maps && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching companies through Google Maps...".to_string(),
        ));

        match search_google_maps(&client, &filter.city, &filter.stack, filter.max_items).await {
            Ok(result) => {
                let count = result.records.len();
                let _ = tx.send(SearchProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Google Maps: found {} companies and hubs", count),
                ));

                for r in result.records {
                    if !all_contacts
                        .iter()
                        .any(|c| c.company == r.company || c.url == r.url)
                    {
                        all_contacts.push(r);
                    }
                }

                if filter.crawl_websites {
                    for site in result.discovered_websites {
                        if !discovered_urls.contains(&site) {
                            discovered_urls.push(site);
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(SearchProgress::Log("WARN".to_string(), e));
            }
        }
    }

    // 9. General company/contact search and vacancy pages
    let mut web_errors = Vec::new();
    let location = if filter.search_web || filter.crawl_websites {
        location::Location::resolve(&client, &filter.city)
            .await
            .unwrap_or_else(|error| {
                let _ = tx.send(SearchProgress::Log("WARN".into(), error));
                location::Location::raw(&filter.city)
            })
    } else {
        location::Location::raw(&filter.city)
    };
    if filter.search_web && all_contacts.len() < filter.max_items {
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            "Searching the web for companies, contacts and jobs...".to_string(),
        ));

        let mut queries = web_queries(&filter.city, &filter.stack).to_vec();
        let mut official_query = format!(
            "{} {} official website",
            filter.stack.trim(),
            filter.city.trim()
        );
        if !location.native.is_empty()
            && location.native.trim().to_lowercase()
                != filter
                    .city
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase()
        {
            let terms = local_query_terms(&client, &location, &filter.stack)
                .await
                .unwrap_or_else(|error| {
                    let _ = tx.send(SearchProgress::Log("WARN".into(), error));
                    filter.stack.clone()
                });
            let local_queries = web_queries(&location.native, &terms.to_lowercase());
            let suffix = if location.country_code == "RU" {
                "официальный сайт"
            } else {
                "official website"
            };
            official_query = format!("{} {} {suffix}", terms.to_lowercase(), location.native);
            queries[1] = queries[0].clone();
            queries[0] = local_queries[0].clone();
            queries.push(local_queries[1].clone());
        }
        for q in queries {
            let _ = tx.send(SearchProgress::Log(
                "INFO".to_string(),
                format!("Web query (Brave / Yahoo / Mwmbl): {q}"),
            ));

            match search_web(&client, &q, filter.max_items.max(15)).await {
                Ok(hits) => {
                    for hit in hits {
                        if !discovered_urls.contains(&hit.url) {
                            discovered_urls.push(hit.url);
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(SearchProgress::Log("WARN".into(), error.clone()));
                    web_errors.push(error);
                }
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        if filter.crawl_websites
            && !discovered_urls
                .iter()
                .any(|url| crawler::is_company_website(url))
        {
            let _ = tx.send(SearchProgress::Log("INFO".into(), format!("Search results contain only listings; finding company websites: {official_query}")));
            match search_web(&client, &official_query, filter.max_items.max(15)).await {
                Ok(hits) => {
                    for hit in hits {
                        if !discovered_urls.contains(&hit.url) {
                            discovered_urls.push(hit.url);
                        }
                    }
                }
                Err(error) => {
                    let _ = tx.send(SearchProgress::Log("WARN".into(), error.clone()));
                    web_errors.push(error);
                }
            }
        }
        if discovered_urls.is_empty() && all_contacts.is_empty() && !web_errors.is_empty() {
            let _ = tx.send(SearchProgress::Error(web_errors.join("; ")));
            return;
        }
    }

    // 10. Deep Website Crawler for all discovered international domains
    if filter.crawl_websites && !discovered_urls.is_empty() {
        let total_urls = discovered_urls.len();
        let _ = tx.send(SearchProgress::Log(
            "INFO".to_string(),
            format!("Contact crawler: checking {} websites...", total_urls),
        ));

        let mut crawled_count = 0;
        let mut crawl_errors = Vec::new();
        for url in discovered_urls {
            if all_contacts.len() >= filter.max_items {
                break;
            }
            crawled_count += 1;
            let _ = tx.send(SearchProgress::Log(
                "INFO".into(),
                format!("Scanning website ({crawled_count}/{total_urls}): {url}"),
            ));

            match crawl_with_location(&client, &url, &filter.city, &filter.stack, &location).await {
                Ok(crawled) => {
                    for mut r in crawled {
                        r.stack = filter.stack.clone();

                        let is_dup = all_contacts.iter().any(|c| {
                            (!c.email.is_empty() && c.email == r.email)
                                || (!c.phone.is_empty() && c.phone == r.phone)
                        });

                        if !is_dup {
                            if !r.email.is_empty() && filter.verify_mx {
                                let (mx_ok, mx_rec) =
                                    mx_verifier.check_email(&client, &r.email).await;
                                r.mx_valid = mx_ok;
                                r.mx_record = mx_rec;
                            }
                            all_contacts.push(r);
                        }
                    }
                }
                Err(error) => {
                    crawl_errors.push(error.clone());
                    let _ = tx.send(SearchProgress::Log(
                        "WARN".into(),
                        format!("{url}: {error}"),
                    ));
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        if all_contacts.is_empty() && !crawl_errors.is_empty() {
            let _ = tx.send(SearchProgress::Error(format!(
                "No contacts collected; {} websites could not be read. {}",
                crawl_errors.len(),
                crawl_errors[0]
            )));
            return;
        }
    }

    // Role-based filtering if chosen
    if all_contacts.is_empty() && !web_errors.is_empty() {
        let _ = tx.send(SearchProgress::Error(format!(
            "No contacts collected; some web queries failed. {}",
            web_errors.join("; ")
        )));
        return;
    }
    let final_contacts: Vec<_> = all_contacts
        .into_iter()
        .filter(|contact| matches_role(&contact.role, &filter.role_filter))
        .take(filter.max_items)
        .collect();
    let with_email_count = final_contacts
        .iter()
        .filter(|c| !c.email.is_empty())
        .count();
    let _ = tx.send(SearchProgress::Log(
        "SUCCESS".to_string(),
        format!(
            "Collection complete: {} contacts ({} with email).",
            final_contacts.len(),
            with_email_count
        ),
    ));

    let _ = tx.send(SearchProgress::Done(final_contacts));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_translation_before_using_it_in_a_query() {
        let data = serde_json::json!({"responseStatus":200,"quotaFinished":false,"responseData":{"translatedText":" Фотостудия. "}});
        let terms = translated_query(&data).unwrap();
        assert_eq!(
            web_queries("Красноярск", &terms)[0],
            "Фотостудия Красноярск контакты"
        );
        assert!(translated_query(&serde_json::json!({"responseStatus":429,"responseData":{"translatedText":"Quota exceeded"}})).is_none());
        assert!(translated_query(&serde_json::json!({"responseStatus":200,"quotaFinished":true,"responseData":{"translatedText":"Quota exceeded"}})).is_none());
        assert!(
            translated_query(
                &serde_json::json!({"responseStatus":200,"responseData":{"translatedText":" "}})
            )
            .is_none()
        );
    }

    #[test]
    fn contact_person_filter_supports_english_and_legacy_values() {
        for filter in ["decision_makers", "руковод"] {
            for title in [
                "CEO / Founder",
                "Head of Engineering",
                "Руководитель отдела",
            ] {
                assert!(matches_role(title, filter), "{title}: {filter}");
            }
            assert!(!matches_role("Junior Developer", filter));
        }
        for title in ["HR", "IT Recruiter", "Talent Acquisition", "Рекрутер"] {
            assert!(matches_role(title, "hr"), "{title}");
        }
        assert!(!matches_role("Software Engineer", "hr"));
        assert!(matches_role("", "All"));
        assert!(matches_role("", "Все"));
    }

    #[tokio::test]
    #[ignore = "Explicit live pipeline audit; requires public network access"]
    async fn live_pipeline_honors_requested_size() -> Result<(), String> {
        let filter = SearchFilter {
            city: "Remote".into(),
            stack: "Security".into(),
            max_items: 2,
            search_linkedin: true,
            search_remotive: false,
            search_arbeitnow: false,
            search_remoteok: false,
            search_jobicy: false,
            search_hn: false,
            search_telegram: false,
            search_google_maps: false,
            search_web: false,
            crawl_websites: false,
            verify_mx: false,
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        tokio::time::timeout(Duration::from_secs(60), run_hybrid_search(filter, tx))
            .await
            .map_err(|e| format!("Live pipeline timed out: {e}"))?;
        for event in rx.try_iter() {
            match event {
                SearchProgress::Log(level, text) => {
                    println!("{level}: {text}");
                    if level == "WARN" {
                        return Err(text);
                    }
                }
                SearchProgress::Error(error) => return Err(error),
                SearchProgress::Collected(_) => {}
                SearchProgress::Done(records) => {
                    if records.is_empty() || records.len() > 2 {
                        return Err(format!(
                            "Expected 1–2 public records, got {}",
                            records.len()
                        ));
                    }
                    if records
                        .iter()
                        .any(|r| !r.url.starts_with("https://") || r.company.is_empty())
                    {
                        return Err("Invalid public company records".into());
                    }
                    println!(
                        "LIVE PIPELINE: {} records, valid company URLs, requested maximum respected",
                        records.len()
                    );
                    return Ok(());
                }
            }
        }
        Err("Live pipeline did not complete".into())
    }

    #[test]
    fn web_search_accepts_arbitrary_places_and_businesses() {
        for (city, query) in [
            ("Улан-Удэ", "мебельные фабрики"),
            ("São Paulo", "clínicas odontológicas"),
        ] {
            let queries = web_queries(city, query);
            assert!(
                queries
                    .iter()
                    .all(|q| q.contains(city) && q.contains(query))
            );
            assert!(!queries[0].contains("site:"));
            assert!(!queries[1].contains("site:"));
            assert!(
                queries
                    .iter()
                    .all(|q| !q.contains("remote") && !q.contains("hiring"))
            );
        }
    }

    #[test]
    fn web_search_allows_omitting_location() {
        let queries = web_queries("  ", "  мебельные фабрики  ");
        assert_eq!(queries[0], "мебельные фабрики контакты");
        assert!(
            queries
                .iter()
                .all(|q| !q.contains("\"\"") && !q.contains("  "))
        );
    }
}
