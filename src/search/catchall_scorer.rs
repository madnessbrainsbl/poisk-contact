pub fn calculate_catchall_confidence(email: &str, mail_provider: &str) -> (u8, &'static str) {
    let lower_email = email.trim().to_lowercase();
    let local = lower_email.split('@').next().unwrap_or("");
    let prov_lower = mail_provider.to_lowercase();

    if local.is_empty() {
        return (20, "Некорректный адрес");
    }

    // 1. Generic Role inboxes on Catch-All (almost 100% accepted)
    match local {
        "info" | "sales" | "contact" | "support" | "office" | "hello" | "mail" | "hi" => {
            return (
                95,
                "Официальный ролевой контакт домена (гарантированный прием Catch-All)",
            );
        }
        _ => {}
    }

    // 2. Personal pattern detection
    let mut base_score: u8 = if local.contains('.') {
        // e.g. first.last
        80
    } else if local.contains('_') || local.contains('-') {
        75
    } else if local.chars().all(|c| c.is_alphabetic()) && local.len() >= 3 && local.len() <= 12 {
        // e.g. first or last
        70
    } else {
        40
    };

    // 3. Provider reputation adjustment
    if prov_lower.contains("google")
        || prov_lower.contains("workspace")
        || prov_lower.contains("microsoft")
        || prov_lower.contains("office 365")
        || prov_lower.contains("yandex")
    {
        base_score = base_score.saturating_add(10).min(90);
    } else if prov_lower.contains("cpanel") || prov_lower.contains("unknown") {
        base_score = base_score.saturating_sub(10);
    }

    let verdict = if base_score >= 80 {
        "Высокая вероятность доставки (типовой корпоративный паттерн)"
    } else if base_score >= 60 {
        "Средняя вероятность доставки (стандартный шаблон Catch-All)"
    } else {
        "Низкая вероятность доставки (нетипичный адрес Catch-All)"
    };

    (base_score, verdict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_catchall_confidence() {
        let (score, _) = calculate_catchall_confidence("info@enterprise.com", "Google Workspace");
        assert_eq!(score, 95);
    }

    #[test]
    fn test_first_last_google_catchall_confidence() {
        let (score, verdict) =
            calculate_catchall_confidence("alexey.smirnov@company.ru", "Google Workspace");
        assert!(score >= 85);
        assert!(verdict.contains("Высокая"));
    }

    #[test]
    fn test_random_cpanel_catchall_confidence() {
        let (score, verdict) = calculate_catchall_confidence("x89234js@unknown.ru", "cPanel Exim");
        assert!(score <= 40);
        assert!(verdict.contains("Низкая"));
    }
}
