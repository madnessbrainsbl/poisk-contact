use crate::models::{ContactRecord, LeadScore};
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct CareerSignalInfo {
    pub is_hiring: bool,
    pub detected_roles: Vec<String>,
    pub signal_triggers: Vec<String>,
}

const HIRING_KEYWORDS: &[(&str, &str)] = &[
    ("мы ищем", "Мы ищем"),
    ("открытые вакансии", "Открытые вакансии"),
    ("вакансии", "Вакансии"),
    ("требуются", "Требуются"),
    ("присоединяйтесь к команде", "Присоединяйтесь к команде"),
    ("работа в компании", "Работа в компании"),
    ("карьера в", "Карьера"),
    ("ищем в команду", "Ищем в команду"),
    ("we are hiring", "We are hiring"),
    ("we're hiring", "We're hiring"),
    ("open positions", "Open positions"),
    ("join our team", "Join our team"),
    ("job openings", "Job openings"),
    ("current vacancies", "Current vacancies"),
    ("view open roles", "Open roles"),
];

const ROLE_PATTERNS: &[(&str, &str)] = &[
    (
        r"(?i)\brust\s+(?:developer|инженер|разработчик)\b",
        "Rust Разработчик",
    ),
    (
        r"(?i)\bpython\s+(?:developer|разработчик)\b",
        "Python Разработчик",
    ),
    (
        r"(?i)\bfrontend\s*(?:developer|разработчик)?\b",
        "Frontend Разработчик",
    ),
    (
        r"(?i)\bbackend\s*(?:developer|разработчик)?\b",
        "Backend Разработчик",
    ),
    (
        r"(?i)\bfullstack\s*(?:developer|разработчик)?\b",
        "Fullstack Разработчик",
    ),
    (r"(?i)\bdevops\s*(?:инженер|engineer)?\b", "DevOps Инженер"),
    (
        r"(?i)\bqa\s*(?:engineer|инженер|тестировщик)?\b",
        "QA / Тестировщик",
    ),
    (r"(?i)\b(?:golang|go\s+разработчик)\b", "Go Разработчик"),
    (
        r"(?i)\b(?:java\s+разработчик|java\s+developer)\b",
        "Java Разработчик",
    ),
    (
        r"(?i)\b(?:ios|android)\s+разработчик\b",
        "Mobile Разработчик",
    ),
    (
        r"(?i)\b(?:менеджер\s+по\s+продажам|b2b\s+sales)\b",
        "Менеджер по продажам",
    ),
    (
        r"(?i)\b(?:project\s+manager|руководитель\s+проектов)\b",
        "Project Manager",
    ),
    (
        r"(?i)\b(?:product\s+manager|продакт\s+менеджер)\b",
        "Product Manager",
    ),
    (
        r"(?i)\b(?:ui/ux|дизайнер\s+интерфейсов)\b",
        "UI/UX Дизайнер",
    ),
    (r"(?i)\b(?:hr|рекрутер|it\s+recruiter)\b", "HR / Рекрутер"),
    (
        r"(?i)\b(?:системный\s+администратор|sysadmin)\b",
        "Системный администратор",
    ),
    (
        r"(?i)\b1с\s*(?:программист|разработчик)?\b",
        "1С Разработчик",
    ),
];

pub fn detect_career_signals(html: &str, url: &str) -> CareerSignalInfo {
    let mut info = CareerSignalInfo::default();
    let lower_html = html.to_lowercase();
    let lower_url = url.to_lowercase();

    let is_career_page = lower_url.contains("/careers")
        || lower_url.contains("/jobs")
        || lower_url.contains("/vakansii")
        || lower_url.contains("/rabota")
        || lower_url.contains("/hiring")
        || lower_url.contains("/join-us");

    // 1. Scan hiring keywords
    let mut trigger_count = 0;
    for (kw, label) in HIRING_KEYWORDS {
        if lower_html.contains(kw) {
            trigger_count += 1;
            if !info.signal_triggers.contains(&label.to_string()) {
                info.signal_triggers.push(label.to_string());
            }
        }
    }

    // Hiring is confirmed if it's a dedicated career URL with at least 1 keyword,
    // or any page with at least 2 distinct hiring triggers
    if (is_career_page && trigger_count >= 1) || trigger_count >= 2 {
        info.is_hiring = true;
    }

    // 2. Scan open roles
    for (pattern, role_name) in ROLE_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(html) {
                if !info.detected_roles.contains(&role_name.to_string()) {
                    info.detected_roles.push(role_name.to_string());
                }
            }
        }
    }

    // If specific roles found, definitely hiring
    if !info.detected_roles.is_empty() && (trigger_count >= 1 || is_career_page) {
        info.is_hiring = true;
    }

    info
}

pub fn apply_career_signals_to_contact(info: &CareerSignalInfo, contact: &mut ContactRecord) {
    if !info.is_hiring {
        return;
    }

    contact.hiring_active = true;
    for role in &info.detected_roles {
        if !contact.hiring_roles.contains(role) {
            contact.hiring_roles.push(role.clone());
        }
    }

    let hiring_tag = "Активно нанимает".to_string();
    if !contact.tags.contains(&hiring_tag) {
        contact.tags.push(hiring_tag);
    }

    // Boost lead score
    contact.lead_score = (contact.lead_score + 20).min(100);
    if !contact.hiring_roles.is_empty() {
        contact.score = LeadScore::Hot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hiring_triggers() {
        let html = r#"
            <html>
                <body>
                    <h1>Работа в компании</h1>
                    <p>Мы ищем талантливых специалистов в команду.</p>
                    <div class="vacancies">
                        <h3>Ищем: Rust разработчик (Middle/Senior)</h3>
                        <p>Также открыта вакансия Frontend разработчик на React.</p>
                    </div>
                </body>
            </html>
        "#;
        let info = detect_career_signals(html, "https://example.com/vakansii");
        assert!(info.is_hiring);
        assert!(
            info.detected_roles
                .contains(&"Rust Разработчик".to_string())
        );
        assert!(
            info.detected_roles
                .contains(&"Frontend Разработчик".to_string())
        );
    }
}
