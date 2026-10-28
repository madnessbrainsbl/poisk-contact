use crate::models::ContactRecord;
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct TaxProfile {
    pub inn: String,
    pub ogrn: String,
    pub kpp: String,
    pub vat_id: String,
    pub is_valid_inn: bool,
    pub entity_type: String, // "Юридическое лицо (ООО/АО)", "Индивидуальный предприниматель (ИП)", "Европейская компания (VAT)"
    pub registry_url: String,
}

pub fn extract_tax_identifiers(text: &str) -> TaxProfile {
    let mut profile = TaxProfile::default();

    // 1. Search Russian INN
    if let Ok(inn_re) = Regex::new(r"(?i)\bинн\s*[:№#]?\s*(\d{10}|\d{12})\b") {
        for cap in inn_re.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str();
                if validate_russian_inn(candidate) {
                    profile.inn = candidate.to_string();
                    profile.is_valid_inn = true;
                    if candidate.len() == 10 {
                        profile.entity_type = "Юридическое лицо (ООО/АО)".to_string();
                    } else {
                        profile.entity_type = "Индивидуальный предприниматель (ИП)".to_string();
                    }
                    profile.registry_url =
                        format!("https://www.rusprofile.ru/search?query={}", candidate);
                    break;
                }
            }
        }
    }

    // Fallback: search stand-alone 10 or 12 digit numbers near keywords "огрн" / "кпп" / "реквизиты"
    if profile.inn.is_empty() {
        if let Ok(general_re) = Regex::new(r"(?i)(?:реквизиты|плательщик).*?\b(\d{10}|\d{12})\b")
        {
            for cap in general_re.captures_iter(text) {
                if let Some(m) = cap.get(1) {
                    let cand = m.as_str();
                    if validate_russian_inn(cand) {
                        profile.inn = cand.to_string();
                        profile.is_valid_inn = true;
                        profile.entity_type = if cand.len() == 10 {
                            "Юридическое лицо (ООО/АО)".to_string()
                        } else {
                            "Индивидуальный предприниматель (ИП)".to_string()
                        };
                        profile.registry_url =
                            format!("https://www.rusprofile.ru/search?query={}", cand);
                        break;
                    }
                }
            }
        }
    }

    // 2. Search Russian OGRN (13 digits) or OGRNIP (15 digits)
    if let Ok(ogrn_re) = Regex::new(r"(?i)\bогрн(?:ип)?\s*[:№#]?\s*(\d{13}|\d{15})\b") {
        if let Some(cap) = ogrn_re.captures(text) {
            if let Some(m) = cap.get(1) {
                profile.ogrn = m.as_str().to_string();
            }
        }
    }

    // 3. Search Russian KPP (9 digits)
    if let Ok(kpp_re) = Regex::new(r"(?i)\bкпп\s*[:№#]?\s*(\d{9})\b") {
        if let Some(cap) = kpp_re.captures(text) {
            if let Some(m) = cap.get(1) {
                profile.kpp = m.as_str().to_string();
            }
        }
    }

    // 4. Search European VAT numbers
    let vat_patterns = [
        r"\b(DE\d{9})\b",
        r"\b(FR[0-9A-Z]{2}\d{9})\b",
        r"\b(GB\d{9}|GB\d{12})\b",
        r"\b(IT\d{11})\b",
        r"\b(NL\d{9}B\d{2})\b",
        r"\b(PL\d{10})\b",
        r"\b(ES[0-9A-Z]\d{7}[0-9A-Z])\b",
    ];

    for pat in vat_patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(text) {
                if let Some(m) = cap.get(1) {
                    profile.vat_id = m.as_str().to_string();
                    if profile.entity_type.is_empty() {
                        profile.entity_type = "Европейская компания (VAT)".to_string();
                        profile.registry_url = format!(
                            "https://ec.europa.eu/taxation_customs/vies/#/vat-validation?vatNumber={}",
                            profile.vat_id
                        );
                    }
                    break;
                }
            }
        }
    }

    profile
}

pub fn validate_russian_inn(inn: &str) -> bool {
    let digits: Vec<u32> = inn.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() == 10 {
        // Legal entity (10 digits)
        let weights = [2, 4, 10, 3, 5, 9, 4, 6, 8];
        let mut sum = 0;
        for i in 0..9 {
            sum += digits[i] * weights[i];
        }
        let check = (sum % 11) % 10;
        check == digits[9]
    } else if digits.len() == 12 {
        // Individual entrepreneur (12 digits)
        let weights_11 = [7, 2, 4, 10, 3, 5, 9, 4, 6, 8];
        let mut sum1 = 0;
        for i in 0..10 {
            sum1 += digits[i] * weights_11[i];
        }
        let check1 = (sum1 % 11) % 10;
        if check1 != digits[10] {
            return false;
        }

        let weights_12 = [3, 7, 2, 4, 10, 3, 5, 9, 4, 6, 8];
        let mut sum2 = 0;
        for i in 0..11 {
            sum2 += digits[i] * weights_12[i];
        }
        let check2 = (sum2 % 11) % 10;
        check2 == digits[11]
    } else {
        false
    }
}

pub fn apply_tax_profile_to_contact(profile: &TaxProfile, contact: &mut ContactRecord) {
    if !profile.inn.is_empty() && contact.inn.is_empty() {
        contact.inn = profile.inn.clone();
        if !contact.tags.iter().any(|t| t.starts_with("ИНН:")) {
            contact.tags.push(format!("ИНН: {}", profile.inn));
        }
    }
    if !profile.ogrn.is_empty() && contact.ogrn.is_empty() {
        contact.ogrn = profile.ogrn.clone();
    }
    if !profile.vat_id.is_empty() && contact.vat_id.is_empty() {
        contact.vat_id = profile.vat_id.clone();
        if !contact.tags.iter().any(|t| t.starts_with("VAT:")) {
            contact.tags.push(format!("VAT: {}", profile.vat_id));
        }
    }
    if !profile.entity_type.is_empty() && contact.legal_name.is_empty() {
        contact.legal_name = profile.entity_type.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_russian_inn_checksum() {
        // Known valid 10-digit INN (e.g. Yandex LLC: 7736207543)
        assert!(validate_russian_inn("7736207543"));
        // Invalid checksum
        assert!(!validate_russian_inn("7736207544"));
    }

    #[test]
    fn test_extract_requisites_from_footer() {
        let text =
            "Реквизиты компании: ООО 'Яндекс', ИНН 7736207543, ОГРН 1027700229193, КПП 773601001";
        let prof = extract_tax_identifiers(text);
        assert_eq!(prof.inn, "7736207543");
        assert_eq!(prof.ogrn, "1027700229193");
        assert_eq!(prof.kpp, "773601001");
        assert!(prof.is_valid_inn);
        assert!(prof.registry_url.contains("7736207543"));
    }
}
