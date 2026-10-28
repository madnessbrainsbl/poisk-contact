use crate::search::extractor::transliterate_to_latin;
use std::collections::HashMap;

const COMMON_TYPOS: &[(&str, &str)] = &[
    ("gmai.com", "gmail.com"),
    ("gamil.com", "gmail.com"),
    ("gmial.com", "gmail.com"),
    ("gmaill.com", "gmail.com"),
    ("gmaul.com", "gmail.com"),
    ("yandexx.ru", "yandex.ru"),
    ("yadnex.ru", "yandex.ru"),
    ("yandx.ru", "yandex.ru"),
    ("yndx.ru", "yandex.ru"),
    ("mail.ry", "mail.ru"),
    ("mial.ru", "mail.ru"),
    ("mai.ru", "mail.ru"),
    ("maill.ru", "mail.ru"),
    ("bk.ry", "bk.ru"),
    ("inbox.ry", "inbox.ru"),
    ("list.ry", "list.ru"),
    ("rambler.ry", "rambler.ru"),
    ("ramler.ru", "rambler.ru"),
    ("outlok.com", "outlook.com"),
    ("outluk.com", "outlook.com"),
    ("hotmial.com", "hotmail.com"),
    ("hotamil.com", "hotmail.com"),
    ("yahooo.com", "yahoo.com"),
    ("yaho.com", "yahoo.com"),
];

pub fn detect_domain_typo(email: &str) -> Option<String> {
    let clean = email.trim().to_lowercase();
    let parts: Vec<&str> = clean.split('@').collect();
    if parts.len() != 2 {
        return None;
    }
    let (user, domain) = (parts[0], parts[1]);
    for (typo, fix) in COMMON_TYPOS {
        if domain == *typo {
            return Some(format!("{}@{}", user, fix));
        }
    }
    None
}

#[derive(Debug, Clone, Default)]
pub struct DomainPatternInfo {
    pub dominant_pattern: String,
    pub confidence_percent: u8,
    pub sample_count: usize,
    pub breakdown: Vec<(String, usize)>,
}

pub fn analyze_domain_patterns(
    contacts: &[(String, String, String)], // (name, email, domain)
) -> Option<DomainPatternInfo> {
    if contacts.is_empty() {
        return None;
    }

    let mut pattern_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_matched = 0;

    for (name, email, domain) in contacts {
        let clean_email = email.trim().to_lowercase();
        let user_part = match clean_email.split_once('@') {
            Some((u, d)) => {
                if !d.eq_ignore_ascii_case(domain) {
                    continue;
                }
                u
            }
            None => continue,
        };

        // Split name into first and last
        let name_words: Vec<&str> = name.split_whitespace().collect();
        if name_words.len() < 2 {
            continue;
        }

        let f = transliterate_to_latin(name_words[0]);
        let l = transliterate_to_latin(name_words[1]);
        if f.is_empty() || l.is_empty() {
            continue;
        }

        let f_init = f.chars().next().map(|c| c.to_string()).unwrap_or_default();
        let l_init = l.chars().next().map(|c| c.to_string()).unwrap_or_default();

        let pattern = if user_part == format!("{}.{}", f, l) {
            "{first}.{last}"
        } else if user_part == f {
            "{first}"
        } else if user_part == format!("{}.{}", f_init, l) {
            "{f}.{last}"
        } else if user_part == format!("{}{}", f_init, l) {
            "{f}{last}"
        } else if user_part == format!("{}_{}", f, l) {
            "{first}_{last}"
        } else if user_part == format!("{}{}", f, l) {
            "{first}{last}"
        } else if user_part == l {
            "{last}"
        } else if user_part == format!("{}.{}", l, f) {
            "{last}.{first}"
        } else if user_part == format!("{}.{}", l, f_init) {
            "{last}.{f}"
        } else if user_part == format!("{}.{}", f, l_init) {
            "{first}.{l}"
        } else {
            continue;
        };

        *pattern_counts.entry(pattern).or_insert(0) += 1;
        total_matched += 1;
    }

    if total_matched == 0 {
        return None;
    }

    let mut sorted_breakdown: Vec<(String, usize)> = pattern_counts
        .into_iter()
        .map(|(p, count)| (p.to_string(), count))
        .collect();
    sorted_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

    let dominant = &sorted_breakdown[0];
    let confidence = ((dominant.1 as f64 / total_matched as f64) * 100.0).round() as u8;

    Some(DomainPatternInfo {
        dominant_pattern: dominant.0.clone(),
        confidence_percent: confidence,
        sample_count: total_matched,
        breakdown: sorted_breakdown,
    })
}

pub fn predict_email_by_pattern(first: &str, last: &str, domain: &str, pattern: &str) -> String {
    let clean_domain = domain.trim().trim_start_matches('@').to_lowercase();
    let f = transliterate_to_latin(first.trim());
    let l = transliterate_to_latin(last.trim());
    let f_init = f.chars().next().map(|c| c.to_string()).unwrap_or_default();
    let l_init = l.chars().next().map(|c| c.to_string()).unwrap_or_default();

    let user = match pattern {
        "{first}.{last}" => format!("{}.{}", f, l),
        "{first}" => f,
        "{f}.{last}" => format!("{}.{}", f_init, l),
        "{f}{last}" => format!("{}{}", f_init, l),
        "{first}_{last}" => format!("{}_{}", f, l),
        "{first}{last}" => format!("{}{}", f, l),
        "{last}" => l,
        "{last}.{first}" => format!("{}.{}", l, f),
        "{last}.{f}" => format!("{}.{}", l, f_init),
        "{first}.{l}" => format!("{}.{}", f, l_init),
        _ => format!("{}.{}", f, l),
    };

    format!("{}@{}", user, clean_domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_typos() {
        assert_eq!(
            detect_domain_typo("user@gmai.com"),
            Some("user@gmail.com".to_string())
        );
        assert_eq!(
            detect_domain_typo("info@yadnex.ru"),
            Some("info@yandex.ru".to_string())
        );
        assert_eq!(detect_domain_typo("valid@yandex.ru"), None);
    }

    #[test]
    fn test_pattern_analysis() {
        let contacts = vec![
            (
                "Алексей Смирнов".to_string(),
                "aleksey.smirnov@acme.com".to_string(),
                "acme.com".to_string(),
            ),
            (
                "Дмитрий Иванов".to_string(),
                "dmitriy.ivanov@acme.com".to_string(),
                "acme.com".to_string(),
            ),
            (
                "Ольга Соколова".to_string(),
                "olga@acme.com".to_string(),
                "acme.com".to_string(),
            ),
        ];

        let res = analyze_domain_patterns(&contacts).unwrap();
        assert_eq!(res.dominant_pattern, "{first}.{last}");
        assert_eq!(res.sample_count, 3);
        assert!(res.confidence_percent >= 66);

        let predicted =
            predict_email_by_pattern("Иван", "Петров", "acme.com", &res.dominant_pattern);
        assert_eq!(predicted, "ivan.petrov@acme.com");
    }
}
