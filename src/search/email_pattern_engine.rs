use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedPatternCandidate {
    pub pattern_name: String,
    pub candidate_email: String,
    pub confidence_percent: u8,
    pub market_prevalence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPatternSynthesisReport {
    pub target_first_name: String,
    pub target_last_name: String,
    pub domain: String,
    pub dominant_pattern: String,
    pub top_candidate: String,
    pub candidates: Vec<SynthesizedPatternCandidate>,
}

pub fn transliterate_cyrillic(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.to_lowercase().chars() {
        match ch {
            'а' => out.push('a'),
            'б' => out.push('b'),
            'в' => out.push('v'),
            'г' => out.push('g'),
            'д' => out.push('d'),
            'е' | 'ё' => out.push('e'),
            'ж' => out.push_str("zh"),
            'з' => out.push('z'),
            'и' => out.push('i'),
            'й' => out.push('y'),
            'к' => out.push('k'),
            'л' => out.push('l'),
            'м' => out.push('m'),
            'н' => out.push('n'),
            'о' => out.push('o'),
            'п' => out.push('p'),
            'р' => out.push('r'),
            'с' => out.push('s'),
            'т' => out.push('t'),
            'у' => out.push('u'),
            'ф' => out.push('f'),
            'х' => out.push_str("kh"),
            'ц' => out.push_str("ts"),
            'ч' => out.push_str("ch"),
            'ш' => out.push_str("sh"),
            'щ' => out.push_str("shch"),
            'ъ' | 'ь' => {}
            'ы' => out.push('y'),
            'э' => out.push('e'),
            'ю' => out.push_str("yu"),
            'я' => out.push_str("ya"),
            c if c.is_ascii_alphanumeric() => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn synthesize_patterns(first: &str, last: &str, domain: &str) -> DomainPatternSynthesisReport {
    synthesize_domain_patterns(first, last, domain, &[])
}

pub fn synthesize_domain_patterns(
    first: &str,
    last: &str,
    domain: &str,
    known_domain_emails: &[String],
) -> DomainPatternSynthesisReport {
    let clean_domain = domain
        .trim()
        .to_lowercase()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_matches('/')
        .to_string();

    let clean_first = transliterate_cyrillic(first.trim());
    let clean_last = transliterate_cyrillic(last.trim());

    let f_initial = clean_first
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "a".to_string());
    let _l_initial = clean_last
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "s".to_string());

    // Check if known emails exhibit a pattern
    let mut known_has_dot = false;
    let mut known_has_underscore = false;
    let mut known_first_only = false;

    for email in known_domain_emails {
        if let Some(local) = email.split('@').next() {
            if local.contains('.') {
                known_has_dot = true;
            } else if local.contains('_') {
                known_has_underscore = true;
            } else if local.len() < 8 && !local.contains(|c: char| c.is_numeric()) {
                known_first_only = true;
            }
        }
    }

    let mut candidates = Vec::new();

    // 1. {first}.{last}@{domain} (42% market prevalence)
    let dot_conf = if known_has_dot { 95 } else { 85 };
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{first}.{last}".to_string(),
        candidate_email: format!("{}.{}@{}", clean_first, clean_last, clean_domain),
        confidence_percent: dot_conf,
        market_prevalence: "42% of Global B2B (Standard corporate default)".to_string(),
    });

    // 2. {first}@{domain} (28% market prevalence)
    let first_conf = if known_first_only { 90 } else { 70 };
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{first}".to_string(),
        candidate_email: format!("{}@{}", clean_first, clean_domain),
        confidence_percent: first_conf,
        market_prevalence: "28% of Global B2B (Startups & Executive accounts)".to_string(),
    });

    // 3. {f}.{last}@{domain} (14% market prevalence)
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{f}.{last}".to_string(),
        candidate_email: format!("{}.{}@{}", f_initial, clean_last, clean_domain),
        confidence_percent: 60,
        market_prevalence: "14% of Global B2B (Enterprise directory standard)".to_string(),
    });

    // 4. {first}_{last}@{domain} (9% market prevalence)
    let under_conf = if known_has_underscore { 90 } else { 50 };
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{first}_{last}".to_string(),
        candidate_email: format!("{}_{}@{}", clean_first, clean_last, clean_domain),
        confidence_percent: under_conf,
        market_prevalence: "9% of Global B2B (IT & Engineering legacy MTA)".to_string(),
    });

    // 5. {first}{last}@{domain} (4% market prevalence)
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{first}{last}".to_string(),
        candidate_email: format!("{}{{}}@{}", clean_first, clean_domain).replace("{}", &clean_last),
        confidence_percent: 40,
        market_prevalence: "4% of Global B2B (Compact concatenated)".to_string(),
    });

    // 6. {last}.{first}@{domain} (3% market prevalence)
    candidates.push(SynthesizedPatternCandidate {
        pattern_name: "{last}.{first}".to_string(),
        candidate_email: format!("{}.{}@{}", clean_last, clean_first, clean_domain),
        confidence_percent: 30,
        market_prevalence: "3% of Global B2B (Eastern European / Academic)".to_string(),
    });

    // Sort by confidence descending
    candidates.sort_by(|a, b| b.confidence_percent.cmp(&a.confidence_percent));

    let top = candidates
        .first()
        .map(|c| c.candidate_email.clone())
        .unwrap_or_default();
    let dominant = candidates
        .first()
        .map(|c| c.pattern_name.clone())
        .unwrap_or_else(|| "{first}.{last}".to_string());

    DomainPatternSynthesisReport {
        target_first_name: first.to_string(),
        target_last_name: last.to_string(),
        domain: clean_domain,
        dominant_pattern: dominant,
        top_candidate: top,
        candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latin_pattern_synthesis() {
        let rep = synthesize_domain_patterns("Alexey", "Smirnov", "company.com", &[]);
        assert_eq!(rep.target_first_name, "Alexey");
        assert_eq!(rep.domain, "company.com");
        assert_eq!(rep.top_candidate, "alexey.smirnov@company.com");
        assert_eq!(rep.dominant_pattern, "{first}.{last}");
        assert!(rep.candidates.len() >= 5);
    }

    #[test]
    fn test_cyrillic_translit_pattern_synthesis() {
        let rep = synthesize_domain_patterns("Павел", "Дуров", "telegram.org", &[]);
        assert_eq!(rep.top_candidate, "pavel.durov@telegram.org");
        assert!(
            rep.candidates
                .iter()
                .any(|c| c.candidate_email == "p.durov@telegram.org")
        );
    }

    #[test]
    fn test_known_sample_boosting() {
        let known = vec!["ivan_petrov@target.ru".to_string()];
        let rep = synthesize_domain_patterns("Sergey", "Volkov", "target.ru", &known);
        assert_eq!(rep.top_candidate, "sergey_volkov@target.ru");
        assert_eq!(rep.dominant_pattern, "{first}_{last}");
    }
}
