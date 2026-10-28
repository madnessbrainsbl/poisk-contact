use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechMaturityProfile {
    pub score: u8,
    pub tier: String,
    pub infra_signals: Vec<String>,
    pub frontend_signals: Vec<String>,
    pub crm_signals: Vec<String>,
    pub analytics_signals: Vec<String>,
    pub ats_signals: Vec<String>,
    pub sales_readiness_verdict: String,
}

pub fn evaluate_tech_maturity_from_html(html: &str) -> TechMaturityProfile {
    let lower = html.to_lowercase();

    let mut infra_signals = Vec::new();
    let mut frontend_signals = Vec::new();
    let mut crm_signals = Vec::new();
    let mut analytics_signals = Vec::new();
    let mut ats_signals = Vec::new();

    // 1. Cloud & CDN Infrastructure (Max 20 pts)
    if lower.contains("cloudflare")
        || lower.contains("cf-ray")
        || lower.contains("cloudflareinsights")
    {
        infra_signals.push("Cloudflare Edge CDN".to_string());
    }
    if lower.contains("amazonaws.com") || lower.contains("aws") || lower.contains("cloudfront") {
        infra_signals.push("Amazon Web Services (AWS)".to_string());
    }
    if lower.contains("azure") || lower.contains("windows.net") {
        infra_signals.push("Microsoft Azure".to_string());
    }
    if lower.contains("selectel") {
        infra_signals.push("Selectel Cloud Infrastructure".to_string());
    }
    let infra_score: u8 = if !infra_signals.is_empty() { 20 } else { 0 };

    // 2. Modern Frontend & CMS (Max 20 pts)
    if lower.contains("__next") || lower.contains("next.js") {
        frontend_signals.push("Next.js / React Modern Framework".to_string());
    } else if lower.contains("react") || lower.contains("data-reactroot") {
        frontend_signals.push("React.js SPA".to_string());
    }
    if lower.contains("vue") || lower.contains("nuxt") {
        frontend_signals.push("Vue.js / Nuxt Ecosystem".to_string());
    }
    if lower.contains("tilda") || lower.contains("tildacdn") {
        frontend_signals.push("Tilda Publishing Platform".to_string());
    }
    if lower.contains("bitrix") || lower.contains("bitrix24") {
        frontend_signals.push("1C-Bitrix Framework".to_string());
    }
    if lower.contains("wp-content") || lower.contains("wordpress") {
        frontend_signals.push("WordPress CMS".to_string());
    }
    let frontend_score: u8 = if !frontend_signals.is_empty() { 20 } else { 0 };

    // 3. CRM & Sales Operations (Max 25 pts)
    if lower.contains("amocrm") || lower.contains("amo-crm") {
        crm_signals.push("AmoCRM Sales Pipeline".to_string());
    }
    if lower.contains("b24-form") || lower.contains("bitrix24.ru") || lower.contains("bitrix24.com")
    {
        crm_signals.push("Bitrix24 Enterprise CRM".to_string());
    }
    if lower.contains("hubspot") || lower.contains("hs-scripts") {
        crm_signals.push("HubSpot Inbound CRM".to_string());
    }
    if lower.contains("salesforce") {
        crm_signals.push("Salesforce Enterprise CRM".to_string());
    }
    let crm_score: u8 = if !crm_signals.is_empty() { 25 } else { 0 };

    // 4. Marketing Automation & Pixels (Max 20 pts)
    if lower.contains("mindbox") {
        analytics_signals.push("Mindbox CDP & Marketing Automation".to_string());
    }
    if lower.contains("carrotquest") || lower.contains("carrot quest") {
        analytics_signals.push("Carrot Quest Lead Generation".to_string());
    }
    if lower.contains("ym(") || lower.contains("metrika") || lower.contains("mc.yandex.ru") {
        analytics_signals.push("Yandex Metrika Analytics".to_string());
    }
    if lower.contains("googletagmanager")
        || lower.contains("gtag(")
        || lower.contains("google-analytics")
    {
        analytics_signals.push("Google Analytics 4 / GTM".to_string());
    }
    if lower.contains("top-fwz1.mail.ru")
        || lower.contains("vk-counter")
        || lower.contains("vk.com/rtrg")
    {
        analytics_signals.push("VK / Top Mail.ru Pixel".to_string());
    }
    let analytics_score: u8 = if !analytics_signals.is_empty() { 20 } else { 0 };

    // 5. Talent Acquisition & ATS (Max 15 pts)
    if lower.contains("huntflow") {
        ats_signals.push("Huntflow ATS Integration".to_string());
    }
    if lower.contains("hh.ru/employer") || lower.contains("hh.ru") && lower.contains("ваканс")
    {
        ats_signals.push("HeadHunter Active Recruiting".to_string());
    }
    if lower.contains("greenhouse") || lower.contains("workday") || lower.contains("lever.co") {
        ats_signals.push("Enterprise ATS (Greenhouse / Workday / Lever)".to_string());
    }
    let ats_score: u8 = if !ats_signals.is_empty() { 15 } else { 0 };

    let total_score =
        (infra_score + frontend_score + crm_score + analytics_score + ats_score).min(100);

    let (tier, verdict) = if total_score >= 70 {
        (
            "Tier 1: Tech-Forward Enterprise".to_string(),
            "High digital maturity: Comprehensive tech adoption with active CRM, analytics automation, and modern infrastructure. Prime ICP target with allocated software budget.".to_string(),
        )
    } else if total_score >= 40 {
        (
            "Tier 2: Digitally Active Growth".to_string(),
            "Moderate digital maturity: Established digital presence with core web and marketing tracking. Receptive to targeted productivity and automation solutions.".to_string(),
        )
    } else {
        (
            "Tier 3: Legacy / Minimal Digital Stack".to_string(),
            "Low digital maturity: Minimal or legacy tech footprint. Focus sales conversations on foundational modernization and cost savings.".to_string(),
        )
    };

    TechMaturityProfile {
        score: total_score,
        tier,
        infra_signals,
        frontend_signals,
        crm_signals,
        analytics_signals,
        ats_signals,
        sales_readiness_verdict: verdict,
    }
}

