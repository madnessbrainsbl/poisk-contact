use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkProfileReport {
    pub target_url: String,
    pub architecture_tier: String,
    pub detected_frameworks: Vec<String>,
    pub detected_backends: Vec<String>,
    pub is_modern_headless: bool,
    pub sales_readiness_score: u8,
    pub pitch_hook: String,
}

pub fn profile_framework_from_html(html: &str, url: &str) -> FrameworkProfileReport {
    let lower = html.to_lowercase();
    let mut frameworks = Vec::new();
    let mut backends = Vec::new();

    // 1. Modern Jamstack / Frontend Frameworks
    if lower.contains("__next_data__")
        || lower.contains("/_next/static")
        || lower.contains("/_next/")
    {
        frameworks.push("Next.js (React Framework)".to_string());
    }
    if lower.contains("__nuxt__") || lower.contains("/_nuxt/") || lower.contains("data-n-head") {
        frameworks.push("Nuxt.js (Vue Framework)".to_string());
    }
    if lower.contains("__sveltekit") || lower.contains("_app/immutable") {
        frameworks.push("SvelteKit (Fullstack Svelte)".to_string());
    }
    if lower.contains("__remixcontext") || lower.contains("remix-run") {
        frameworks.push("Remix (Fullstack Web)".to_string());
    }
    if lower.contains("data-astro-cid") || lower.contains("astro-island") {
        frameworks.push("Astro (Zero-JS Islands)".to_string());
    }
    if lower.contains("@vitejs") || lower.contains("vite/client") {
        frameworks.push("Vite (Modern Bundler)".to_string());
    }
    if lower.contains("___gatsby") || lower.contains("gatsby-data") {
        frameworks.push("Gatsby (Static Jamstack)".to_string());
    }
    if frameworks.is_empty() {
        if lower.contains("data-reactroot") || lower.contains("react.production") {
            frameworks.push("React SPA".to_string());
        } else if lower.contains("data-v-") || lower.contains("vue.runtime") {
            frameworks.push("Vue.js SPA".to_string());
        } else if lower.contains("ng-version") || lower.contains("ng-app") {
            frameworks.push("Angular SPA".to_string());
        }
    }

    // 2. Headless & Monolithic Backends / CMS
    if lower.contains("strapi") {
        backends.push("Strapi (Headless CMS)".to_string());
    }
    if lower.contains("sanity.io") || lower.contains("cdn.sanity.io") {
        backends.push("Sanity.io (Headless CMS)".to_string());
    }
    if lower.contains("contentful.com") || lower.contains("images.ctfassets.net") {
        backends.push("Contentful (API-first CMS)".to_string());
    }
    if lower.contains("supabase.co") {
        backends.push("Supabase (PostgreSQL BaaS)".to_string());
    }
    if lower.contains("tilda.ws")
        || lower.contains("tilda-blocks")
        || lower.contains("tildacdn.com")
    {
        backends.push("Tilda Publishing".to_string());
    }
    if lower.contains("/wp-content/")
        || lower.contains("/wp-includes/")
        || lower.contains("wp-json")
    {
        backends.push("WordPress (PHP Monolith)".to_string());
    }
    if lower.contains("/bitrix/js/") || lower.contains("/bitrix/templates/") {
        backends.push("1C-Bitrix (Enterprise Monolith)".to_string());
    }

    let is_modern = frameworks.iter().any(|f| {
        f.contains("Next.js")
            || f.contains("Nuxt.js")
            || f.contains("SvelteKit")
            || f.contains("Astro")
            || f.contains("Remix")
    });

    let (tier, score, hook) = if is_modern {
        (
            "Modern Jamstack & Headless Architecture".to_string(),
            95u8,
            "Tech-Forward engineering culture: Fast adoption of modern frontend stacks (Next.js/Nuxt). Highly receptive to developer tooling, AppSec, and API integrations.".to_string(),
        )
    } else if !frameworks.is_empty() {
        (
            "Modern Single Page Application (SPA)".to_string(),
            75u8,
            "Client-rendered SPA architecture. Target with frontend monitoring, API performance, and security testing solutions.".to_string(),
        )
    } else if backends.iter().any(|b| b.contains("Tilda")) {
        (
            "Cloud No-Code Builder (Tilda)".to_string(),
            45u8,
            "Low-code marketing landing page. Pitch turnkey B2B services rather than deep engineering integrations.".to_string(),
        )
    } else if backends
        .iter()
        .any(|b| b.contains("WordPress") || b.contains("Bitrix"))
    {
        (
            "Legacy Monolithic CMS (PHP / Self-Hosted)".to_string(),
            35u8,
            "Traditional monolithic setup. Potential pain points: server maintenance, legacy plugin vulnerabilities, and slow page speeds.".to_string(),
        )
    } else {
        (
            "Traditional Web Stack / Static HTML".to_string(),
            50u8,
            "Standard web presence. Qualify tech priorities directly during discovery call."
                .to_string(),
        )
    };

    FrameworkProfileReport {
        target_url: url.trim().to_string(),
        architecture_tier: tier,
        detected_frameworks: frameworks,
        detected_backends: backends,
        is_modern_headless: is_modern,
        sales_readiness_score: score,
        pitch_hook: hook,
    }
}

pub async fn profile_framework_from_url(
    client: &Client,
    url: &str,
) -> Result<FrameworkProfileReport, String> {
    let clean_url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url.trim())
    } else {
        url.trim().to_string()
    };

    let resp = client
        .get(&clean_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Failed reading response body: {}", e))?;
    Ok(profile_framework_from_html(&html, &clean_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_nextjs_jamstack() {
        let html = r#"<!DOCTYPE html><html><head><script id="__NEXT_DATA__" type="application/json">{"props":{}}</script></head><body><div id="__next">Hello</div></body></html>"#;
        let rep = profile_framework_from_html(html, "https://modern.io");
        assert!(rep.is_modern_headless);
        assert_eq!(
            rep.architecture_tier,
            "Modern Jamstack & Headless Architecture"
        );
        assert!(
            rep.detected_frameworks
                .iter()
                .any(|f| f.contains("Next.js"))
        );
        assert_eq!(rep.sales_readiness_score, 95);
    }

    #[test]
    fn test_detect_tilda_builder() {
        let html = r#"<html><head><script src="https://static.tildacdn.com/js/tilda-scripts-3.0.min.js"></script></head><body><div class="t-records"></div></body></html>"#;
        let rep = profile_framework_from_html(html, "https://agency.ru");
        assert!(!rep.is_modern_headless);
        assert_eq!(rep.architecture_tier, "Cloud No-Code Builder (Tilda)");
        assert!(rep.detected_backends.iter().any(|b| b.contains("Tilda")));
    }

    #[test]
    fn test_detect_legacy_wordpress() {
        let html = r#"<html><head><link rel="stylesheet" href="/wp-content/themes/twentytwenty/style.css"></head><body><h1>Blog</h1></body></html>"#;
        let rep = profile_framework_from_html(html, "https://legacy.com");
        assert!(!rep.is_modern_headless);
        assert_eq!(
            rep.architecture_tier,
            "Legacy Monolithic CMS (PHP / Self-Hosted)"
        );
        assert!(
            rep.detected_backends
                .iter()
                .any(|b| b.contains("WordPress"))
        );
    }
}
