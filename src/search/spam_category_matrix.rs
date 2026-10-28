use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpamWordCategory {
    Urgency,
    Financial,
    Freebies,
    Hype,
}

impl std::fmt::Display for SpamWordCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpamWordCategory::Urgency => write!(f, "Urgency / False Pressure"),
            SpamWordCategory::Financial => write!(f, "Financial / Exaggerated Claims"),
            SpamWordCategory::Freebies => write!(f, "Freebies / Bait"),
            SpamWordCategory::Hype => write!(f, "Marketing Hype / Hyperbole"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizedSpamHit {
    pub word: String,
    pub category: SpamWordCategory,
    pub suggested_replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamMatrixReport {
    pub total_words: usize,
    pub spam_words_count: usize,
    pub spam_word_density_percent: f64,
    pub urgency_hits: usize,
    pub financial_hits: usize,
    pub freebie_hits: usize,
    pub hype_hits: usize,
    pub penalty_score: u8,
    pub deliverability_verdict: String,
    pub hits: Vec<CategorizedSpamHit>,
    pub clean_b2b_recommendation: String,
}

struct DictionaryEntry {
    pattern: &'static str,
    category: SpamWordCategory,
    replacement: &'static str,
}

const SPAM_DICTIONARY: &[DictionaryEntry] = &[
    // Urgency
    DictionaryEntry {
        pattern: "срочно",
        category: SpamWordCategory::Urgency,
        replacement: "при наличии времени",
    },
    DictionaryEntry {
        pattern: "только сегодня",
        category: SpamWordCategory::Urgency,
        replacement: "на этой неделе",
    },
    DictionaryEntry {
        pattern: "успейте",
        category: SpamWordCategory::Urgency,
        replacement: "приглашаем ознакомиться",
    },
    DictionaryEntry {
        pattern: "ограниченное предложение",
        category: SpamWordCategory::Urgency,
        replacement: "текущие условия",
    },
    DictionaryEntry {
        pattern: "urgent",
        category: SpamWordCategory::Urgency,
        replacement: "timely",
    },
    DictionaryEntry {
        pattern: "act now",
        category: SpamWordCategory::Urgency,
        replacement: "when convenient",
    },
    DictionaryEntry {
        pattern: "limited time",
        category: SpamWordCategory::Urgency,
        replacement: "current roadmap",
    },
    DictionaryEntry {
        pattern: "deadline",
        category: SpamWordCategory::Urgency,
        replacement: "target timeframe",
    },
    DictionaryEntry {
        pattern: "last chance",
        category: SpamWordCategory::Urgency,
        replacement: "final check-in",
    },
    // Financial
    DictionaryEntry {
        pattern: "гарантия 100%",
        category: SpamWordCategory::Financial,
        replacement: "проверенная методология",
    },
    DictionaryEntry {
        pattern: "без вложений",
        category: SpamWordCategory::Financial,
        replacement: "оптимальное бюджетирование",
    },
    DictionaryEntry {
        pattern: "пассивный доход",
        category: SpamWordCategory::Financial,
        replacement: "операционная эффективность",
    },
    DictionaryEntry {
        pattern: "заработок",
        category: SpamWordCategory::Financial,
        replacement: "рост показателей",
    },
    DictionaryEntry {
        pattern: "100% free",
        category: SpamWordCategory::Financial,
        replacement: "complimentary review",
    },
    DictionaryEntry {
        pattern: "make money",
        category: SpamWordCategory::Financial,
        replacement: "accelerate revenue",
    },
    DictionaryEntry {
        pattern: "risk-free",
        category: SpamWordCategory::Financial,
        replacement: "verified process",
    },
    DictionaryEntry {
        pattern: "extra income",
        category: SpamWordCategory::Financial,
        replacement: "efficiency gains",
    },
    // Freebies
    DictionaryEntry {
        pattern: "бесплатно",
        category: SpamWordCategory::Freebies,
        replacement: "в рамках ознакомления",
    },
    DictionaryEntry {
        pattern: "подарок",
        category: SpamWordCategory::Freebies,
        replacement: "прикладные материалы",
    },
    DictionaryEntry {
        pattern: "приз",
        category: SpamWordCategory::Freebies,
        replacement: "аналитический обзор",
    },
    DictionaryEntry {
        pattern: "free gift",
        category: SpamWordCategory::Freebies,
        replacement: "summary benchmark",
    },
    DictionaryEntry {
        pattern: "free trial",
        category: SpamWordCategory::Freebies,
        replacement: "pilot evaluation",
    },
    DictionaryEntry {
        pattern: "free",
        category: SpamWordCategory::Freebies,
        replacement: "complimentary",
    },
    DictionaryEntry {
        pattern: "prize",
        category: SpamWordCategory::Freebies,
        replacement: "overview",
    },
    // Hype
    DictionaryEntry {
        pattern: "революционный",
        category: SpamWordCategory::Hype,
        replacement: "современный",
    },
    DictionaryEntry {
        pattern: "уникальный",
        category: SpamWordCategory::Hype,
        replacement: "специализированный",
    },
    DictionaryEntry {
        pattern: "лучший на рынке",
        category: SpamWordCategory::Hype,
        replacement: "проверенное решение",
    },
    DictionaryEntry {
        pattern: "не имеет аналогов",
        category: SpamWordCategory::Hype,
        replacement: "высокоэффективный подход",
    },
    DictionaryEntry {
        pattern: "miracle",
        category: SpamWordCategory::Hype,
        replacement: "practical methodology",
    },
    DictionaryEntry {
        pattern: "unbelievable",
        category: SpamWordCategory::Hype,
        replacement: "substantial",
    },
    DictionaryEntry {
        pattern: "once in a lifetime",
        category: SpamWordCategory::Hype,
        replacement: "timely opportunity",
    },
];

pub fn audit_spam_categories(text: &str) -> SpamMatrixReport {
    audit_spam_category_matrix(text)
}

pub fn audit_spam_category_matrix(text: &str) -> SpamMatrixReport {
    let clean = text.trim();
    if clean.is_empty() {
        return SpamMatrixReport {
            total_words: 0,
            spam_words_count: 0,
            spam_word_density_percent: 0.0,
            urgency_hits: 0,
            financial_hits: 0,
            freebie_hits: 0,
            hype_hits: 0,
            penalty_score: 0,
            deliverability_verdict: "Clean / Empty".to_string(),
            hits: Vec::new(),
            clean_b2b_recommendation: "Text is empty.".to_string(),
        };
    }

    let words: Vec<&str> = clean.split_whitespace().collect();
    let total_words = words.len().max(1);
    let lower_text = clean.to_lowercase();

    let mut hits = Vec::new();
    let mut urgency_hits = 0;
    let mut financial_hits = 0;
    let mut freebie_hits = 0;
    let mut hype_hits = 0;

    for entry in SPAM_DICTIONARY {
        if lower_text.contains(entry.pattern) {
            match entry.category {
                SpamWordCategory::Urgency => urgency_hits += 1,
                SpamWordCategory::Financial => financial_hits += 1,
                SpamWordCategory::Freebies => freebie_hits += 1,
                SpamWordCategory::Hype => hype_hits += 1,
            }
            hits.push(CategorizedSpamHit {
                word: entry.pattern.to_string(),
                category: entry.category.clone(),
                suggested_replacement: entry.replacement.to_string(),
            });
        }
    }

    let spam_words_count = hits.len();
    let spam_word_density_percent =
        ((spam_words_count as f64 / total_words as f64) * 1000.0).round() / 10.0;

    // Weighted penalty scoring
    let raw_penalty =
        (urgency_hits * 20) + (financial_hits * 30) + (freebie_hits * 25) + (hype_hits * 15);
    let penalty_score = (raw_penalty as u8).min(100);

    let (deliverability_verdict, clean_b2b_recommendation) = if penalty_score >= 40 {
        (
            "High Spam Filter Penalty (>40): Extreme Risk of Junk/Spam folder".to_string(),
            "Email copy contains aggressive marketing triggers across multiple psychological categories. High probability of triggering ESP filters (SpamAssassin, Google Defender). Replace flagged hype terms with conversational B2B phrasing.".to_string(),
        )
    } else if penalty_score >= 15 {
        (
            "Elevated Promotions Risk (15-39): Likely Promotions Tab Routing".to_string(),
            "Moderate trigger density detected. While unlikely to trigger outright spam block, it may degrade primary inbox placement in Gmail and Outlook. Apply suggested clean B2B replacements.".to_string(),
        )
    } else {
        (
            "Inbox Safe (<15): Optimal Peer-to-Peer Deliverability".to_string(),
            "Copy is free of aggressive promotional triggers and reads as a professional one-on-one business correspondence.".to_string(),
        )
    };

    SpamMatrixReport {
        total_words,
        spam_words_count,
        spam_word_density_percent,
        urgency_hits,
        financial_hits,
        freebie_hits,
        hype_hits,
        penalty_score,
        deliverability_verdict,
        hits,
        clean_b2b_recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_b2b_outreach_matrix() {
        let text = "Здравствуйте, Алексей. Заметил интересные проекты вашей команды. Предлагаем рассмотреть интеграцию безопасности в текущем цикле разработки. Будет ли удобно пообщаться в четверг?";
        let rep = audit_spam_category_matrix(text);
        assert_eq!(rep.spam_words_count, 0);
        assert_eq!(rep.penalty_score, 0);
        assert!(rep.deliverability_verdict.contains("Inbox Safe"));
    }

    #[test]
    fn test_aggressive_hype_and_financial_matrix() {
        let text = "Срочно! Успейте получить революционный продукт! Гарантия 100%, без вложений, абсолютно бесплатно для всех участников.";
        let rep = audit_spam_category_matrix(text);
        assert!(rep.spam_words_count >= 4);
        assert!(rep.urgency_hits >= 1);
        assert!(rep.financial_hits >= 1);
        assert!(rep.freebie_hits >= 1);
        assert!(rep.hype_hits >= 1);
        assert!(rep.penalty_score >= 40);
        assert!(
            rep.deliverability_verdict
                .contains("High Spam Filter Penalty")
        );
    }

    #[test]
    fn test_english_matrix_hits() {
        let text =
            "Act now! This 100% free software will help you make money with a risk-free approach.";
        let rep = audit_spam_category_matrix(text);
        assert!(rep.urgency_hits >= 1);
        assert!(rep.financial_hits >= 2);
        assert!(rep.freebie_hits >= 1);
        assert!(rep.penalty_score >= 35);
    }
}
