#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhoneInfo {
    pub raw: String,
    pub e164: String,
    pub formatted: String,
    pub line_type: String,
    pub carrier: String,
    pub region: String,
    pub is_valid: bool,
    pub whatsapp_url: Option<String>,
    pub telegram_url: Option<String>,
}

pub fn parse_and_enrich_phone(raw: &str) -> PhoneInfo {
    let clean_raw = raw.trim().to_string();
    if clean_raw.is_empty() {
        return PhoneInfo::default();
    }

    let digits: String = clean_raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 {
        return PhoneInfo {
            raw: clean_raw,
            is_valid: false,
            ..Default::default()
        };
    }

    // National numbers need a country; never silently assign them Russia's +7.
    if !clean_raw.starts_with('+') && !(digits.len() == 11 && digits.starts_with('8')) {
        return PhoneInfo {
            raw: clean_raw.clone(),
            formatted: clean_raw,
            ..Default::default()
        };
    }
    // Russia / Kazakhstan
    if digits.len() == 11 && (digits.starts_with('7') || digits.starts_with('8')) {
        let national = &digits[1..];
        enrich_russian_phone(&clean_raw, national)
    } else if digits.len() == 11 && digits.starts_with('1') {
        // USA / Canada (+1)
        let e164 = format!("+{}", digits);
        let formatted = format!("+1 ({}) {}-{}", &digits[1..4], &digits[4..7], &digits[7..]);
        PhoneInfo {
            raw: clean_raw,
            e164: e164.clone(),
            formatted,
            line_type: "Международный (США/Канада)".to_string(),
            carrier: "Североамериканский оператор".to_string(),
            region: "США / Канада".to_string(),
            is_valid: true,
            whatsapp_url: Some(format!("https://wa.me/{}", digits)),
            telegram_url: Some(format!("tg://resolve?phone={}", digits)),
        }
    } else if digits.starts_with("375") && digits.len() == 12 {
        // Belarus (+375)
        let e164 = format!("+{}", digits);
        PhoneInfo {
            raw: clean_raw,
            e164: e164.clone(),
            formatted: format!(
                "+375 ({}) {}-{}-{}",
                &digits[3..5],
                &digits[5..8],
                &digits[8..10],
                &digits[10..12]
            ),
            line_type: "Международный (Беларусь)".to_string(),
            carrier: "Белорусский оператор".to_string(),
            region: "Беларусь".to_string(),
            is_valid: true,
            whatsapp_url: Some(format!("https://wa.me/{}", digits)),
            telegram_url: Some(format!("tg://resolve?phone={}", digits)),
        }
    } else {
        // General international fallback
        if digits.starts_with('0') || !(9..=15).contains(&digits.len()) {
            return PhoneInfo {
                raw: clean_raw.clone(),
                formatted: clean_raw,
                ..Default::default()
            };
        }
        let e164 = format!("+{}", digits);
        PhoneInfo {
            raw: clean_raw,
            e164: e164.clone(),
            formatted: e164.clone(),
            line_type: "Международный".to_string(),
            carrier: "Международный оператор".to_string(),
            region: "Международный".to_string(),
            is_valid: digits.len() >= 9 && digits.len() <= 15,
            whatsapp_url: Some(format!("https://wa.me/{}", digits)),
            telegram_url: Some(format!("tg://resolve?phone={}", digits)),
        }
    }
}

fn enrich_russian_phone(raw: &str, nat: &str) -> PhoneInfo {
    if nat.len() != 10 {
        return PhoneInfo {
            raw: raw.to_string(),
            is_valid: false,
            ..Default::default()
        };
    }

    let code = &nat[..3];
    let e164 = format!("+7{}", nat);
    let formatted = format!(
        "+7 ({}) {}-{}-{}",
        &nat[..3],
        &nat[3..6],
        &nat[6..8],
        &nat[8..10]
    );

    if code == "800" || code == "804" {
        return PhoneInfo {
            raw: raw.to_string(),
            e164: e164.clone(),
            formatted,
            line_type: "8-800 Бесплатный вызов".to_string(),
            carrier: "Федеральная бесплатная линия".to_string(),
            region: "Вся Россия".to_string(),
            is_valid: true,
            whatsapp_url: None,
            telegram_url: None,
        };
    }

    if code.starts_with('9') {
        let (carrier, region) = resolve_def_carrier_and_region(code);
        let wa_url = format!("https://wa.me/7{}", nat);
        let tg_url = format!("tg://resolve?phone=7{}", nat);
        return PhoneInfo {
            raw: raw.to_string(),
            e164,
            formatted,
            line_type: "Мобильный".to_string(),
            carrier,
            region,
            is_valid: true,
            whatsapp_url: Some(wa_url),
            telegram_url: Some(tg_url),
        };
    }

    let region = resolve_abc_region(code);
    PhoneInfo {
        raw: raw.to_string(),
        e164,
        formatted,
        line_type: "Городской".to_string(),
        carrier: "Городская фиксированная связь".to_string(),
        region,
        is_valid: true,
        whatsapp_url: None,
        telegram_url: None,
    }
}

