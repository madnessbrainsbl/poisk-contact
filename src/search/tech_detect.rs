use regex::Regex;
use std::sync::LazyLock;

static YM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:ym\(|watch/|metrika/tag\.js[^\d]{0,20})(\d{6,10})").unwrap()
});

static GA4_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(G-[A-Z0-9]{7,12})\b").unwrap());

static GTM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(GTM-[A-Z0-9]{6,10})\b").unwrap());

static VK_PIXEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(VK-RTRG-\d{4,10})\b").unwrap());

static GH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)boards\.greenhouse\.io/(?:embed/job_board/js\?for=|v1/boards/|)([a-zA-Z0-9_-]{2,40})"#).unwrap()
});

static ASHBY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)jobs\.ashbyhq\.com/([a-zA-Z0-9_-]{2,40})"#).unwrap());

static LEVER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)jobs\.lever\.co/([a-zA-Z0-9_-]{2,40})"#).unwrap());

static SR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)jobs\.smartrecruiters\.com/([a-zA-Z0-9_-]{2,40})"#).unwrap()
});

static WORKABLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)apply\.workable\.com/([a-zA-Z0-9_-]{2,40})"#).unwrap());

static WORKDAY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(?:https?://)?([a-zA-Z0-9_-]+)\.(?:wd\d+\.)?myworkdayjobs\.com/([a-zA-Z0-9_-]+)"#,
    )
    .unwrap()
});

static BAMBOO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)([a-zA-Z0-9_-]+)\.bamboohr\.com/careers"#).unwrap());

#[derive(Debug, Clone, Default)]
pub struct TechProfile {
    pub cms: Option<String>,
    pub crm: Option<String>,
    pub live_chat: Option<String>,
    pub ats: Option<(String, String)>,
    pub analytics: Vec<String>,
    pub all_tags: Vec<String>,
}

impl TechProfile {
    pub fn summary_string(&self) -> String {
        self.all_tags.join(", ")
    }
}

