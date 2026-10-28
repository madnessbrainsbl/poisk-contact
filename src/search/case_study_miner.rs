use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialProofProfile {
    pub featured_clients: Vec<String>,
    pub certifications: Vec<String>,
    pub has_case_studies: bool,
    pub has_testimonials: bool,
    pub b2b_trust_score: u8,
    pub personalized_hook: String,
}

pub fn mine_social_proof_from_html(html: &str) -> SocialProofProfile {
    let lower = html.to_lowercase();
    let mut featured_clients = Vec::new();
    let mut certifications = Vec::new();

    // 1. Client brand detection
    let client_markers = [
        ("Сбер", &["сбербанк", "сбер", "sberbank"][..]),
        ("Яндекс", &["яндекс", "yandex"][..]),
        ("Ozon", &["ozon", "озон"][..]),
        ("VK", &["вконтакте", "vkontakte", "vk group"][..]),
        ("Т-Банк / Тинькофф", &["тинькофф", "т-банк", "tinkoff"][..]),
        ("Газпром", &["газпром", "gazprom"][..]),
        ("Ростелеком", &["ростелеком", "rostelecom"][..]),
        ("МТС", &["мтс", "mts.ru"][..]),
        ("МегаФон", &["мегафон", "megafon"][..]),
        ("Авито", &["авито", "avito"][..]),
        ("Альфа-Банк", &["альфа-банк", "alfabank"][..]),
        ("ВкусВилл", &["вкусвилл", "vkusvill"][..]),
        ("X5 Group", &["x5 group", "пятерочка", "перекресток"][..]),
        ("Google", &["google", "google cloud"][..]),
        ("Microsoft", &["microsoft", "azure"][..]),
    ];

    // Only detect client names if within client/partner/case-study context
    let client_section_markers = [
        "нам доверяют",
        "наши клиенты",
        "среди наших клиентов",
        "клиенты",
        "партнеры",
        "кейсы",
        "истории успеха",
        "отзывы",
        "trusted by",
        "our clients",
        "case studies",
        "featured customers",
        "companies we work with",
    ];

    let has_client_section = client_section_markers.iter().any(|m| lower.contains(m));

    if has_client_section {
        for (name, aliases) in &client_markers {
            if aliases.iter().any(|a| lower.contains(a)) {
                featured_clients.push(name.to_string());
                if featured_clients.len() >= 6 {
                    break;
                }
            }
        }
    }

    // 2. Certifications & Accreditations detection
    let cert_rules = [
        (
            "Лицензия ФСТЭК России",
            &["лицензия фстэк", "фстэк россии", "фстэк"][..],
        ),
        ("Лицензия ФСБ России", &["лицензия фсб", "фсб россии"][..]),
        (
            "Резидент Сколково",
            &["резидент сколково", "фонд сколково", "skolkovo resident"][..],
        ),
        (
            "Реестр отечественного ПО",
            &[
                "реестр отечественного по",
                "реестр минцифры",
                "единый реестр российских программ",
            ][..],
        ),
        (
            "Аккредитованная IT-компания",
            &["аккредитованная ит-компания", "аккредитованная it-компания"][..],
        ),
        ("ISO/IEC 27001", &["iso 27001", "iso/iec 27001"][..]),
        ("ISO 9001", &["iso 9001", "сертификат iso 9001"][..]),
        ("SOC 2 Type II", &["soc 2", "soc 2 type ii", "soc2"][..]),
        ("PCI DSS", &["pci dss", "pci-dss"][..]),
        ("ГОСТ Р ИСО", &["гост р исо", "гост р"][..]),
    ];

    for (cert_name, aliases) in &cert_rules {
        if aliases.iter().any(|a| lower.contains(a)) {
            certifications.push(cert_name.to_string());
        }
    }

    // 3. Structural markers
    let has_case_studies = lower.contains("кейсы")
        || lower.contains("case studies")
        || lower.contains("истории успеха")
        || lower.contains("наши проекты");

    let has_testimonials = lower.contains("отзывы")
        || lower.contains("testimonials")
        || lower.contains("рекомендации клиентов")
        || lower.contains("клиенты говорят");

    // 4. Calculate B2B Trust Score
    let mut trust_score: u8 = 30;
    if has_client_section {
        trust_score = trust_score.saturating_add(15);
    }
    if !featured_clients.is_empty() {
        trust_score = trust_score.saturating_add(20);
    }
    if !certifications.is_empty() {
        trust_score = trust_score.saturating_add(25);
    }
    if has_case_studies {
        trust_score = trust_score.saturating_add(10);
    }
    if has_testimonials {
        trust_score = trust_score.saturating_add(10);
    }

    if trust_score > 95 {
        trust_score = 95;
    }

    // 5. Generate Personalized Outreach Hook
    let personalized_hook = if !featured_clients.is_empty() && !certifications.is_empty() {
        format!(
            "Заметил ваше сотрудничество с {} и статус '{}' — впечатляющий уровень доверия в отрасли.",
            featured_clients[0], certifications[0]
        )
    } else if !featured_clients.is_empty() {
        format!(
            "Обратил внимание на блок клиентов — отличное портфолио сотрудничества с {}.",
            featured_clients.join(", ")
        )
    } else if !certifications.is_empty() {
        format!(
            "Отметил подтвержденную экспертизу компании ('{}') — серьезный фундамент для B2B партнерства.",
            certifications[0]
        )
    } else if has_case_studies {
        "Изучил ваши опубликованные кейсы и проекты — сильный практический подход к задачам клиентов.".to_string()
    } else {
        "".to_string()
    };

    SocialProofProfile {
        featured_clients,
        certifications,
        has_case_studies,
        has_testimonials,
        b2b_trust_score: trust_score,
        personalized_hook,
    }
}