fn resolve_def_carrier_and_region(code: &str) -> (String, String) {
    match code {
        "902" | "908" | "950" | "953" | "913" => (
            "МТС / Tele2".to_string(),
            "Сибирь (Красноярск / Новосибирск)".to_string(),
        ),
        "923" => (
            "МегаФон".to_string(),
            "Сибирский регион (Красноярск / Новосибирск)".to_string(),
        ),
        "903" | "905" | "906" | "909" | "960" | "961" | "962" | "963" | "964" | "965" | "967"
        | "968" => ("Билайн".to_string(), "Россия (Федеральный)".to_string()),
        "910" | "911" | "912" | "914" | "915" | "916" | "917" | "918" | "919" | "980" | "981"
        | "982" | "983" | "984" | "985" | "986" | "987" | "988" | "989" => {
            ("МТС".to_string(), "Россия (Федеральный)".to_string())
        }
        "920" | "921" | "922" | "924" | "925" | "926" | "927" | "928" | "929" | "930" | "931"
        | "932" | "933" | "936" | "937" | "938" | "939" => {
            ("МегаФон".to_string(), "Россия (Федеральный)".to_string())
        }
        "951" | "952" | "977" | "991" | "992" | "993" | "994" | "995" | "996" | "999" => (
            "Tele2 / Ростелеком / Т-Мобайл".to_string(),
            "Россия (Федеральный)".to_string(),
        ),
        _ => ("Мобильный оператор РФ".to_string(), "Россия".to_string()),
    }
}

fn resolve_abc_region(code: &str) -> String {
    match code {
        "391" => "Красноярск и Красноярский край".to_string(),
        "495" | "499" => "Москва и Московская область".to_string(),
        "812" => "Санкт-Петербург и Ленинградская область".to_string(),
        "383" => "Новосибирск и Новосибирская область".to_string(),
        "343" => "Екатеринбург и Свердловская область".to_string(),
        "831" => "Нижний Новгород".to_string(),
        "843" => "Казань и Республика Татарстан".to_string(),
        "846" => "Самара и Самарская область".to_string(),
        "861" => "Краснодар и Краснодарский край".to_string(),
        "863" => "Ростов-на-Дону и Ростовская область".to_string(),
        "351" => "Челябинск и Челябинская область".to_string(),
        "381" => "Омск и Омская область".to_string(),
        "342" => "Пермь и Пермский край".to_string(),
        "347" => "Уфа и Республика Башкортостан".to_string(),
        "385" => "Барнаул и Алтайский край".to_string(),
        "395" => "Иркутск и Иркутская область".to_string(),
        "382" => "Томск и Томская область".to_string(),
        "384" => "Кемерово и Кузбасс".to_string(),
        "423" => "Владивосток и Приморский край".to_string(),
        "421" => "Хабаровск и Хабаровский край".to_string(),
        "401" => "Калининград и область".to_string(),
        "862" => "Сочи".to_string(),
        _ => format!("Регион код {}", code),
    }
}

pub fn apply_phone_intel_to_contact(info: &PhoneInfo, contact: &mut crate::models::ContactRecord) {
    if !info.formatted.is_empty() {
        contact.phone = info.formatted.clone();
    }
    if !info.carrier.is_empty() {
        contact.phone_carrier = info.carrier.clone();
    }
    if !info.region.is_empty() {
        contact.phone_region = info.region.clone();
    }
    if !info.line_type.is_empty() {
        contact.phone_type = info.line_type.clone();
    }
    if let Some(wa) = &info.whatsapp_url {
        contact.whatsapp_url = wa.clone();
    }
    if let Some(tg) = &info.telegram_url {
        contact.telegram_url = tg.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_assign_a_country_to_local_foreign_numbers() {
        for raw in ["02072211680", "03088762090", "2072533232"] {
            let info = parse_and_enrich_phone(raw);
            assert!(!info.is_valid);
            assert!(info.e164.is_empty());
            assert_eq!(info.formatted, raw);
        }
        assert_eq!(
            parse_and_enrich_phone("+442073800034").e164,
            "+442073800034"
        );
        assert_eq!(
            parse_and_enrich_phone("+4930959999410").e164,
            "+4930959999410"
        );
    }

    #[test]
    fn test_mobile_enrichment() {
        let p = parse_and_enrich_phone("8 (913) 555-12-34");
        assert!(p.is_valid);
        assert_eq!(p.e164, "+79135551234");
        assert_eq!(p.formatted, "+7 (913) 555-12-34");
        assert_eq!(p.line_type, "Мобильный");
        assert_eq!(
            p.whatsapp_url,
            Some("https://wa.me/79135551234".to_string())
        );
        assert_eq!(
            p.telegram_url,
            Some("tg://resolve?phone=79135551234".to_string())
        );
    }

    #[test]
    fn test_landline_enrichment() {
        let p = parse_and_enrich_phone("+7 (391) 200-11-22");
        assert!(p.is_valid);
        assert_eq!(p.e164, "+73912001122");
        assert_eq!(p.line_type, "Городской");
        assert!(p.region.contains("Красноярск"));
        assert!(p.whatsapp_url.is_none());
    }

    #[test]
    fn test_tollfree_enrichment() {
        let p = parse_and_enrich_phone("8-800-555-35-35");
        assert!(p.is_valid);
        assert_eq!(p.line_type, "8-800 Бесплатный вызов");
        assert_eq!(p.region, "Вся Россия");
    }
}
