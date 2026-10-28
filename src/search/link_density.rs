use crate::models::LinkAuditReport;

const SUSPICIOUS_SHORTENERS: &[&str] = &[
    "bit.ly",
    "tinyurl.com",
    "t.co",
    "goo.gl",
    "ow.ly",
    "is.gd",
    "buff.ly",
    "clck.ru",
    "cutt.ly",
];

pub fn audit_email_links(body: &str) -> LinkAuditReport {
    let clean_body = body.trim();
    if clean_body.is_empty() {
        return LinkAuditReport {
            is_safe_for_inbox: true,
            ..Default::default()
        };
    }

    let url_re = regex::Regex::new(r#"(?i)https?://[^\s<>"]+|www\.[^\s<>"]+"#).unwrap();
    let mut links = Vec::new();
    let mut total_link_chars = 0;
    let mut has_suspicious_redirects = false;
    let mut advice = Vec::new();

    for mat in url_re.find_iter(clean_body) {
        let u = mat
            .as_str()
            .trim_end_matches(&['.', ',', ';', '!', '?', ')', ']', '"', '\''][..]);
        links.push(u);
        total_link_chars += u.chars().count();

        let lower = u.to_lowercase();
        for &shortener in SUSPICIOUS_SHORTENERS {
            if lower.contains(shortener) {
                has_suspicious_redirects = true;
            }
        }
    }

    let total_body_chars = clean_body.chars().count().max(1);
    let link_density_pct = (total_link_chars as f32 / total_body_chars as f32) * 100.0;
    let link_count = links.len();

    if link_count > 2 {
        advice.push(format!(
            "В письме {} ссылок. Для первого холодного касания рекомендуется максимум 1 ссылка. 3+ ссылки активируют вкладку 'Промоакции' в Gmail.",
            link_count
        ));
    }

    if link_density_pct > 25.0 && link_count > 1 {
        advice.push(format!(
            "Плотность ссылок {:.1}% (>25% текста). Почтовые фильтры расценивают высокую плотность ссылок как спам.",
            link_density_pct
        ));
    }

    if has_suspicious_redirects {
        advice.push("Обнаружены сервисы сокращения ссылок (bit.ly/clck.ru). Спам-фильтры блокируют сокращатели из-за невозможности проверки конечного домена.".to_string());
    }

    let is_safe_for_inbox = link_count <= 2
        && (link_density_pct <= 25.0 || (link_count == 1 && link_density_pct <= 35.0))
        && !has_suspicious_redirects;

    LinkAuditReport {
        link_count,
        link_density_pct,
        has_suspicious_redirects,
        is_safe_for_inbox,
        advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_single_link_email() {
        let body = "Добрый день, Иван! Ознакомиться с нашими кейсами можно на сайте https://example.com/portfolio. Хорошего дня!";
        let rep = audit_email_links(body);
        assert_eq!(rep.link_count, 1);
        assert!(rep.is_safe_for_inbox);
        assert!(!rep.has_suspicious_redirects);
    }

    #[test]
    fn test_shortener_and_multiple_links() {
        let body = "Смотрите тут: https://bit.ly/123, также тут https://test.com/a, и тут https://test.com/b";
        let rep = audit_email_links(body);
        assert_eq!(rep.link_count, 3);
        assert!(rep.has_suspicious_redirects);
        assert!(!rep.is_safe_for_inbox);
        assert!(
            rep.advice
                .iter()
                .any(|a| a.contains("сокращения") || a.contains("3+ ссылки"))
        );
    }
}
