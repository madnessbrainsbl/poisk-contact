use crate::models::SpamTrapAuditResult;

const HONEYPOT_LOCAL_PARTS: &[&str] = &[
    "spamtrap",
    "trap",
    "honeypot",
    "blackhole",
    "sinkhole",
    "dontsend",
    "bounces",
    "null",
    "spam",
    "fake",
];

const COMPLIANCE_TRAP_LOCAL_PARTS: &[&str] = &[
    "abuse",
    "postmaster",
    "fbl",
    "noc",
    "hostmaster",
    "compliance",
    "privacy",
    "legal",
    "dpo",
];

pub fn audit_spam_trap(email: &str) -> SpamTrapAuditResult {
    let lower = email.trim().to_lowercase();
    let local = lower.split('@').next().unwrap_or("");

    if local.is_empty() {
        return SpamTrapAuditResult {
            email: email.to_string(),
            is_spam_trap: false,
            trap_category: "Некорректный адрес".to_string(),
            risk_level: "High".to_string(),
            action_recommended: "Адрес не валиден".to_string(),
        };
    }

    // 1. Pristine / Blacklist Honey Pots
    for &hp in HONEYPOT_LOCAL_PARTS {
        if local == hp
            || local.starts_with(&format!("{}_", hp))
            || local.starts_with(&format!("{}.", hp))
        {
            return SpamTrapAuditResult {
                email: email.to_string(),
                is_spam_trap: true,
                trap_category: "Blacklist Honey Pot (Spamhaus / SORBS Seed)".to_string(),
                risk_level: "Critical".to_string(),
                action_recommended: "Категорически исключить из рассылки (гарантированная мгновенная блокировка IP/домена в DNSBL)".to_string(),
            };
        }
    }

    // 2. Compliance / Abuse Traps
    for &comp in COMPLIANCE_TRAP_LOCAL_PARTS {
        if local == comp {
            return SpamTrapAuditResult {
                email: email.to_string(),
                is_spam_trap: true,
                trap_category: "Provider Abuse & Compliance Gateway".to_string(),
                risk_level: "High".to_string(),
                action_recommended: "Служебный адрес мониторинга нарушений. Отправка приведет к жалобе хостинг-провайдеру или регистратору".to_string(),
            };
        }
    }

    SpamTrapAuditResult {
        email: email.to_string(),
        is_spam_trap: false,
        trap_category: "Пользовательский / Корпоративный адрес".to_string(),
        risk_level: "Safe".to_string(),
        action_recommended: "Безопасен для отправки (паттернов спам-ловушек не обнаружено)"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honeypot_trap() {
        let res = audit_spam_trap("spamtrap_seed@target.com");
        assert!(res.is_spam_trap);
        assert_eq!(res.risk_level, "Critical");
        assert!(res.trap_category.contains("Honey Pot"));

        let res2 = audit_spam_trap("honeypot@company.ru");
        assert!(res2.is_spam_trap);
        assert_eq!(res2.risk_level, "Critical");
    }

    #[test]
    fn test_compliance_trap() {
        let res = audit_spam_trap("abuse@provider.net");
        assert!(res.is_spam_trap);
        assert_eq!(res.risk_level, "High");
        assert!(res.trap_category.contains("Abuse"));
    }

    #[test]
    fn test_clean_mailbox() {
        let res = audit_spam_trap("alexey.smirnov@company.ru");
        assert!(!res.is_spam_trap);
        assert_eq!(res.risk_level, "Safe");
    }
}