pub async fn evaluate_tech_maturity_from_url(
    client: &Client,
    url: &str,
) -> Result<TechMaturityProfile, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    let html = resp.text().await.unwrap_or_default();
    Ok(evaluate_tech_maturity_from_html(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_full_stack_maturity() {
        let html = r#"
            <html>
                <head>
                    <script src="https://challenges.cloudflare.com/turnstile/v0/api.js"></script>
                    <script src="https://js.hs-scripts.com/12345.js"></script>
                    <script src="https://api.mindbox.ru/scripts/v1/tracker.js"></script>
                </head>
                <body>
                    <div id="__next">Enterprise Platform</div>
                    <a href="https://huntflow.ru/apply">Apply via Huntflow</a>
                </body>
            </html>
        "#;
        let prof = evaluate_tech_maturity_from_html(html);
        assert!(prof.score >= 80, "Expected score >= 80, got {}", prof.score);
        assert_eq!(prof.tier, "Tier 1: Tech-Forward Enterprise");
        assert!(!prof.infra_signals.is_empty());
        assert!(!prof.crm_signals.is_empty());
        assert!(!prof.analytics_signals.is_empty());
    }

    #[test]
    fn test_midmarket_growth_maturity() {
        let html = r#"
            <html>
                <head>
                    <script src="https://mc.yandex.ru/metrika/tag.js"></script>
                </head>
                <body>
                    <div class="tildacdn">Tilda Landing</div>
                    <form action="https://amocrm.ru/forms/123"></form>
                </body>
            </html>
        "#;
        let prof = evaluate_tech_maturity_from_html(html);
        assert!(
            prof.score >= 40 && prof.score < 80,
            "Score was {}",
            prof.score
        );
        assert_eq!(prof.tier, "Tier 2: Digitally Active Growth");
    }

    #[test]
    fn test_legacy_minimal_maturity() {
        let html = "<html><body><h1>Local Bakery</h1><p>Call us by phone.</p></body></html>";
        let prof = evaluate_tech_maturity_from_html(html);
        assert_eq!(prof.score, 0);
        assert_eq!(prof.tier, "Tier 3: Legacy / Minimal Digital Stack");
    }
}