pub async fn mine_social_proof_from_url(
    client: &Client,
    url: &str,
) -> Result<SocialProofProfile, String> {
    let target = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    let resp = client
        .get(&target)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let html = resp.text().await.unwrap_or_default();
    Ok(mine_social_proof_from_html(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_proof_with_clients_and_fstek() {
        let html = r#"
            <html>
                <body>
                    <h1>Информационная безопасность</h1>
                    <div class="partners">
                        <h2>Наши клиенты</h2>
                        <p>Нам доверяют: Сбербанк, Ozon и Яндекс.</p>
                    </div>
                    <footer>
                        <p>Лицензия ФСТЭК России на деятельность по техзащите информации.</p>
                        <p>Включены в Реестр отечественного ПО.</p>
                    </footer>
                </body>
            </html>
        "#;
        let prof = mine_social_proof_from_html(html);
        assert!(prof.featured_clients.contains(&"Сбер".to_string()));
        assert!(prof.featured_clients.contains(&"Ozon".to_string()));
        assert!(prof.featured_clients.contains(&"Яндекс".to_string()));
        assert!(prof.certifications.iter().any(|c| c.contains("ФСТЭК")));
        assert!(prof.certifications.iter().any(|c| c.contains("Реестр")));
        assert!(prof.b2b_trust_score >= 80);
        assert!(prof.personalized_hook.contains("Сбер"));
    }

    #[test]
    fn test_social_proof_with_skolkovo_and_iso() {
        let html = r#"
            <html>
                <body>
                    <p>Резидент Сколково с 2021 года.</p>
                    <p>Сертифицированы по стандарту ISO 27001.</p>
                    <div class="cases">Кейсы и проекты команды</div>
                </body>
            </html>
        "#;
        let prof = mine_social_proof_from_html(html);
        assert!(prof.certifications.iter().any(|c| c.contains("Сколково")));
        assert!(prof.certifications.iter().any(|c| c.contains("ISO")));
        assert!(prof.has_case_studies);
        assert!(prof.b2b_trust_score >= 60);
    }

    #[test]
    fn test_minimal_site_fallback() {
        let html = "<html><body>Простой сайт без отзывов и логотипов.</body></html>";
        let prof = mine_social_proof_from_html(html);
        assert!(prof.featured_clients.is_empty());
        assert!(prof.certifications.is_empty());
        assert_eq!(prof.b2b_trust_score, 30);
    }
}
