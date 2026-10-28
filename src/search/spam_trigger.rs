use crate::models::SpamCheckReport;

const SPAM_TRIGGERS: &[&str] = &[
    "100% бесплатно",
    "гарантия дохода",
    "без вложений",
    "быстрый заработок",
    "миллион рублей",
    "спешите",
    "только сегодня",
    "кликните сюда",
    "перейдите по ссылке",
    "легкие деньги",
    "без риска",
    "пассивный доход",
    "доступно всем",
    "увеличьте продажи в 10 раз",
    "специальное предложение",
    "жми сюда",
    "заработай прямо сейчас",
    "финансовая свобода",
    "удвойте прибыль",
    "нажми здесь",
    "100% free",
    "guaranteed income",
    "make money fast",
    "act now",
    "urgent",
    "click here",
    "click below",
    "buy now",
    "risk free",
    "risk-free",
    "winner",
    "limited time",
    "special promotion",
    "earn extra cash",
    "no obligation",
    "double your income",
    "apply now",
];

pub fn check_spam_triggers(subject: &str, body: &str) -> SpamCheckReport {
    let full_text = format!("{}\n{}", subject, body);
    let lower = full_text.to_lowercase();

    let mut trigger_words_found = Vec::new();
    let mut advice = Vec::new();
    let mut spam_score: u8 = 0;

    // 1. Scan for stop-words
    for &trigger in SPAM_TRIGGERS {
        if lower.contains(trigger) {
            trigger_words_found.push(trigger.to_string());
        }
    }

    if !trigger_words_found.is_empty() {
        let penalty = (trigger_words_found.len() * 15).min(50) as u8;
        spam_score = spam_score.saturating_add(penalty);
        advice.push(format!(
            "Обнаружены агрессивные спам-маркеры ({} шт.): {}. Почтовые фильтры (SpamAssassin, Google) отправят письмо в Спам.",
            trigger_words_found.len(),
            trigger_words_found.join(", ")
        ));
    }

    // 2. CAPS check
    let letters: Vec<char> = full_text.chars().filter(|c| c.is_alphabetic()).collect();
    let caps_count = letters.iter().filter(|c| c.is_uppercase()).count();
    let caps_ratio = if !letters.is_empty() {
        caps_count as f32 / letters.len() as f32
    } else {
        0.0
    };

    if caps_ratio > 0.20 {
        spam_score = spam_score.saturating_add(25);
        advice.push(format!(
            "Высокая доля заглавных букв ({:.1}% CAPS). Спам-фильтры снижают репутацию писем с 'кричащими' заголовками.",
            caps_ratio * 100.0
        ));
    }

    // 3. Exclamation marks check
    let exclamation_count = full_text.chars().filter(|&c| c == '!').count();
    if exclamation_count >= 3 {
        let penalty = (exclamation_count * 5).min(25) as u8;
        spam_score = spam_score.saturating_add(penalty);
        advice.push(format!(
            "Слишком много восклицательных знаков ({} шт.). В B2B переписке используйте максимум 0-1 знак восклицания.",
            exclamation_count
        ));
    }

    // 4. Subject spam words check
    let lower_subj = subject.to_lowercase();
    if lower_subj.contains("срочно") || lower_subj.contains("urgent") || lower_subj.contains("re:")
    {
        spam_score = spam_score.saturating_add(15);
        advice.push(
            "Тема содержит манипулятивные триггеры ('Срочно', 'Re:' без предыдущей переписки)."
                .to_string(),
        );
    }

    let is_high_risk = spam_score >= 35;

    let has_emojis = full_text
        .chars()
        .any(|c| (c >= '\u{1F300}' && c <= '\u{1FAFF}') || (c >= '\u{2600}' && c <= '\u{27BF}'));
    let body_len = body.chars().count();
    let is_long_promo = body_len > 750;
    let has_bullet_points =
        body.contains('—') || body.contains("- ") || body.contains('•') || body.contains("* ");

    let (gmail_risk_level, gmail_warning_reason) = if has_emojis
        || is_long_promo
        || !trigger_words_found.is_empty()
        || exclamation_count >= 2
    {
        let mut reasons = Vec::new();
        if is_long_promo {
            reasons.push(format!("длина {} симв. (>750)", body_len));
        }
        if has_emojis {
            reasons.push("эмодзи в тексте".to_string());
        }
        if has_bullet_points && is_long_promo {
            reasons.push("маркетинговый перечень услуг".to_string());
        }
        if !trigger_words_found.is_empty() {
            reasons.push(format!("триггер-слова: {}", trigger_words_found.join(", ")));
        }
        (
            "Google/Gmail: Риск спама".to_string(),
            Some(format!(
                "В Mail.ru и Яндекс такой текст проходит во Входящие. Но спам-фильтры Google сочтут его промо-рассылкой из-за: {}. Для Gmail сократите текст до 3-4 предложений без эмодзи.",
                reasons.join(", ")
            )),
        )
    } else {
        ("Google/Gmail: Во Входящие (B2B)".to_string(), None)
    };

    let mailru_deliverability = if trigger_words_found.is_empty() && caps_ratio < 0.25 {
        "Mail.ru & Яндекс: Во Входящие".to_string()
    } else {
        "Mail.ru & Яндекс: Возможен спам".to_string()
    };

    SpamCheckReport {
        spam_score,
        trigger_words_found,
        caps_ratio,
        exclamation_count,
        is_high_risk,
        advice,
        mailru_deliverability,
        gmail_risk_level,
        gmail_warning_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_b2b_email_spam_check() {
        let subject = "Партнерство по разработке ПО";
        let body = "Добрый день, Алексей. Изучил ваш проект и хотел предложить обсудить оптимизацию процессов.";
        let rep = check_spam_triggers(subject, body);
        assert_eq!(rep.spam_score, 0);
        assert!(!rep.is_high_risk);
        assert!(rep.trigger_words_found.is_empty());
    }

    #[test]
    fn test_aggressive_spam_detection() {
        let subject = "СРОЧНО! 100% БЕСПЛАТНО ТОЛЬКО СЕГОДНЯ!!!";
        let body = "Гарантия дохода без вложений! Кликните сюда прямо сейчас чтобы заработать миллион рублей!";
        let rep = check_spam_triggers(subject, body);
        assert!(rep.is_high_risk);
        assert!(rep.spam_score >= 60);
        assert!(
            rep.trigger_words_found
                .iter()
                .any(|t| t.contains("100% бесплатно"))
        );
        assert!(
            rep.trigger_words_found
                .iter()
                .any(|t| t.contains("кликните сюда"))
        );
        assert!(
            rep.advice
                .iter()
                .any(|a| a.contains("CAPS") || a.contains("маркеры"))
        );
    }
}
