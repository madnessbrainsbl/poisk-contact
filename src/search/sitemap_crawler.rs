use regex::Regex;
use reqwest::Client;
use std::time::Duration;

const CONTACT_KEYWORDS: &[&str] = &[
    "contact",
    "kontakty",
    "o-nas",
    "about",
    "team",
    "komanda",
    "rekvizit",
    "career",
    "job",
    "vakansii",
    "feedback",
    "support",
    "management",
    "leadership",
];

pub async fn discover_contact_pages_via_sitemap(client: &Client, domain: &str) -> Vec<String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    if clean_domain.is_empty() {
        return Vec::new();
    }

    let mut candidate_sitemaps = Vec::new();

    // 1. Check robots.txt for Sitemap directive
    let robots_url = format!("https://{}/robots.txt", clean_domain);
    if let Ok(resp) = client
        .get(&robots_url)
        .timeout(Duration::from_secs(4))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                if let Ok(sitemap_re) = Regex::new(r#"(?i)^\s*Sitemap:\s*(https?://\S+)"#) {
                    for cap in sitemap_re.captures_iter(&body) {
                        if let Some(m) = cap.get(1) {
                            let sm_url = m.as_str().trim().to_string();
                            if !candidate_sitemaps.contains(&sm_url) {
                                candidate_sitemaps.push(sm_url);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallbacks if robots.txt didn't specify sitemap
    if candidate_sitemaps.is_empty() {
        candidate_sitemaps.push(format!("https://{}/sitemap.xml", clean_domain));
        candidate_sitemaps.push(format!("https://{}/sitemap_index.xml", clean_domain));
    }

    let loc_re = match Regex::new(r#"(?is)<loc>\s*(https?://[^<\s]+)\s*</loc>"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut discovered_pages: Vec<String> = Vec::new();

    // Probe candidate sitemaps (take max 2 to prevent excessive crawling)
    for sm_url in candidate_sitemaps.into_iter().take(2) {
        let resp = match client
            .get(&sm_url)
            .timeout(Duration::from_secs(5))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => continue,
        };

        for cap in loc_re.captures_iter(&body) {
            if let Some(m) = cap.get(1) {
                let loc = m.as_str().trim();
                let lower = loc.to_lowercase();

                // If loc points to sub-sitemap (e.g. page-sitemap.xml)
                if lower.ends_with(".xml") || lower.contains("sitemap") {
                    continue;
                }

                // Check against high-value contact keywords
                if CONTACT_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
                    let page = loc.to_string();
                    if !discovered_pages.contains(&page) {
                        discovered_pages.push(page);
                        if discovered_pages.len() >= 5 {
                            break;
                        }
                    }
                }
            }
        }

        if !discovered_pages.is_empty() {
            break;
        }
    }

    discovered_pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_sitemap_loc() {
        let xml = r#"
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
                <url><loc>https://example.com/</loc></url>
                <url><loc>https://example.com/products/item-1</loc></url>
                <url><loc>https://example.com/kontakty</loc></url>
                <url><loc>https://example.com/o-nas/komanda</loc></url>
            </urlset>
        "#;
        let loc_re = Regex::new(r#"(?is)<loc>\s*(https?://[^<\s]+)\s*</loc>"#).unwrap();
        let mut matches = Vec::new();
        for cap in loc_re.captures_iter(xml) {
            let loc = cap.get(1).unwrap().as_str();
            let lower = loc.to_lowercase();
            if CONTACT_KEYWORDS.iter().any(|kw| lower.contains(kw)) {
                matches.push(loc.to_string());
            }
        }
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"https://example.com/kontakty".to_string()));
        assert!(matches.contains(&"https://example.com/o-nas/komanda".to_string()));
    }
}
