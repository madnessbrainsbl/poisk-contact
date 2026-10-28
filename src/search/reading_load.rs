use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingLoadReport {
    pub word_count: usize,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub reading_time_secs: u32,
    pub avg_words_per_sentence: f64,
    pub max_paragraph_sentences: usize,
    pub cognitive_load_tier: String,
    pub mobile_skimmable: bool,
    pub suggestions: Vec<String>,
}

pub fn audit_reading_load(text: &str) -> ReadingLoadReport {
    let clean = text.trim();
    if clean.is_empty() {
        return ReadingLoadReport {
            word_count: 0,
            sentence_count: 0,
            paragraph_count: 0,
            reading_time_secs: 0,
            avg_words_per_sentence: 0.0,
            max_paragraph_sentences: 0,
            cognitive_load_tier: "Empty Content".to_string(),
            mobile_skimmable: false,
            suggestions: vec!["Text is empty. Provide email body for analysis.".to_string()],
        };
    }

    let paragraphs: Vec<&str> = clean
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let paragraph_count = paragraphs.len().max(1);
    let mut words: Vec<&str> = Vec::new();
    let mut sentences: Vec<String> = Vec::new();
    let mut max_paragraph_sentences = 0;

    for para in &paragraphs {
        let para_words: Vec<&str> = para.split_whitespace().filter(|w| !w.is_empty()).collect();
        words.extend(para_words);

        let mut current_sentence = String::new();
        let mut para_sentence_count = 0;

        for ch in para.chars() {
            current_sentence.push(ch);
            if ch == '.' || ch == '!' || ch == '?' || ch == '\n' {
                let trimmed = current_sentence.trim();
                if trimmed.len() > 2 {
                    sentences.push(trimmed.to_string());
                    para_sentence_count += 1;
                }
                current_sentence.clear();
            }
        }
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
            para_sentence_count += 1;
        }

        if para_sentence_count > max_paragraph_sentences {
            max_paragraph_sentences = para_sentence_count;
        }
    }

    let word_count = words.len();
    let sentence_count = sentences.len().max(1);
    let avg_words_per_sentence = (word_count as f64 / sentence_count as f64 * 10.0).round() / 10.0;

    // Skimming reading speed: 200 words/min = ~3.33 words/sec
    let reading_time_secs = ((word_count as f64 / 3.33).ceil() as u32).max(1);

    let mut suggestions = Vec::new();
    let mut is_optimal = true;

    if word_count > 150 {
        is_optimal = false;
        suggestions.push(format!(
            "Length is {} words. Cold email studies (Lavender/Mailshake) show reply rates drop 50% above 125 words. Cut non-essential details.",
            word_count
        ));
    } else if word_count < 35 {
        suggestions.push(format!(
            "Email is very brief ({} words). Verify the value proposition and call to action are clear.",
            word_count
        ));
    }

    if reading_time_secs > 35 {
        is_optimal = false;
        suggestions.push(format!(
            "Reading time is ~{}s. Busy B2B decision-makers skim cold pitches in 15-30 seconds. Trim to under 30s.",
            reading_time_secs
        ));
    }

    if max_paragraph_sentences > 3 {
        is_optimal = false;
        suggestions.push(format!(
            "Detected dense block with {} sentences. Break paragraphs into 1-2 sentence bites for effortless mobile reading.",
            max_paragraph_sentences
        ));
    }

    if avg_words_per_sentence > 18.0 {
        is_optimal = false;
        suggestions.push(format!(
            "Average sentence length is {:.1} words. Complex sentences create cognitive friction. Target 10-14 words per sentence.",
            avg_words_per_sentence
        ));
    }

    let cognitive_load_tier = if is_optimal && word_count <= 125 && reading_time_secs <= 35 {
        "Light / Optimal (Skimmable in < 30s)".to_string()
    } else if word_count <= 170 && reading_time_secs <= 50 {
        "Moderate (Acceptable, minor trim recommended)".to_string()
    } else {
        "Heavy / Overloaded (High Mobile Archive Risk)".to_string()
    };

    let mobile_skimmable = is_optimal;

    if suggestions.is_empty() {
        suggestions.push("Email length, sentence structure, and paragraph pacing are perfectly tuned for high executive reply rates.".to_string());
    }

    ReadingLoadReport {
        word_count,
        sentence_count,
        paragraph_count,
        reading_time_secs,
        avg_words_per_sentence,
        max_paragraph_sentences,
        cognitive_load_tier,
        mobile_skimmable,
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_short_outreach() {
        let text = "Здравствуйте, Алексей.\n\nЗаметил, что вы расширяете команду разработки в Москве. Помогаем IT-компаниям закрывать Senior позиции за 14 дней.\n\nБудет ли вам интересно взглянуть на 2-3 релевантных профиля на этой неделе?";
        let rep = audit_reading_load(text);
        assert!(rep.word_count <= 50);
        assert!(rep.reading_time_secs <= 20);
        assert!(rep.mobile_skimmable);
        assert_eq!(
            rep.cognitive_load_tier,
            "Light / Optimal (Skimmable in < 30s)"
        );
    }

    #[test]
    fn test_dense_overloaded_outreach() {
        let text = "Добрый день! Наша огромная международная компания занимается широким спектром консалтинговых, аудиторских, аналитических и внедренческих услуг на рынке корпоративного программного обеспечения уже более пятнадцати лет, успешно реализуя масштабные интеграционные проекты для сотен предприятий в различных секторах экономики, включая банковский сектор, тяжелую металлургию, телекоммуникации, розничную торговлю, логистику и фармацевтическую промышленность. Мы предлагаем вам ознакомиться с нашим подробным 80-страничным коммерческим предложением, которое описывает все возможные методологии, сертификаты и варианты сотрудничества, а также назначить полуторачасовую видеоконференцию с нашей рабочей группой экспертов в любое удобное для вас время.";
        let rep = audit_reading_load(text);
        assert!(rep.word_count >= 60);
        assert!(rep.avg_words_per_sentence > 18.0);
        assert!(!rep.mobile_skimmable);
        assert!(
            rep.suggestions
                .iter()
                .any(|s| s.contains("Complex sentences"))
        );
    }
}
