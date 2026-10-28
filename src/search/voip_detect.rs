use crate::models::{ContactRecord, VoipClassification};
use crate::search::phone_intel::parse_and_enrich_phone;

const VOIP_CLOUD_OPERATORS: &[&str] = &[
    "манго",
    "mango",
    "задарма",
    "zadarma",
    "телфин",
    "telphin",
    "сипуни",
    "sipuni",
    "новофон",
    "novofon",
    "юис",
    "uis",
    "twilio",
    "voximplant",
    "транзиттелеком",
    "мтт",
    "зебра",
    "алло инкогнито",
];

const REAL_MOBILE_OPERATORS: &[&str] = &[
    "мтс",
    "мегафон",
    "билайн",
    "вымпелком",
    "tele2",
    "теле2",
    "ростелеком",
    "yota",
    "йота",
    "тинькофф",
    "т-мобайл",
    "сбермобайл",
];

pub fn classify_phone_line(raw_phone: &str) -> VoipClassification {
    let enriched = parse_and_enrich_phone(raw_phone);
    let carrier_lower = enriched.carrier.to_lowercase();
    let line_type_lower = enriched.line_type.to_lowercase();

    let mut is_voip = false;
    let mut is_real_mobile = false;

    for &voip_op in VOIP_CLOUD_OPERATORS {
        if carrier_lower.contains(voip_op) {
            is_voip = true;
            break;
        }
    }

    if !is_voip {
        for &mob_op in REAL_MOBILE_OPERATORS {
            if carrier_lower.contains(mob_op) && line_type_lower.contains("мобильн") {
                is_real_mobile = true;
                break;
            }
        }
    }

    let (category, desc) = if is_voip {
        (
            "Виртуальная АТС / VoIP".to_string(),
            "Облачная корпоративная АТС / Колл-центр (высокий риск робота/автоответчика)"
                .to_string(),
        )
    } else if is_real_mobile {
        (
            "Прямой мобильный (SIM)".to_string(),
            "Физический мобильный номер абонента (высокая ценность для WhatsApp/звонка)"
                .to_string(),
        )
    } else if line_type_lower.contains("городск") {
        (
            "Проводной городской".to_string(),
            "Стационарный офисный телефон компании".to_string(),
        )
    } else {
        (
            "Не определен".to_string(),
            "Стандартный телефонный номер".to_string(),
        )
    };

    VoipClassification {
        phone_number: enriched.formatted,
        is_voip_cloud_pbx: is_voip,
        is_real_mobile,
        carrier_category: category,
        description: desc,
    }
}

pub fn apply_voip_to_contact(contact: &mut ContactRecord) {
    if !contact.phone.is_empty() {
        let cls = classify_phone_line(&contact.phone);
        contact.phone_line_category = cls.carrier_category;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_mobile_detection() {
        let res = classify_phone_line("+7 (913) 555-01-02");
        assert!(res.is_real_mobile);
        assert!(!res.is_voip_cloud_pbx);
        assert_eq!(res.carrier_category, "Прямой мобильный (SIM)");
    }

    #[test]
    fn test_custom_voip_operator_check() {
        let mut raw = parse_and_enrich_phone("88001234567");
        raw.carrier = "Манго Телеком".to_string();
        let carrier_lower = raw.carrier.to_lowercase();
        let is_voip = VOIP_CLOUD_OPERATORS
            .iter()
            .any(|&v| carrier_lower.contains(v));
        assert!(is_voip);
    }
}
