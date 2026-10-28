use crate::models::{ContactRecord, LeadScore};
use crate::search::web::search_web;
use reqwest::Client;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GoogleMapsResult {
    pub records: Vec<ContactRecord>,
    pub discovered_websites: Vec<String>,
}

pub async fn search_google_maps(
    client: &Client,
    city: &str,
    stack: &str,
    limit: usize,
) -> Result<GoogleMapsResult, String> {
    let mut records = Vec::new();
    let mut discovered_websites = Vec::new();
    let mut seen_names = HashSet::new();

    let dork_queries = vec![
        format!("site:google.com/maps/place {} {}", city, stack),
        format!("site:google.com/maps {} {} contacts", city, stack),
        format!("\\\"google.com/maps\\\" {} {} website", stack, city),
    ];

    for query in dork_queries {
        if records.len() >= limit {
            break;
        }

        {
            let hits = search_web(client, &query, limit.max(15)).await?;
            for hit in hits {
                if records.len() >= limit {
                    break;
                }

                let url = hit.url;
                if url.contains("google.com/maps") {
                    let name = extract_place_name(&url);
                    if !name.is_empty() && !seen_names.contains(&name) {
                        seen_names.insert(name.clone());

                        let rec = ContactRecord::new(
                            "Sales / administration",
                            "",
                            &name,
                            "Google Maps / Places",
                            city,
                            stack,
                            "",
                            "",
                            "Google Maps",
                            &url,
                            LeadScore::Warm,
                            false,
                            "",
                        );
                        records.push(rec);
                    }
                } else if !url.contains("google.com")
                    && !url.contains("search.brave.com")
                    && !url.contains("yandex.ru")
                {
                    let clean_dom = clean_domain_from_url(&url);
                    if !clean_dom.is_empty() && !discovered_websites.contains(&clean_dom) {
                        discovered_websites.push(clean_dom);
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    Ok(GoogleMapsResult {
        records,
        discovered_websites,
    })
}

fn extract_place_name(url: &str) -> String {
    // Example: https://www.google.com/maps/place/Name+Of+Place/...
    if let Some(pos) = url.find("/place/") {
        let after = &url[pos + 7..];
        let raw_name = after.split(['/', '?', '#', '@']).next().unwrap_or("");
        if !raw_name.is_empty() {
            let decoded = urldecode_basic(raw_name).replace('+', " ");
            return decoded.trim().to_string();
        }
    }
    String::new()
}

fn clean_domain_from_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return format!("https://{}", host);
        }
    }
    url.to_string()
}

fn urldecode_basic(input: &str) -> String {
    let mut res = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                res.push(b as char);
                i += 3;
                continue;
            }
        }
        res.push(bytes[i] as char);
        i += 1;
    }
    res
}