pub fn detect_tech_stack(html: &str, headers_hint: &str) -> TechProfile {
    let mut profile = TechProfile::default();
    let lower_html = html.to_lowercase();
    let lower_headers = headers_hint.to_lowercase();

    // 1. CMS & Website Builders
    if lower_html.contains("/bitrix/")
        || lower_html.contains("bitrix_sessid")
        || lower_html.contains("bx.ready")
        || lower_html.contains("bx-panel")
    {
        profile.cms = Some("1C-Битрикс".to_string());
        profile.all_tags.push("CMS: 1C-Битрикс".to_string());
    } else if lower_html.contains("tilda.cc")
        || lower_html.contains("tilda-blocks")
        || lower_html.contains("tilda-publishing")
        || lower_html.contains("tildacdn.com")
    {
        profile.cms = Some("Tilda".to_string());
        profile.all_tags.push("CMS: Tilda".to_string());
    } else if lower_html.contains("/wp-content/")
        || lower_html.contains("/wp-includes/")
        || lower_html.contains("wp-json")
    {
        profile.cms = Some("WordPress".to_string());
        profile.all_tags.push("CMS: WordPress".to_string());
    } else if lower_html.contains("cdn.shopify.com") || lower_html.contains("shopify.theme") {
        profile.cms = Some("Shopify".to_string());
        profile.all_tags.push("CMS: Shopify".to_string());
    } else if lower_html.contains("catalog/view/theme") || lower_html.contains("index.php?route=") {
        profile.cms = Some("OpenCart".to_string());
        profile.all_tags.push("CMS: OpenCart".to_string());
    } else if lower_html.contains("webflow.js") || lower_html.contains("wf-page") {
        profile.cms = Some("Webflow".to_string());
        profile.all_tags.push("CMS: Webflow".to_string());
    } else if lower_html.contains("__next_data__") || lower_html.contains("_next/static") {
        profile.cms = Some("Next.js".to_string());
        profile.all_tags.push("Frontend: Next.js".to_string());
    } else if lower_html.contains("__nuxt__") || lower_html.contains("_nuxt/") {
        profile.cms = Some("Nuxt.js".to_string());
        profile.all_tags.push("Frontend: Nuxt.js".to_string());
    }

    // 2. CRM & Lead Management
    if lower_html.contains("b24-widget")
        || lower_html.contains("bitrix24.ru")
        || lower_html.contains("b24form")
    {
        profile.crm = Some("Битрикс24".to_string());
        profile.all_tags.push("CRM: Битрикс24".to_string());
    } else if lower_html.contains("amo-button")
        || lower_html.contains("amocrm.ru")
        || lower_html.contains("crm_lead")
    {
        profile.crm = Some("amoCRM".to_string());
        profile.all_tags.push("CRM: amoCRM".to_string());
    } else if lower_html.contains("retailcrm.ru") || lower_html.contains("retailcrm.pro") {
        profile.crm = Some("RetailCRM".to_string());
        profile.all_tags.push("CRM: RetailCRM".to_string());
    } else if lower_html.contains("hs-scripts.com") || lower_html.contains("hbspt.forms") {
        profile.crm = Some("HubSpot".to_string());
        profile.all_tags.push("CRM: HubSpot".to_string());
    }

    // 3. Live Chat & Customer Support
    if lower_html.contains("code.jivo.ru")
        || lower_html.contains("jivosite")
        || lower_html.contains("jv_ready")
    {
        profile.live_chat = Some("JivoChat".to_string());
        profile.all_tags.push("Чат: JivoChat".to_string());
    } else if lower_html.contains("carrotquest.io") || lower_html.contains("carrotquest") {
        profile.live_chat = Some("Carrot quest".to_string());
        profile.all_tags.push("Чат: Carrot quest".to_string());
    } else if lower_html.contains("widget.intercom.io") || lower_html.contains("intercomsettings") {
        profile.live_chat = Some("Intercom".to_string());
        profile.all_tags.push("Чат: Intercom".to_string());
    } else if lower_html.contains("static.zdassets.com") || lower_html.contains("zopim") {
        profile.live_chat = Some("Zendesk".to_string());
        profile.all_tags.push("Чат: Zendesk".to_string());
    } else if lower_html.contains("envybox.io") {
        profile.live_chat = Some("Envybox".to_string());
        profile.all_tags.push("Чат: Envybox".to_string());
    } else if lower_html.contains("talk-me.ru") {
        profile.live_chat = Some("Talk-Me".to_string());
        profile.all_tags.push("Чат: Talk-Me".to_string());
    }

    // 4. Analytics & Digital Footprint IDs
    for cap in YM_RE.captures_iter(html) {
        if let Some(id_match) = cap.get(1) {
            let id = id_match.as_str();
            let tag = format!("Метрика ID: {}", id);
            if !profile.analytics.contains(&tag) {
                profile.analytics.push(tag.clone());
                profile.all_tags.push(tag);
            }
        }
    }

    for cap in GA4_RE.captures_iter(html) {
        if let Some(id_match) = cap.get(1) {
            let id = id_match.as_str().to_uppercase();
            let tag = format!("GA4: {}", id);
            if !profile.analytics.contains(&tag) {
                profile.analytics.push(tag.clone());
                profile.all_tags.push(tag);
            }
        }
    }

    for cap in GTM_RE.captures_iter(html) {
        if let Some(id_match) = cap.get(1) {
            let id = id_match.as_str().to_uppercase();
            let tag = format!("GTM: {}", id);
            if !profile.analytics.contains(&tag) {
                profile.analytics.push(tag.clone());
                profile.all_tags.push(tag);
            }
        }
    }

    for cap in VK_PIXEL_RE.captures_iter(html) {
        if let Some(id_match) = cap.get(1) {
            let id = id_match.as_str().to_uppercase();
            let tag = format!("VK Pixel: {}", id);
            if !profile.analytics.contains(&tag) {
                profile.analytics.push(tag.clone());
                profile.all_tags.push(tag);
            }
        }
    }

    // Server hints from headers
    if lower_headers.contains("nginx") && !profile.all_tags.contains(&"Сервер: Nginx".to_string())
    {
        profile.all_tags.push("Сервер: Nginx".to_string());
    } else if lower_headers.contains("apache")
        && !profile.all_tags.contains(&"Сервер: Apache".to_string())
    {
        profile.all_tags.push("Сервер: Apache".to_string());
    } else if lower_headers.contains("cloudflare")
        && !profile.all_tags.contains(&"CDN: Cloudflare".to_string())
    {
        profile.all_tags.push("CDN: Cloudflare".to_string());
    }

    // 5. Embedded ATS & Career Widget Detection
    if let Some(cap) = GH_RE.captures(html) {
        if let Some(slug_m) = cap.get(1) {
            let slug = slug_m.as_str().to_string();
            if slug != "embed" && slug != "js" && slug != "v1" {
                profile.ats = Some(("greenhouse".to_string(), slug.clone()));
                profile.all_tags.push(format!("ATS: Greenhouse ({})", slug));
            }
        }
    } else if let Some(cap) = ASHBY_RE.captures(html) {
        if let Some(slug_m) = cap.get(1) {
            let slug = slug_m.as_str().to_string();
            profile.ats = Some(("ashby".to_string(), slug.clone()));
            profile.all_tags.push(format!("ATS: Ashby ({})", slug));
        }
    } else if let Some(cap) = LEVER_RE.captures(html) {
        if let Some(slug_m) = cap.get(1) {
            let slug = slug_m.as_str().to_string();
            profile.ats = Some(("lever".to_string(), slug.clone()));
            profile.all_tags.push(format!("ATS: Lever ({})", slug));
        }
    } else if let Some(cap) = SR_RE.captures(html) {
        if let Some(slug_m) = cap.get(1) {
            let slug = slug_m.as_str().to_string();
            profile.ats = Some(("smartrecruiters".to_string(), slug.clone()));
            profile
                .all_tags
                .push(format!("ATS: SmartRecruiters ({})", slug));
        }
    } else if let Some(cap) = WORKABLE_RE.captures(html) {
        if let Some(slug_m) = cap.get(1) {
            let slug = slug_m.as_str().to_string();
            profile.ats = Some(("workable".to_string(), slug.clone()));
            profile.all_tags.push(format!("ATS: Workable ({})", slug));
        }
    } else if let Some(cap) = WORKDAY_RE.captures(html) {
        let tenant = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let site = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        profile.ats = Some(("workday".to_string(), format!("{}/{}", tenant, site)));
        profile.all_tags.push(format!("ATS: Workday ({})", tenant));
    } else if let Some(cap) = BAMBOO_RE.captures(html) {
        let slug = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        profile.ats = Some(("bamboohr".to_string(), slug.to_string()));
        profile.all_tags.push(format!("ATS: BambooHR ({})", slug));
    }

    // 6. Anti-Bot WAF & Challenge Protection
    if lower_html.contains("challenges.cloudflare.com/turnstile")
        || lower_html.contains("cf-turnstile")
    {
        profile
            .all_tags
            .push("WAF: Cloudflare Turnstile".to_string());
    } else if lower_html.contains("datadome.co") {
        profile.all_tags.push("WAF: DataDome".to_string());
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ats_detection() {
        let sample = r#"<html><body><script src="https://boards.greenhouse.io/embed/job_board/js?for=canonical"></script></body></html>"#;
        let p = detect_tech_stack(sample, "");
        assert_eq!(
            p.ats,
            Some(("greenhouse".to_string(), "canonical".to_string()))
        );
        assert!(p.all_tags.iter().any(|t| t.contains("Greenhouse")));
    }

    #[test]
    fn test_tech_detection() {
        let sample_html = r#"
            <html>
                <head>
                    <script src="/bitrix/js/main/core/core.js"></script>
                    <!-- Yandex.Metrika counter -->
                    <script>ym(98765432, "init", {});</script>
                    <script async src="https://www.googletagmanager.com/gtag/js?id=G-ABC1234567"></script>
                </head>
                <body>
                    <div class="b24-widget-button"></div>
                    <script src="//code.jivo.ru/widget/xyz123"></script>
                </body>
            </html>
        "#;

        let res = detect_tech_stack(sample_html, "Server: nginx");
        assert_eq!(res.cms, Some("1C-Битрикс".to_string()));
        assert_eq!(res.crm, Some("Битрикс24".to_string()));
        assert_eq!(res.live_chat, Some("JivoChat".to_string()));
        assert!(res.analytics.iter().any(|a| a.contains("98765432")));
        assert!(res.analytics.iter().any(|a| a.contains("G-ABC1234567")));
    }

    #[test]
    fn test_workday_and_hubspot_detection() {
        let sample = r#"<html><head><script src="//js.hs-scripts.com/12345.js"></script><script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script></head><body><a href="https://adobe.wd5.myworkdayjobs.com/external_experienced">Careers</a></body></html>"#;
        let p = detect_tech_stack(sample, "");
        assert_eq!(p.crm, Some("HubSpot".to_string()));
        assert_eq!(
            p.ats,
            Some((
                "workday".to_string(),
                "adobe/external_experienced".to_string()
            ))
        );
        assert!(
            p.all_tags
                .iter()
                .any(|t| t.contains("Cloudflare Turnstile"))
        );
    }
}
