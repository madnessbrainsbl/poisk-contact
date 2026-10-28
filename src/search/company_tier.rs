use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HeadcountTier {
    Micro,      // 1-10
    Smb,        // 11-50
    MidMarket,  // 51-250
    Enterprise, // 250+
    Unknown,
}

impl std::fmt::Display for HeadcountTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeadcountTier::Micro => write!(f, "Micro (1-10)"),
            HeadcountTier::Smb => write!(f, "SMB (11-50)"),
            HeadcountTier::MidMarket => write!(f, "Mid-Market (51-250)"),
            HeadcountTier::Enterprise => write!(f, "Enterprise (250+)"),
            HeadcountTier::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndustryVertical {
    Cybersecurity,
    Fintech,
    Edtech,
    Ecommerce,
    Saas,
    Logistics,
    Healthcare,
    Telecom,
    Manufacturing,
    Consulting,
    General,
}

impl std::fmt::Display for IndustryVertical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndustryVertical::Cybersecurity => write!(f, "Cybersecurity"),
            IndustryVertical::Fintech => write!(f, "Fintech / Banking"),
            IndustryVertical::Edtech => write!(f, "EdTech / E-Learning"),
            IndustryVertical::Ecommerce => write!(f, "E-Commerce / Retail"),
            IndustryVertical::Saas => write!(f, "SaaS / Cloud Software"),
            IndustryVertical::Logistics => write!(f, "Logistics & Supply Chain"),
            IndustryVertical::Healthcare => write!(f, "Healthcare / MedTech"),
            IndustryVertical::Telecom => write!(f, "Telecom & Networks"),
            IndustryVertical::Manufacturing => write!(f, "Manufacturing & Heavy Industry"),
            IndustryVertical::Consulting => write!(f, "Legal & Consulting"),
            IndustryVertical::General => write!(f, "General B2B"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyFirmographics {
    pub headcount_tier: HeadcountTier,
    pub industry: IndustryVertical,
    pub confidence: u8,
    pub signals: Vec<String>,
}

pub fn classify_company_from_html(html: &str) -> CompanyFirmographics {
    let lower = html.to_lowercase();
    let mut signals = Vec::new();
    let mut confidence: u8 = 40;

    // 1. Headcount Tier Detection
    let mut detected_tier = HeadcountTier::Unknown;

    // Enterprise signals (250+)
    let enterprise_patterns = [
        "более 1000 сотрудников",
        "более 500 сотрудников",
        "более 300 сотрудников",
        "over 1,000 employees",
        "over 500 employees",
        "over 1000 employees",
        "fortune 500",
        "международный холдинг",
        "публичное акционерное общество",
        "пао ",
        "глобальная корпорация",
        "global enterprise",
        "worldwide offices",
        "тысячи сотрудников",
        "тысяч специалистов",
    ];
    for pat in &enterprise_patterns {
        if lower.contains(pat) {
            detected_tier = HeadcountTier::Enterprise;
            signals.push(format!("Enterprise signal: {}", pat));
            confidence = confidence.saturating_add(25);
            break;
        }
    }

    // Mid-Market signals (51-250)
    if detected_tier == HeadcountTier::Unknown {
        let midmarket_patterns = [
            "более 100 специалистов",
            "более 100 человек",
            "более 50 специалистов",
            "более 50 сотрудников",
            "over 100 employees",
            "over 50 employees",
            "команда из 100",
            "команда из 70",
            "команда из 80",
            "команда из 150",
            "50-250 employees",
            "50+ engineers",
            "филиалы в 5 городах",
        ];
        for pat in &midmarket_patterns {
            if lower.contains(pat) {
                detected_tier = HeadcountTier::MidMarket;
                signals.push(format!("Mid-Market signal: {}", pat));
                confidence = confidence.saturating_add(20);
                break;
            }
        }
    }

    // SMB signals (11-50)
    if detected_tier == HeadcountTier::Unknown {
        let smb_patterns = [
            "команда из 15",
            "команда из 20",
            "команда из 25",
            "команда из 30",
            "команда из 40",
            "команда из 12",
            "команда из 35",
            "11-50 employees",
            "15-50 сотрудников",
            "10-50 специалистов",
            "growing startup",
            "растущий стартап",
            "небольшая сплоченная команда",
        ];
        for pat in &smb_patterns {
            if lower.contains(pat) {
                detected_tier = HeadcountTier::Smb;
                signals.push(format!("SMB signal: {}", pat));
                confidence = confidence.saturating_add(20);
                break;
            }
        }
    }

    // Micro signals (1-10)
    if detected_tier == HeadcountTier::Unknown {
        let micro_patterns = [
            "частный специалист",
            "семейный бизнес",
            "бутик-агентство",
            "boutique agency",
            "solo founder",
            "команда из 3",
            "команда из 4",
            "команда из 5",
            "индивидуальный предприниматель",
            "частная практика",
            "1-10 employees",
        ];
        for pat in &micro_patterns {
            if lower.contains(pat) {
                detected_tier = HeadcountTier::Micro;
                signals.push(format!("Micro signal: {}", pat));
                confidence = confidence.saturating_add(15);
                break;
            }
        }
    }

    // Fallback: search regex for employee numbers
    if detected_tier == HeadcountTier::Unknown {
        let re_emp = Regex::new(r"(?i)(?:штат|команда|коллектив|сотрудников|специалистов|employees)\s*(?:из|составляет|более|около|:)?\s*(\d{1,5})").ok();
        if let Some(re) = re_emp {
            if let Some(caps) = re.captures(&lower) {
                if let Some(m) = caps.get(1) {
                    if let Ok(count) = m.as_str().parse::<u32>() {
                        if count >= 250 {
                            detected_tier = HeadcountTier::Enterprise;
                            signals.push(format!("Regex parsed employees: {}", count));
                            confidence = confidence.saturating_add(20);
                        } else if count >= 51 {
                            detected_tier = HeadcountTier::MidMarket;
                            signals.push(format!("Regex parsed employees: {}", count));
                            confidence = confidence.saturating_add(20);
                        } else if count >= 11 {
                            detected_tier = HeadcountTier::Smb;
                            signals.push(format!("Regex parsed employees: {}", count));
                            confidence = confidence.saturating_add(20);
                        } else if count >= 1 {
                            detected_tier = HeadcountTier::Micro;
                            signals.push(format!("Regex parsed employees: {}", count));
                            confidence = confidence.saturating_add(15);
                        }
                    }
                }
            }
        }
    }

    // 2. Industry Vertical Classification
    let mut detected_industry = IndustryVertical::General;

    // Prioritized keyword scoring
    let cyber_kw = [
        "кибербезопасн",
        "информационн безопасность",
        "soc",
        "siem",
        "пентест",
        "pentest",
        "vulnerability",
        "уязвимост",
        "cybersecurity",
        "threat intelligence",
        "waf",
        "антивирус",
        "infosec",
    ];
    let fintech_kw = [
        "эквайринг",
        "платежн",
        "банкинг",
        "fintech",
        "финтех",
        "кредитован",
        "инвестиции",
        "брокер",
        "депозит",
        "crypto",
        "blockchain",
        "payment gateway",
    ];
    let edtech_kw = [
        "онлайн-курс",
        "edtech",
        "обучение онлайн",
        "вебинар",
        "платформа обучения",
        "школа программирования",
        "lms",
        "курс повышения квалификации",
    ];
    let ecom_kw = [
        "интернет-магазин",
        "корзина покупок",
        "оформить заказ",
        "каталог товаров",
        "доставка курьером",
        "ecommerce",
        "e-commerce",
        "маркетплейс",
        "карточка товара",
    ];
    let saas_kw = [
        "b2b saas",
        "облачная платформа",
        "облачный сервис",
        "saas platform",
        "api integration",
        "crm-система",
        "подписка на сервис",
        "тарифные планы",
        "software as a service",
    ];
    let log_kw = [
        "грузоперевозк",
        "логистик",
        "складское хранение",
        "доставка грузов",
        "фрахт",
        "таможенное оформление",
        "supply chain",
        "флот",
    ];
    let health_kw = [
        "клиника",
        "медицинск",
        "врач",
        "диагностика",
        "фармацевт",
        "телемедицина",
        "пациент",
        "стоматолог",
        "лечение",
    ];
    let telecom_kw = [
        "телеком",
        "провайдер интернета",
        "оптоволокно",
        "сотовая связь",
        "сим-карт",
        "voip телефония",
        "магистральный канал",
    ];
    let mfg_kw = [
        "производство",
        "завод",
        "промышленност",
        "станочн",
        "металлопрокат",
        "цех",
        "оборудование для производства",
        "heavy industry",
    ];
    let consult_kw = [
        "юридические услуги",
        "бухгалтерское сопровождение",
        "аудит компании",
        "налоговый консалтинг",
        "адвокат",
        "юрист",
    ];

    let count_matches =
        |keywords: &[&str]| -> usize { keywords.iter().filter(|k| lower.contains(*k)).count() };

    let cyber_count = count_matches(&cyber_kw);
    let fintech_count = count_matches(&fintech_kw);
    let edtech_count = count_matches(&edtech_kw);
    let ecom_count = count_matches(&ecom_kw);
    let saas_count = count_matches(&saas_kw);
    let log_count = count_matches(&log_kw);
    let health_count = count_matches(&health_kw);
    let telecom_count = count_matches(&telecom_kw);
    let mfg_count = count_matches(&mfg_kw);
    let consult_count = count_matches(&consult_kw);

    let max_count = [
        cyber_count,
        fintech_count,
        edtech_count,
        ecom_count,
        saas_count,
        log_count,
        health_count,
        telecom_count,
        mfg_count,
        consult_count,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    if max_count > 0 {
        if max_count == cyber_count {
            detected_industry = IndustryVertical::Cybersecurity;
            signals.push(format!("Matched {} cybersecurity keywords", cyber_count));
        } else if max_count == fintech_count {
            detected_industry = IndustryVertical::Fintech;
            signals.push(format!("Matched {} fintech keywords", fintech_count));
        } else if max_count == edtech_count {
            detected_industry = IndustryVertical::Edtech;
            signals.push(format!("Matched {} edtech keywords", edtech_count));
        } else if max_count == ecom_count {
            detected_industry = IndustryVertical::Ecommerce;
            signals.push(format!("Matched {} ecommerce keywords", ecom_count));
        } else if max_count == saas_count {
            detected_industry = IndustryVertical::Saas;
            signals.push(format!("Matched {} saas keywords", saas_count));
        } else if max_count == log_count {
            detected_industry = IndustryVertical::Logistics;
            signals.push(format!("Matched {} logistics keywords", log_count));
        } else if max_count == health_count {
            detected_industry = IndustryVertical::Healthcare;
            signals.push(format!("Matched {} healthcare keywords", health_count));
        } else if max_count == telecom_count {
            detected_industry = IndustryVertical::Telecom;
            signals.push(format!("Matched {} telecom keywords", telecom_count));
        } else if max_count == mfg_count {
            detected_industry = IndustryVertical::Manufacturing;
            signals.push(format!("Matched {} manufacturing keywords", mfg_count));
        } else if max_count == consult_count {
            detected_industry = IndustryVertical::Consulting;
            signals.push(format!("Matched {} consulting keywords", consult_count));
        }
        confidence = confidence.saturating_add(25);
    }

    if confidence > 95 {
        confidence = 95;
    }

    CompanyFirmographics {
        headcount_tier: detected_tier,
        industry: detected_industry,
        confidence,
        signals,
    }
}

pub async fn classify_company_from_url(
    client: &Client,
    url: &str,
) -> Result<CompanyFirmographics, String> {
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
        .map_err(|e| format!("Network request failed: {}", e))?;

    let html = resp.text().await.unwrap_or_default();
    Ok(classify_company_from_html(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_fintech() {
        let html = r#"
            <html>
                <head><title>ПАО Банк Финанс - Эквайринг и инвестиции</title></head>
                <body>
                    <h1>Глобальный банкинг</h1>
                    <p>Более 1000 сотрудников работают в филиалах по всей стране.</p>
                    <p>Платежный шлюз и торговый эквайринг для крупного бизнеса.</p>
                </body>
            </html>
        "#;
        let firmo = classify_company_from_html(html);
        assert_eq!(firmo.headcount_tier, HeadcountTier::Enterprise);
        assert_eq!(firmo.industry, IndustryVertical::Fintech);
        assert!(firmo.confidence >= 80);
    }

    #[test]
    fn test_smb_cybersecurity() {
        let html = r#"
            <html>
                <body>
                    <h1>Центр кибербезопасности</h1>
                    <p>Наша команда из 25 экспертов проводит пентест и аудит уязвимостей.</p>
                    <p>Внедрение SOC и защита от кибератак.</p>
                </body>
            </html>
        "#;
        let firmo = classify_company_from_html(html);
        assert_eq!(firmo.headcount_tier, HeadcountTier::Smb);
        assert_eq!(firmo.industry, IndustryVertical::Cybersecurity);
    }

    #[test]
    fn test_micro_consulting() {
        let html = r#"
            <html>
                <body>
                    <h1>Адвокатский кабинет</h1>
                    <p>Бутик-агентство и юридические услуги для предпринимателей.</p>
                    <p>Налоговый консалтинг и аудит компании от частного специалиста.</p>
                </body>
            </html>
        "#;
        let firmo = classify_company_from_html(html);
        assert_eq!(firmo.headcount_tier, HeadcountTier::Micro);
        assert_eq!(firmo.industry, IndustryVertical::Consulting);
    }

    #[test]
    fn test_midmarket_ecommerce() {
        let html = r#"
            <html>
                <body>
                    <h1>Федеральный интернет-магазин техники</h1>
                    <p>Команда из 100 человек обеспечивает доставку заказов по всей стране.</p>
                    <p>Удобная корзина покупок, каталог товаров и маркетплейс электроники.</p>
                </body>
            </html>
        "#;
        let firmo = classify_company_from_html(html);
        assert_eq!(firmo.headcount_tier, HeadcountTier::MidMarket);
        assert_eq!(firmo.industry, IndustryVertical::Ecommerce);
    }
}
