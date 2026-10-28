use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightComplianceReport {
    pub compliance_score: u8,
    pub has_rfc8058_headers: bool,
    pub has_physical_address: bool,
    pub has_opt_out_link_or_text: bool,
    pub has_sender_identity: bool,
    pub preflight_status: String,
    pub checklist_passed: Vec<String>,
    pub checklist_failed: Vec<String>,
    pub recommendations: Vec<String>,
}

pub fn audit_outbound_preflight(
    body: &str,
    has_unsubscribe_headers: bool,
) -> PreflightComplianceReport {
    let clean = body.trim();
    let lower = clean.to_lowercase();

    let mut score = 0u8;
    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let mut recs = Vec::new();

    // 1. RFC 8058 One-Click Unsubscribe Headers
    let has_rfc8058 = has_unsubscribe_headers;
    if has_rfc8058 {
        score += 30;
        passed.push(
            "RFC 8058 One-Click List-Unsubscribe headers configured (Google/Yahoo 2024 compliance)"
                .to_string(),
        );
    } else {
        failed.push("Missing List-Unsubscribe / List-Unsubscribe-Post headers".to_string());
        recs.push("Inject RFC 8058 headers into outbound SMTP packets to enable native unsubscribe buttons in Gmail/Yahoo.".to_string());
    }

    // 2. Opt-out link or conversational opt-out phrasing (Rule 55: Invariant word stems and imperative synonyms)
    let has_opt_out = lower.contains("unsubscribe")
        || lower.contains("отписаться")
        || lower.contains("отписка")
        || lower.contains("opt-out")
        || lower.contains("opt out")
        || lower.contains("не актуальн")
        || lower.contains("не интересн")
        || lower.contains("напишите")
        || lower.contains("ответьте")
        || lower.contains("дайте знать")
        || lower.contains("сообщите")
        || lower.contains("больше не беспоко")
        || lower.contains("больше не пишите")
        || lower.contains("не беспокойте")
        || lower.contains("не нужно")
        || lower.contains("kein interesse")
        || lower.contains("abmelden")
        || lower.contains("nicht mehr")
        || lower.contains("reply stop")
        || lower.contains("reply no");

    if has_opt_out {
        score += 30;
        passed.push(
            "Clear opt-out text or unsubscribe mechanism present in email footer".to_string(),
        );
    } else {
        failed.push("No opt-out wording or unsubscribe mechanism found in body".to_string());
        recs.push("Add a gentle opt-out statement (e.g., 'Если тема не актуальна, просто дайте знать — больше не побеспокою').".to_string());
    }

    // 3. Physical Address / Legal Entity presence
    let has_physical = lower.contains("г. москва")
        || lower.contains("г. санкт-петербург")
        || lower.contains("москва")
        || lower.contains("ул.")
        || lower.contains("улица")
        || lower.contains("пр-т")
        || lower.contains("street")
        || lower.contains("st.")
        || lower.contains("suite")
        || lower.contains("ave")
        || lower.contains("avenue")
        || lower.contains("box ")
        || lower.contains("россия")
        || lower.contains("ооо \"")
        || lower.contains("ао \"")
        || lower.contains("llc");

    if has_physical {
        score += 20;
        passed.push(
            "Physical postal address or registered entity identified (CAN-SPAM / 152-ФЗ)"
                .to_string(),
        );
    } else {
        failed.push(
            "Missing physical mailing address or registered legal entity in signature".to_string(),
        );
        recs.push("Include company physical address or registered legal name in email signature to satisfy CAN-SPAM and 152-ФЗ requirements.".to_string());
    }

    // 4. Sender Identity
    let has_identity = clean.len() > 20
        && (lower.contains("с уважением")
            || lower.contains("best regards")
            || lower.contains("regards")
            || lower.contains("команда")
            || lower.contains("team")
            || lower.contains("founder")
            || lower.contains("ceo")
            || lower.contains("руководитель"));

    if has_identity {
        score += 20;
        passed.push("Legitimate sender identity and professional sign-off present".to_string());
    } else {
        failed.push("Vague or missing sender identity/sign-off".to_string());
        recs.push(
            "Include a transparent sign-off with sender name, title, and organization.".to_string(),
        );
    }

    let status = if score >= 85 {
        "GREEN: Ready for Dispatch (Full Regulatory Compliance)".to_string()
    } else if score >= 50 {
        "YELLOW: Minor Warnings (Elevated Promotions / Compliance Risk)".to_string()
    } else {
        "RED: Blocked (Severe Non-Compliance / High Spam Complaint Risk)".to_string()
    };

    PreflightComplianceReport {
        compliance_score: score,
        has_rfc8058_headers: has_rfc8058,
        has_physical_address: has_physical,
        has_opt_out_link_or_text: has_opt_out,
        has_sender_identity: has_identity,
        preflight_status: status,
        checklist_passed: passed,
        checklist_failed: failed,
        recommendations: recs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fully_compliant_email() {
        let body = "Здравствуйте!\n\nПредлагаем аудит безопасности.\n\nЕсли тема не актуальна, просто ответьте 'нет'.\n\nС уважением, Алексей Смирнов\nCEO, ООО \"Секьюрити Лабс\"\nг. Москва, ул. Ленина, д. 10";
        let rep = audit_outbound_preflight(body, true);
        assert_eq!(rep.compliance_score, 100);
        assert!(rep.preflight_status.contains("GREEN"));
        assert!(rep.checklist_failed.is_empty());
    }

    #[test]
    fn test_missing_opt_out_and_address() {
        let body = "Купите наш курс прямо сейчас! Ссылка: http://buy.now";
        let rep = audit_outbound_preflight(body, false);
        assert_eq!(rep.compliance_score, 0);
        assert!(rep.preflight_status.contains("RED"));
        assert_eq!(rep.checklist_failed.len(), 4);
    }

    #[test]
    fn test_conversational_opt_out() {
        let body = "Worth a chat? If not relevant, please reply no.\n\nBest regards,\nAlex, Founder\n123 Market St, Suite 400";
        let rep = audit_outbound_preflight(body, true);
        assert_eq!(rep.compliance_score, 100);
        assert!(rep.has_opt_out_link_or_text);
    }

    #[test]
    fn test_multilingual_stem_matching() {
        let ru_body = "Здравствуйте!\n\nЕсли это не актуально для вашей команды, просто дайте знать.\n\nС уважением, Иван, ООО \"ИТ Решения\", Москва, ул. Тверская 1";
        let rep_ru = audit_outbound_preflight(ru_body, true);
        assert!(rep_ru.has_opt_out_link_or_text);
        assert_eq!(rep_ru.compliance_score, 100);

        let de_body = "Hallo,\n\nFalls kein Interesse besteht, geben Sie kurz Bescheid.\n\nMit freundlichen Grüßen,\nMax, CEO, Muster GmbH, Berlin, Hauptstr. 5";
        let rep_de = audit_outbound_preflight(de_body, true);
        assert!(rep_de.has_opt_out_link_or_text);
    }
}
