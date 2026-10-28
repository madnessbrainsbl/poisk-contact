use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtaFrictionReport {
    pub cta_excerpt: String,
    pub cta_tier: String,
    pub friction_score: u8,
    pub has_booking_link: bool,
    pub conversion_prediction: String,
    pub optimization_advice: String,
}

const SCHEDULER_DOMAINS: &[&str] = &[
    "calendly.com",
    "cal.com",
    "hubspot.com/meetings",
    "tidycal.com",
    "savvycal.com",
    "acuityscheduling.com",
    "doodle.com",
    "youcanbook.me",
];

const LOW_FRICTION_TRIGGERS: &[&str] = &[
    "стоит ли обсудить",
    "было бы полезно",
    "будет ли интересно",
    "интересно взглянуть",
    "имеет смысл",
    "актуальна ли тема",
    "worth exploring",
    "open to checking",
    "worth a chat",
    "does this sound relevant",
    "open to hearing more",
    "mind if i share",
    "open to connecting",
    "opposed to seeing",
];

const MEDIUM_FRICTION_TRIGGERS: &[&str] = &[
    "в четверг",
    "в пятницу",
    "на следующей неделе",
    "10 минут",
    "15 минут",
    "10-minute",
    "15 minutes",
    "free this thursday",
    "free tomorrow",
    "sometime next week",
    "brief call",
    "quick 10 min",
];

const HIGH_FRICTION_TRIGGERS: &[&str] = &[
    "демонстрация",
    "демо",
    "выберите слот",
    "запишитесь",
    "купите",
    "оформите",
    "30-minute demo",
    "45-minute",
    "book a time",
    "schedule a demo",
    "click below to buy",
    "sign up now",
    "calendar link",
    "my calendar",
    "забронируйте",
];

pub fn audit_cta_friction(text: &str) -> CtaFrictionReport {
    let clean = text.trim();
    if clean.is_empty() {
        return CtaFrictionReport {
            cta_excerpt: "".to_string(),
            cta_tier: "Empty CTA".to_string(),
            friction_score: 0,
            has_booking_link: false,
            conversion_prediction: "N/A".to_string(),
            optimization_advice: "Text is empty. Provide email body for analysis.".to_string(),
        };
    }

    let lower = clean.to_lowercase();
    let has_booking_link = SCHEDULER_DOMAINS.iter().any(|&dom| lower.contains(dom));

    // Extract closing paragraph as CTA excerpt
    let paragraphs: Vec<&str> = clean
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let cta_excerpt = paragraphs.last().copied().unwrap_or(clean).to_string();
    let lower_cta = cta_excerpt.to_lowercase();

    let mut is_low = LOW_FRICTION_TRIGGERS
        .iter()
        .any(|&trig| lower_cta.contains(trig));
    let is_medium = MEDIUM_FRICTION_TRIGGERS
        .iter()
        .any(|&trig| lower_cta.contains(trig));
    let mut is_high = HIGH_FRICTION_TRIGGERS
        .iter()
        .any(|&trig| lower_cta.contains(trig))
        || has_booking_link;

    if has_booking_link {
        is_high = true;
        is_low = false;
    }

    let (tier, friction_score, conversion, advice) = if is_high {
        (
            "High-Friction Hard Sell / Calendar Trap".to_string(),
            85,
            "Low (~3-6% Reply Rate) / High Spam/Archive Risk".to_string(),
            "Woodpecker & Gong.io Benchmark: Pitching an immediate meeting link (Calendly) or 30-min demo in Touch #1 imposes high cognitive effort on busy executives. Replace with interest-based permission CTA ('Worth exploring?').".to_string(),
        )
    } else if is_medium {
        (
            "Medium-Friction Specific Time Slot".to_string(),
            45,
            "Moderate (~18-22% Reply Rate)".to_string(),
            "Suggesting a specific micro-commitment (10 min on Thursday) works well for warm prospects, but can create calendar conflicts. Consider softening to an interest question for colder audiences.".to_string(),
        )
    } else if is_low {
        (
            "Low-Friction Interest-Based (Recommended)".to_string(),
            15,
            "High (~30-35% Reply Rate) / Maximum Engagement".to_string(),
            "Optimal cold outreach psychology: Asks for intellectual interest rather than time commitment. Decision-makers find it effortless to respond with a quick 'Yes' or 'Send details'.".to_string(),
        )
    } else {
        (
            "Neutral / Standard CTA".to_string(),
            30,
            "Average (~15% Reply Rate)".to_string(),
            "Closing statement is neutral. Ensure you end with a single, clear question to make replying frictionless.".to_string(),
        )
    };

    CtaFrictionReport {
        cta_excerpt,
        cta_tier: tier,
        friction_score,
        has_booking_link,
        conversion_prediction: conversion,
        optimization_advice: advice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_friction_interest_cta() {
        let text = "Здравствуйте, Алексей.\n\nЗаметили ваши проекты по безопасности.\n\nБудет ли вам интересно взглянуть на краткую презентацию решений?";
        let rep = audit_cta_friction(text);
        assert_eq!(rep.cta_tier, "Low-Friction Interest-Based (Recommended)");
        assert_eq!(rep.friction_score, 15);
        assert!(!rep.has_booking_link);
        assert!(rep.conversion_prediction.contains("High (~30-35%"));
    }

    #[test]
    fn test_high_friction_calendly_cta() {
        let text = "Добрый день.\n\nМы предлагаем комплексный аудит. Запишитесь на 45-минутное демо по ссылке: https://calendly.com/consultant/demo";
        let rep = audit_cta_friction(text);
        assert_eq!(rep.cta_tier, "High-Friction Hard Sell / Calendar Trap");
        assert!(rep.friction_score >= 80);
        assert!(rep.has_booking_link);
        assert!(rep.optimization_advice.contains("Woodpecker & Gong.io"));
    }

    #[test]
    fn test_medium_friction_specific_slot() {
        let text = "Приветствую.\n\nУдобно созвониться на 10 минут в четверг?";
        let rep = audit_cta_friction(text);
        assert_eq!(rep.cta_tier, "Medium-Friction Specific Time Slot");
        assert_eq!(rep.friction_score, 45);
    }
}
