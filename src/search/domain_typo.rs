use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainTypoAnalysis {
    pub domain: String,
    pub is_canonical: bool,
    pub is_typo_risk: bool,
    pub suggested_domain: Option<String>,
    pub edit_distance: Option<usize>,
    pub risk_score: u32,
    pub reason: String,
}

const CANONICAL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "yahoo.com",
    "yandex.ru",
    "ya.ru",
    "mail.ru",
    "inbox.ru",
    "list.ru",
    "bk.ru",
    "rambler.ru",
    "icloud.com",
    "me.com",
    "mac.com",
    "proton.me",
    "protonmail.com",
    "zoho.com",
    "aol.com",
];

pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    let mut dp = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        dp[i][0] = i;
    }
    for j in 0..=len2 {
        dp[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[len1][len2]
}

pub fn detect_domain_typo(raw_domain: &str) -> DomainTypoAnalysis {
    let clean = raw_domain.trim().to_lowercase();
    let trimmed = clean.trim_matches(|c: char| c == '.' || c == ',' || c == ' ');

    if trimmed.is_empty() {
        return DomainTypoAnalysis {
            domain: raw_domain.to_string(),
            is_canonical: false,
            is_typo_risk: true,
            suggested_domain: None,
            edit_distance: None,
            risk_score: 95,
            reason: "Empty or invalid domain structure.".to_string(),
        };
    }

    if trimmed != clean {
        return DomainTypoAnalysis {
            domain: raw_domain.to_string(),
            is_canonical: false,
            is_typo_risk: true,
            suggested_domain: Some(trimmed.to_string()),
            edit_distance: Some(clean.len() - trimmed.len()),
            risk_score: 85,
            reason: format!(
                "Domain contains trailing/leading punctuation: '{}'",
                raw_domain
            ),
        };
    }

    if trimmed.contains("..") || trimmed.ends_with(".com.com") || trimmed.ends_with(".ru.ru") {
        return DomainTypoAnalysis {
            domain: raw_domain.to_string(),
            is_canonical: false,
            is_typo_risk: true,
            suggested_domain: None,
            edit_distance: None,
            risk_score: 90,
            reason: "Malformed repeated TLD or double-dot detected.".to_string(),
        };
    }

    if CANONICAL_DOMAINS.contains(&trimmed) {
        return DomainTypoAnalysis {
            domain: trimmed.to_string(),
            is_canonical: true,
            is_typo_risk: false,
            suggested_domain: None,
            edit_distance: Some(0),
            risk_score: 0,
            reason: "Domain is a verified canonical email provider.".to_string(),
        };
    }

    let mut best_suggestion: Option<(&str, usize)> = None;

    for &canonical in CANONICAL_DOMAINS {
        let dist = levenshtein_distance(trimmed, canonical);
        let max_allowed = if canonical.len() <= 6 { 1 } else { 2 };
        if dist > 0 && dist <= max_allowed {
            match best_suggestion {
                None => best_suggestion = Some((canonical, dist)),
                Some((_, existing_dist)) => {
                    if dist < existing_dist {
                        best_suggestion = Some((canonical, dist));
                    }
                }
            }
        }
    }

    if let Some((suggested, dist)) = best_suggestion {
        DomainTypoAnalysis {
            domain: trimmed.to_string(),
            is_canonical: false,
            is_typo_risk: true,
            suggested_domain: Some(suggested.to_string()),
            edit_distance: Some(dist),
            risk_score: 90,
            reason: format!(
                "High probability typo (edit distance: {}). Likely intended canonical provider '{}'.",
                dist, suggested
            ),
        }
    } else {
        DomainTypoAnalysis {
            domain: trimmed.to_string(),
            is_canonical: false,
            is_typo_risk: false,
            suggested_domain: None,
            edit_distance: None,
            risk_score: 0,
            reason:
                "Domain appears to be a unique corporate or custom domain without obvious typos."
                    .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("gmail.com", "gmai.com"), 1);
        assert_eq!(levenshtein_distance("same", "same"), 0);
    }

    #[test]
    fn test_canonical_domain() {
        let res = detect_domain_typo("gmail.com");
        assert!(res.is_canonical);
        assert!(!res.is_typo_risk);
        assert_eq!(res.risk_score, 0);
    }

    #[test]
    fn test_typo_gmail_and_outlook() {
        let res = detect_domain_typo("gmai.com");
        assert!(res.is_typo_risk);
        assert_eq!(res.suggested_domain.as_deref(), Some("gmail.com"));
        assert_eq!(res.edit_distance, Some(1));
        assert_eq!(res.risk_score, 90);

        let res2 = detect_domain_typo("outlok.com");
        assert!(res2.is_typo_risk);
        assert_eq!(res2.suggested_domain.as_deref(), Some("outlook.com"));
    }

    #[test]
    fn test_typo_yandex_and_yahoo() {
        let res = detect_domain_typo("yaho.com");
        assert!(res.is_typo_risk);
        assert_eq!(res.suggested_domain.as_deref(), Some("yahoo.com"));

        let res2 = detect_domain_typo("yandexx.ru");
        assert!(res2.is_typo_risk);
        assert_eq!(res2.suggested_domain.as_deref(), Some("yandex.ru"));
    }

    #[test]
    fn test_legitimate_custom_corporate_domain() {
        let res = detect_domain_typo("cloudflare.com");
        assert!(!res.is_typo_risk);
        assert!(!res.is_canonical);
        assert_eq!(res.risk_score, 0);

        let res2 = detect_domain_typo("acme-security-corp.io");
        assert!(!res2.is_typo_risk);
    }

    #[test]
    fn test_trailing_period_malformed() {
        let res = detect_domain_typo("example.com.");
        assert!(res.is_typo_risk);
        assert_eq!(res.suggested_domain.as_deref(), Some("example.com"));
    }
}
