use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplyCategory {
    InterestedPositive,
    HardObjection,
    ReferralWrongPerson,
    OutOfOffice,
    UncertainNeutral,
}

impl std::fmt::Display for ReplyCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplyCategory::InterestedPositive => write!(f, "Interested / Positive Meeting Request"),
            ReplyCategory::HardObjection => write!(f, "Hard Objection / Do Not Contact"),
            ReplyCategory::ReferralWrongPerson => {
                write!(f, "Referral / Wrong Contact (Route to Colleague)")
            }
            ReplyCategory::OutOfOffice => write!(f, "Out of Office / Auto-Responder"),
            ReplyCategory::UncertainNeutral => write!(f, "Neutral Inquiry / General Response"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplySentimentReport {
    pub raw_reply: String,
    pub category: ReplyCategory,
    pub sentiment_score: i8,
    pub extracted_referral_target: Option<String>,
    pub cadence_action: String,
    pub handling_recommendation: String,
}

pub fn classify_sales_reply(text: &str) -> ReplySentimentReport {
    let clean = text.trim();
    if clean.is_empty() {
        return ReplySentimentReport {
            raw_reply: "".to_string(),
            category: ReplyCategory::UncertainNeutral,
            sentiment_score: 0,
            extracted_referral_target: None,
            cadence_action: "MANUAL_REVIEW".to_string(),
            handling_recommendation: "Empty reply text.".to_string(),
        };
    }

    let lower = clean.to_lowercase();

    // 1. Out of Office
    let is_ooo = lower.contains("out of office")
        || lower.contains("automatic reply")
        || lower.contains("auto-reply")
        || lower.contains("автоответ")
        || lower.contains("в отпуске")
        || lower.contains("в командировке")
        || lower.contains("вернусь")
        || lower.contains("on annual leave")
        || lower.contains("limited access to email");

    if is_ooo {
        return ReplySentimentReport {
            raw_reply: clean.to_string(),
            category: ReplyCategory::OutOfOffice,
            sentiment_score: 0,
            extracted_referral_target: None,
            cadence_action: "PAUSE_AND_RESCHEDULE_AFTER_RETURN".to_string(),
            handling_recommendation: "Auto-responder detected. Pause sequence and schedule follow-up 3 business days after expected return date.".to_string(),
        };
    }

    // 2. Referral / Wrong Person
    let is_referral = lower.contains("not the right person")
        || lower.contains("reach out to")
        || lower.contains("contact our")
        || lower.contains("forward to")
        || lower.contains("обратитесь к")
        || lower.contains("напишите нашему")
        || lower.contains("перешлите")
        || lower.contains("этим занимается")
        || lower.contains("не занимаюсь этим");

    if is_referral {
        // Try extracting email or name if mentioned
        let referral_target = clean
            .split_whitespace()
            .find(|word| word.contains('@'))
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_')
                    .to_string()
            });

        return ReplySentimentReport {
            raw_reply: clean.to_string(),
            category: ReplyCategory::ReferralWrongPerson,
            sentiment_score: 25,
            extracted_referral_target: referral_target.clone(),
            cadence_action: "BRANCH_TO_NEW_REFERRAL_CONTACT".to_string(),
            handling_recommendation: format!(
                "Lead provided a referral{}. Branch outreach cadence to the designated colleague mentioning the internal introduction.",
                if let Some(ref t) = referral_target {
                    format!(" ({})", t)
                } else {
                    "".to_string()
                }
            ),
        };
    }

    // 3. Hard Objection / Unsubscribe
    let is_hard_objection = lower.contains("not interested")
        || lower.contains("unsubscribe")
        || lower.contains("remove me")
        || lower.contains("stop emailing")
        || lower.contains("no thanks")
        || lower.contains("не интересно")
        || lower.contains("не актуальн")
        || lower.contains("отпишите")
        || lower.contains("удалите")
        || lower.contains("больше не пишите")
        || lower.contains("уже есть подрядчик")
        || lower.contains("не беспокойте");

    if is_hard_objection {
        return ReplySentimentReport {
            raw_reply: clean.to_string(),
            category: ReplyCategory::HardObjection,
            sentiment_score: -80,
            extracted_referral_target: None,
            cadence_action: "SUPPRESS_PERMANENTLY".to_string(),
            handling_recommendation: "Definitive negative objection. Terminate sequence immediately and add contact to global suppression list to prevent spam flagging.".to_string(),
        };
    }

    // 4. Interested / Positive Meeting Request
    let is_interested = lower.contains("sounds good")
        || lower.contains("let's talk")
        || lower.contains("let's chat")
        || lower.contains("send me details")
        || lower.contains("send more info")
        || lower.contains("happy to connect")
        || lower.contains("available")
        || lower.contains("давайте созвонимся")
        || lower.contains("пришлите кп")
        || lower.contains("пришлите презентацию")
        || lower.contains("присылайте")
        || lower.contains("интересно")
        || lower.contains("готов обсудить")
        || lower.contains("удобно во вторник")
        || lower.contains("удобно в четверг");

    if is_interested {
        return ReplySentimentReport {
            raw_reply: clean.to_string(),
            category: ReplyCategory::InterestedPositive,
            sentiment_score: 90,
            extracted_referral_target: None,
            cadence_action: "PAUSE_CADENCE_HANDOFF_TO_ACCOUNT_EXECUTIVE".to_string(),
            handling_recommendation: "High-value sales opportunity detected. Immediately pause automated bot sequence and alert Account Executive / SDR for human response within 15 minutes.".to_string(),
        };
    }

    // 5. Neutral
    ReplySentimentReport {
        raw_reply: clean.to_string(),
        category: ReplyCategory::UncertainNeutral,
        sentiment_score: 10,
        extracted_referral_target: None,
        cadence_action: "FLAG_FOR_MANUAL_REVIEW".to_string(),
        handling_recommendation:
            "Ambiguous response. Flag for operator review before executing next automated touch."
                .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_interested_positive() {
        let rep = classify_sales_reply(
            "Здравствуйте! Тема интересная, давайте созвонимся на этой неделе.",
        );
        assert_eq!(rep.category, ReplyCategory::InterestedPositive);
        assert!(rep.sentiment_score > 50);
        assert_eq!(
            rep.cadence_action,
            "PAUSE_CADENCE_HANDOFF_TO_ACCOUNT_EXECUTIVE"
        );
    }

    #[test]
    fn test_classify_hard_objection() {
        let rep = classify_sales_reply("Тема нам не актуальна, отпишите пожалуйста из вашей базы.");
        assert_eq!(rep.category, ReplyCategory::HardObjection);
        assert!(rep.sentiment_score < 0);
        assert_eq!(rep.cadence_action, "SUPPRESS_PERMANENTLY");
    }

    #[test]
    fn test_classify_referral_wrong_person() {
        let rep = classify_sales_reply(
            "Я не занимаюсь закупками, обратитесь к нашему техническому директору: cto@company.ru",
        );
        assert_eq!(rep.category, ReplyCategory::ReferralWrongPerson);
        assert_eq!(
            rep.extracted_referral_target,
            Some("cto@company.ru".to_string())
        );
    }

    #[test]
    fn test_classify_out_of_office() {
        let rep = classify_sales_reply(
            "Auto-reply: I am out of the office on annual leave until October 20th.",
        );
        assert_eq!(rep.category, ReplyCategory::OutOfOffice);
        assert_eq!(rep.sentiment_score, 0);
    }
}
