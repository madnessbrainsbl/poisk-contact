use crate::models::{ComplianceProfile, ContactRecord};
use reqwest::Client;
use std::time::Duration;

const PRIVACY_PATHS: &[&str] = &[
    "/privacy",
    "/privacy-policy",
    "/policy",
    "/politika-konfidencialnosti",
    "/politika",
    "/personal-data",
    "/soglasie-na-obrabotku-dannyh",
    "/terms",
    "/cookie",
    "/cookies",
    "/legal",
];

const FZ_152_MARKERS: &[&str] = &[
    "обработка персональных данных",
    "оператор персональных данных",
    "152-фз",
    "согласие на обработку",
    "субъект персональных данных",
    "роскомнадзор",
    "обработку персональных данных",
];

const GDPR_MARKERS: &[&str] = &[
    "general data protection regulation",
    "gdpr",
    "data protection officer",
    "article 6",
    "regulation (eu) 2016/679",
    "supervisory authority",
    "controller and processor",
];

const COOKIE_MARKERS: &[&str] = &[
    "cookie",
    "файлы cookie",
    "куки",
    "cookie policy",
    "мы используем cookie",
    "политика использования cookie",
];

pub fn analyze_compliance_text(text: &str, page_url: &str) -> ComplianceProfile {
    let lower = text.to_lowercase();
    let mut profile = ComplianceProfile {
        url: page_url.to_string(),
        privacy_policy_url: page_url.to_string(),
        ..Default::default()
    };

    let mut marker_count_fz = 0;
    for marker in FZ_152_MARKERS {
        if lower.contains(marker) {
            marker_count_fz += 1;
            profile.detected_markers.push(format!("152-ФЗ: {}", marker));
        }
    }

    let mut marker_count_gdpr = 0;
    for marker in GDPR_MARKERS {
        if lower.contains(marker) {
            marker_count_gdpr += 1;
            profile.detected_markers.push(format!("GDPR: {}", marker));
        }
    }

    let mut marker_count_cookie = 0;
    for marker in COOKIE_MARKERS {
        if lower.contains(marker) {
            marker_count_cookie += 1;
            profile.detected_markers.push(format!("Cookie: {}", marker));
        }
    }

    profile.law_152_fz_compliant = marker_count_fz >= 2;
    profile.gdpr_compliant = marker_count_gdpr >= 2;
    profile.has_cookie_notice = marker_count_cookie >= 1;
    profile.has_privacy_policy = (profile.law_152_fz_compliant
        || profile.gdpr_compliant
        || lower.contains("privacy")
        || lower.contains("конфиденциальност"))
        && text.chars().count() >= 50;

    let mut score: u8 = 0;
    if profile.has_privacy_policy {
        score += 40;
    }
    if profile.law_152_fz_compliant {
        score += 30;
    }
    if profile.gdpr_compliant {
        score += 20;
    }
    if profile.has_cookie_notice {
        score += 10;
    }

    profile.compliance_score = score.min(100);

    profile.compliance_status = if profile.compliance_score >= 80 {
        "Высокий комплаенс (152-ФЗ / GDPR)".to_string()
    } else if profile.compliance_score >= 40 {
        "Частичный комплаенс".to_string()
    } else {
        "Низкий комплаенс / Риск".to_string()
    };

    if !profile.has_privacy_policy {
        profile.recommendations.push(
            "На сайте не обнаружен опубликованный документ Политики конфиденциальности."
                .to_string(),
        );
    }
    if !profile.law_152_fz_compliant && !profile.gdpr_compliant {
        profile.recommendations.push("Отсутствуют обязательные формулировки об операторе и согласии на обработку персональных данных (152-ФЗ / GDPR).".to_string());
    }
    if !profile.has_cookie_notice {
        profile.recommendations.push(
            "Не обнаружено явное уведомление пользователей об использовании cookie-файлов."
                .to_string(),
        );
    }

    profile
}

pub async fn audit_website_compliance(client: &Client, raw_target: &str) -> ComplianceProfile {
    let base_url = if raw_target.starts_with("http://") || raw_target.starts_with("https://") {
        raw_target.trim().trim_end_matches('/').to_string()
    } else {
        format!("https://{}", raw_target.trim().trim_end_matches('/'))
    };

    let mut best_profile = ComplianceProfile {
        url: base_url.clone(),
        ..Default::default()
    };

    // 1. First probe root page for in-page policy and cookie markers
    if let Ok(resp) = client
        .get(&base_url)
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(html) = resp.text().await {
                best_profile = analyze_compliance_text(&html, &base_url);
            }
        }
    }

    if best_profile.compliance_score >= 80 {
        return best_profile;
    }

    // 2. Probe dedicated privacy policy paths
    for path in PRIVACY_PATHS {
        let probe_url = format!("{}{}", base_url, path);
        if let Ok(resp) = client
            .get(&probe_url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(html) = resp.text().await {
                    let page_profile = analyze_compliance_text(&html, &probe_url);
                    if page_profile.compliance_score > best_profile.compliance_score {
                        best_profile = page_profile;
                    }
                    if best_profile.compliance_score >= 70 {
                        break;
                    }
                }
            }
        }
    }

    if best_profile.compliance_score == 0 {
        best_profile.compliance_status = "Политика не найдена".to_string();
        best_profile.recommendations.push(format!(
            "Страницы комплаенса по адресу {} не найдены. Холодная рассылка может нести регуляторные риски.",
            base_url
        ));
    }

    best_profile
}

pub fn apply_compliance_to_contact(profile: &ComplianceProfile, contact: &mut ContactRecord) {
    contact.compliance_score = profile.compliance_score;
    contact.compliance_status = profile.compliance_status.clone();
    if !profile.privacy_policy_url.is_empty() {
        contact.privacy_policy_url = profile.privacy_policy_url.clone();
    }
    if profile.compliance_score >= 70 {
        let tag = "[Compliance OK]".to_string();
        if !contact.tags.contains(&tag) {
            contact.tags.push(tag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_152_fz_markers() {
        let text = "Политика в отношении обработки персональных данных. Настоящий документ определяет порядок, в котором оператор персональных данных осуществляет обработку персональных данных в соответствии с Федеральным законом 152-ФЗ. Субъект персональных данных подтверждает свое согласие на обработку.";
        let profile = analyze_compliance_text(text, "https://example.ru/privacy");
        assert!(profile.law_152_fz_compliant);
        assert!(profile.has_privacy_policy);
        assert!(profile.compliance_score >= 70);
    }

    #[test]
    fn test_gdpr_markers() {
        let text = "This Privacy Notice describes how we collect personal data pursuant to the General Data Protection Regulation (GDPR) Regulation (EU) 2016/679. The Data Protection Officer can be reached at dpo@example.com for Article 6 rights.";
        let profile = analyze_compliance_text(text, "https://example.com/privacy");
        assert!(profile.gdpr_compliant);
        assert!(profile.has_privacy_policy);
        assert!(profile.compliance_score >= 60);
    }

    #[test]
    fn test_empty_html() {
        let profile = analyze_compliance_text("", "https://empty.com");
        assert!(!profile.has_privacy_policy);
        assert_eq!(profile.compliance_score, 0);
    }
}
