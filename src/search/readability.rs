use crate::models::OutreachPsychologyReport;

pub fn count_syllables_ru_en(word: &str) -> usize {
    let mut count = 0;
    let lower = word.to_lowercase();
    for ch in lower.chars() {
        match ch {
            'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э'
            | 'ю' | 'я' => {
                count += 1;
            }
            _ => {}
        }
    }
    count.max(1)
}

pub fn analyze_outreach_psychology(subject: &str, body: &str) -> OutreachPsychologyReport {
    let clean_body = body.trim();
    let clean_subj = subject.trim();

    if clean_body.is_empty() {
        return OutreachPsychologyReport {
            grade: "F".to_string(),
            recommendations: vec!["Тело письма пустое.".to_string()],
            ..Default::default()
        };
    }

    // 1. Word extraction
    let words: Vec<&str> = clean_body
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let word_count = words.len();

    // 2. Sentence extraction
    let sentence_delims = ['.', '!', '?', '\n'];
    let sentences: Vec<&str> = clean_body
        .split(|c| sentence_delims.contains(&c))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let sentence_count = sentences.len().max(1);

    // 3. Syllable count
    let total_syllables: usize = words.iter().map(|w| count_syllables_ru_en(w)).sum();

    // 4. Flesch Reading Ease calculation
    let words_per_sentence = word_count as f32 / sentence_count as f32;
    let syllables_per_word = if word_count > 0 {
        total_syllables as f32 / word_count as f32
    } else {
        1.0
    };

    let has_cyrillic = clean_body
        .chars()
        .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    let reading_ease_raw = if has_cyrillic {
        // Solovyev / Oborneva adaptation for Russian texts
        206.835 - (1.3 * words_per_sentence) - (60.1 * syllables_per_word)
    } else {
        // Standard Flesch Reading Ease for English
        206.835 - (1.015 * words_per_sentence) - (84.6 * syllables_per_word)
    };
    let reading_ease = reading_ease_raw.clamp(0.0, 100.0);

    // 5. Question count (CTA Friction)
    let question_count = clean_body.chars().filter(|&c| c == '?').count();

    // 6. Pronoun ratio ("You" vs "I/We")
    let mut you_count = 0;
    let mut i_count = 0;

    for w in &words {
        let lw = w.to_lowercase();
        match lw.as_str() {
            "вы" | "вас" | "вам" | "вами" | "ваш" | "ваша" | "ваше" | "ваши" | "вашего"
            | "вашей" | "вашим" | "ваших" | "ты" | "тебя" | "тебе" | "тобой" | "твой" | "твоя"
            | "твое" | "твои" | "you" | "your" | "yours" | "yourself" => {
                you_count += 1;
            }
            "я" | "меня" | "мне" | "мной" | "мой" | "моя" | "мое" | "мои" | "мы" | "нас"
            | "нам" | "нами" | "наш" | "наша" | "наше" | "наши" | "i" | "me" | "my" | "mine"
            | "myself" | "we" | "us" | "our" | "ours" => {
                i_count += 1;
            }
            _ => {}
        }
    }

    let you_to_i_ratio = if i_count == 0 {
        if you_count > 0 { 3.0 } else { 1.0 }
    } else {
        you_count as f32 / i_count as f32
    };

    // 7. Subject words
    let subj_words: Vec<&str> = clean_subj
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let subject_word_count = subj_words.len();

    // 8. Reading time (assuming 200 WPM)
    let reading_time_sec = ((word_count as u32 * 60) / 200).max(1);

    // 9. Scoring and recommendations
    let mut score = 0u8;
    let mut recommendations = Vec::new();

    // Length check (Lavender optimum: 25..=100 words)
    if (25..=100).contains(&word_count) {
        score += 25;
    } else if (101..=150).contains(&word_count) {
        score += 15;
        recommendations.push(format!(
            "Длина письма ({} слов) на грани оптимума. Lavender.ai рекомендует 25-100 слов для максимальной конверсии.",
            word_count
        ));
    } else if word_count > 150 {
        score += 5;
        recommendations.push(format!(
            "Внимание: письмо перегружено ({} слов). Длинные письма получают на 50% меньше ответов. Сократите до 100 слов.",
            word_count
        ));
    } else {
        score += 5;
        recommendations.push(
            "Письмо слишком краткое (<25 слов), не раскрыта суть сотрудничества.".to_string(),
        );
    }

    // Readability Ease check (calibrated for Cyrillic / English)
    let ease_threshold_high = if has_cyrillic { 50.0 } else { 65.0 };
    let ease_threshold_mid = if has_cyrillic { 35.0 } else { 50.0 };

    if reading_ease >= ease_threshold_high {
        score += 25;
    } else if reading_ease >= ease_threshold_mid {
        score += 15;
        recommendations.push(format!(
            "Читаемость Flesch {:.1}/100: предложения сложные. Разбейте длинные конструкции на простые фразы.",
            reading_ease
        ));
    } else {
        score += 5;
        recommendations.push(format!(
            "Сложный академический текст (Flesch {:.1}/100). Пишите как для разговора в мессенджере (уровень 6-8 класса).",
            reading_ease
        ));
    }

    // Question count check (Lavender optimum: exactly 1 clear question)
    if question_count == 1 {
        score += 20;
    } else if question_count == 2 {
        score += 15;
        recommendations.push(
            "Задано 2 вопроса. Оставьте 1 самый главный вопрос-призыв, чтобы упростить ответ."
                .to_string(),
        );
    } else if question_count == 0 {
        score += 5;
        recommendations.push("В письме нет четкого вопроса (Call To Action). Добавьте конкретный закрывающий вопрос.".to_string());
    } else {
        score += 5;
        recommendations.push(format!(
            "Слишком много вопросов ({} шт.). Это распыляет внимание лида (Choice Paralysis). Оставьте 1 вопрос.",
            question_count
        ));
    }

    // You-to-I ratio check (Customer-centric outreach)
    if you_to_i_ratio >= 1.5 {
        score += 20;
    } else if you_to_i_ratio >= 1.0 {
        score += 12;
        recommendations.push(format!(
            "Соотношение местоимений Вы/Мы ({:.1}): баланс на грани. Говорите больше о задачах клиента, а не о своей компании.",
            you_to_i_ratio
        ));
    } else {
        score += 5;
        recommendations.push(format!(
            "Письмо сфокусировано на 'Я/Мы', а не на клиенте (соотношение {:.1}). Переформулируйте в пользу 'Вы/Ваш бизнес'.",
            you_to_i_ratio
        ));
    }

    // Subject check (Optimum: 2..=5 words)
    if (2..=5).contains(&subject_word_count) {
        score += 10;
    } else if subject_word_count == 1 {
        score += 5;
        recommendations.push("Тема состоит из 1 слова. Добавьте контекст (2-4 слова).".to_string());
    } else if subject_word_count > 5 {
        score += 3;
        recommendations.push(format!(
            "Тема длинная ({} слов). На экранах мобильных телефонов обрезаются темы длиннее 30 символов.",
            subject_word_count
        ));
    } else {
        recommendations.push("Тема письма отсутствует.".to_string());
    }

    let grade = if score >= 90 {
        "A+".to_string()
    } else if score >= 75 {
        "A".to_string()
    } else if score >= 60 {
        "B".to_string()
    } else if score >= 45 {
        "C".to_string()
    } else {
        "F".to_string()
    };

    OutreachPsychologyReport {
        word_count,
        reading_time_sec,
        sentence_count,
        question_count,
        you_to_i_ratio,
        subject_words: subject_word_count,
        readability_score: score,
        grade,
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_outreach_email() {
        let subject = "Партнерство в разработке";
        let body = "Здравствуйте, Иван!\n\nИзучил ваш проект и заметил отличную архитектуру на Rust. Мы помогаем командам автоматизировать тестирование и ускорять релизы. Было бы вам интересно созвониться на 10 минут в четверг?\n\nС уважением,\nАлексей";
        let rep = analyze_outreach_psychology(subject, body);
        assert!(rep.readability_score >= 70);
        assert_eq!(rep.question_count, 1);
        assert!(rep.word_count >= 20 && rep.word_count <= 100);
        assert!(rep.grade == "A+" || rep.grade == "A" || rep.grade == "B");
    }

    #[test]
    fn test_bad_overloaded_email() {
        let subject = "Наше мега крутое предложение нашей супер компании по всем услугам в мире";
        let body = "Мы хотим вам рассказать про наши услуги. Мы делаем сайты. Мы пишем код. Мы лучшие на рынке. А вы что думаете? А когда созвонимся? А какой бюджет?";
        let rep = analyze_outreach_psychology(subject, body);
        assert_eq!(rep.question_count, 3);
        assert!(rep.recommendations.iter().any(|r| r.contains("вопросов")));
    }
}
