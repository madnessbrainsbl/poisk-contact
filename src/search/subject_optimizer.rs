use crate::models::SubjectOptimizationReport;

const SUBJECT_SPAM_WORDS: &[&str] = &[
    "бесплатно",
    "гарантия",
    "срочно",
    "акция",
    "скидка",
    "внимание",
    "free",
    "guarantee",
    "urgent",
    "special offer",
    "100%",
    "купи",
    "заработай",
];

pub fn optimize_subject_line(subject: &str) -> SubjectOptimizationReport {
    let clean = subject.trim();
    let char_count = clean.chars().count();
    let words: Vec<&str> = clean.split_whitespace().collect();
    let word_count = words.len();

    let lower = clean.to_lowercase();
    let mut has_spam_words = false;
    for &sw in SUBJECT_SPAM_WORDS {
        if lower.contains(sw) {
            has_spam_words = true;
            break;
        }
    }

    let is_mobile_friendly =
        char_count <= 42 && word_count >= 2 && word_count <= 6 && !has_spam_words;

    let lowercase_variant = {
        let s = lower
            .trim_end_matches('!')
            .trim_end_matches('.')
            .to_string();
        if !s.is_empty() { s } else { clean.to_string() }
    };

    let mut advice = Vec::new();

    if char_count > 42 {
        advice.push(format!(
            "Длина темы {} символов. На смартфонах тема будет обрезана после 38-42 символов (рекомендуется до 40 символов).",
            char_count
        ));
    }

    if word_count > 6 {
        advice.push(format!(
            "Тема состоит из {} слов. Оптимальный диапазон для B2B cold outreach: от 2 до 5 слов (наивысший Open Rate в Instantly/Lavender).",
            word_count
        ));
    } else if word_count < 2 {
        advice.push("Тема состоит из 1 слова. Рекомендуется добавить контекст компании (например: 'вопрос по стеку {company}').".to_string());
    }

    if has_spam_words {
        advice.push(
            "Тема содержит маркетинговые стоп-слова, повышающие риск фильтрации ESP.".to_string(),
        );
    }

    if advice.is_empty() {
        advice.push("Тема идеально оптимизирована под мобильные экраны и высокий Open Rate (2-5 слов, до 40 символов).".to_string());
    }

    SubjectOptimizationReport {
        original_subject: clean.to_string(),
        char_count,
        word_count,
        is_mobile_friendly,
        has_spam_words,
        lowercase_variant,
        advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_mobile_subject() {
        let rep = optimize_subject_line("Вопрос по безопасности Яндекс");
        assert!(rep.is_mobile_friendly);
        assert_eq!(rep.word_count, 4);
        assert!(rep.char_count <= 42);
        assert!(!rep.has_spam_words);
        assert_eq!(rep.lowercase_variant, "вопрос по безопасности яндекс");
    }

    #[test]
    fn test_overlong_subject_with_spam_word() {
        let rep = optimize_subject_line(
            "Срочно получите 100% скидка на аудит информационной безопасности компании",
        );
        assert!(!rep.is_mobile_friendly);
        assert!(rep.has_spam_words);
        assert!(rep.char_count > 42);
        assert!(rep.word_count > 6);
        assert!(rep.advice.len() >= 2);
    }
}
