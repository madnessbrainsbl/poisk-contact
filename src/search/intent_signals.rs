use crate::models::MarketingIntentProfile;

pub fn detect_marketing_intent(html: &str) -> MarketingIntentProfile {
    let lower = html.to_lowercase();
    let mut ad_pixels = Vec::new();
    let mut automation_platforms = Vec::new();

    // 1. Paid Advertising Tracking Pixels (AdTech)
    if lower.contains("reachgoal")
        || lower.contains("yandex.ru/metrika")
        || lower.contains("mc.yandex.ru")
    {
        ad_pixels.push("Яндекс Директ / Метрика Цели".to_string());
    }
    if lower.contains("vk-pixel") || lower.contains("vk.com/rtrg") || lower.contains("vk-rtrg") {
        ad_pixels.push("VK Реклама (VK Pixel)".to_string());
    }
    if lower.contains("gtag('config', 'aw-")
        || lower.contains("googleads")
        || lower.contains("adservices.google")
    {
        ad_pixels.push("Google Ads Conversion Tracking".to_string());
    }
    if lower.contains("fbq('init'") || lower.contains("fbevents.js") {
        ad_pixels.push("Meta Pixel (Facebook/Instagram)".to_string());
    }
    if lower.contains("analytics.tiktok.com") || lower.contains("ttq.load") {
        ad_pixels.push("TikTok Pixel".to_string());
    }
    if lower.contains("snap.licdn.com") || lower.contains("linkedin insight") {
        ad_pixels.push("LinkedIn Insight Tag".to_string());
    }

    // 2. Marketing Automation & CRM ESP Systems
    if lower.contains("hs-scripts.com") || lower.contains("js.hs-scripts.com") {
        automation_platforms.push("HubSpot Marketing Automation".to_string());
    }
    if lower.contains("mindbox.js") || lower.contains("api.mindbox.ru") {
        automation_platforms.push("Mindbox CDP / Automation".to_string());
    }
    if lower.contains("sendpulse.com") || lower.contains("cdn.sendpulse.com") {
        automation_platforms.push("SendPulse ESP".to_string());
    }
    if lower.contains("unisender.com") || lower.contains("popup.unisender.com") {
        automation_platforms.push("UniSender ESP".to_string());
    }
    if lower.contains("trackcmp.net") || lower.contains("activecampaign.com") {
        automation_platforms.push("ActiveCampaign".to_string());
    }
    if lower.contains("chimp.js") || lower.contains("mailchimp.com") {
        automation_platforms.push("Mailchimp".to_string());
    }

    let mut score = 0u8;
    score = score.saturating_add((ad_pixels.len() * 20).min(50) as u8);
    score = score.saturating_add((automation_platforms.len() * 25).min(50) as u8);

    let is_high_intent = score >= 40;

    MarketingIntentProfile {
        ad_pixels,
        automation_platforms,
        intent_score: score,
        is_high_intent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yandex_and_vk_pixel_intent() {
        let html = r#"
            <script>
                ym(123456, 'reachGoal', 'lead_submit');
                VK.Retargeting.Init("VK-RTRG-12345-abcde");
            </script>
        "#;
        let prof = detect_marketing_intent(html);
        assert!(prof.is_high_intent);
        assert!(prof.ad_pixels.iter().any(|p| p.contains("Яндекс")));
        assert!(prof.ad_pixels.iter().any(|p| p.contains("VK")));
        assert_eq!(prof.intent_score, 40);
    }

    #[test]
    fn test_hubspot_and_mindbox_automation() {
        let html = r#"
            <script src="https://js.hs-scripts.com/123456.js"></script>
            <script src="https://api.mindbox.ru/scripts/v1/tracker.js"></script>
        "#;
        let prof = detect_marketing_intent(html);
        assert!(prof.is_high_intent);
        assert!(
            prof.automation_platforms
                .iter()
                .any(|p| p.contains("HubSpot"))
        );
        assert!(
            prof.automation_platforms
                .iter()
                .any(|p| p.contains("Mindbox"))
        );
        assert_eq!(prof.intent_score, 50);
    }
}
