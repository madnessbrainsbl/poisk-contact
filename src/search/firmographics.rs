use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct FirmographicProfile {
    pub legal_name: String,
    pub legal_name_explicit: bool,
    pub description: String,
    pub address: String,
    pub country: String,
    pub locality: String,
    pub phone: String,
    pub email: String,
    pub socials: Vec<String>,
}

fn decode_basic_html_entities(input: &str) -> String {
    input
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&laquo;", "«")
        .replace("&raquo;", "»")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&ndash;", "-")
        .replace("&mdash;", "-")
        .replace("\n", " ")
        .replace("\r", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn extract_firmographics(html: &str) -> FirmographicProfile {
    let mut profile = FirmographicProfile::default();

    // 1. Parse JSON-LD Schema.org scripts
    if let Ok(json_ld_re) =
        Regex::new(r#"(?is)<script[^>]*type=["']application/ld\+json["'][^>]*>(.*?)</script>"#)
    {
        for cap in json_ld_re.captures_iter(html) {
            let json_str = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if json_str.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                inspect_json_ld_node(&val, &mut profile);
            }
        }
    }

    // 2. OpenGraph & Meta fallbacks
    if profile.description.is_empty() {
        if let Ok(og_desc_re) = Regex::new(
            r#"(?i)<meta[^>]+(?:property|name)=["'](?:og:description|description)["'][^>]+content=["']([^"']+)["']"#,
        ) {
            if let Some(cap) = og_desc_re.captures(html) {
                if let Some(m) = cap.get(1) {
                    profile.description = decode_basic_html_entities(m.as_str());
                }
            }
        }
        // Fallback for reversed attributes: content="..." property="og:description"
        if profile.description.is_empty() {
            if let Ok(rev_re) = Regex::new(
                r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+(?:property|name)=["'](?:og:description|description)["']"#,
            ) {
                if let Some(cap) = rev_re.captures(html) {
                    if let Some(m) = cap.get(1) {
                        profile.description = decode_basic_html_entities(m.as_str());
                    }
                }
            }
        }
    }

    if profile.legal_name.is_empty() {
        if let Ok(og_site_re) = Regex::new(
            r#"(?i)<meta[^>]+(?:property|name)=["']og:site_name["'][^>]+content=["']([^"']+)["']"#,
        ) {
            if let Some(cap) = og_site_re.captures(html) {
                if let Some(m) = cap.get(1) {
                    profile.legal_name = decode_basic_html_entities(m.as_str());
                }
            }
        }
    }

    // Limit length to keep database clean (char count safe for UTF-8 Cyrillic)
    if profile.address.is_empty() {
        if let Some((address, locality, country)) = plain_postal_address(html) {
            profile.address = address;
            profile.locality = locality;
            profile.country = country;
        }
    }
    if profile.description.chars().count() > 300 {
        profile.description = format!(
            "{}...",
            profile.description.chars().take(297).collect::<String>()
        );
    }
    if profile.legal_name.chars().count() > 100 {
        profile.legal_name = profile.legal_name.chars().take(100).collect();
    }
    if profile.address.chars().count() > 150 {
        profile.address = profile.address.chars().take(150).collect();
    }

    profile
}

fn plain_postal_address(html: &str) -> Option<(String, String, String)> {
    let paragraphs = Regex::new(r"(?is)<(?:p|address)\b[^>]*>(.*?)</(?:p|address)\s*>").unwrap();
    let breaks = Regex::new(r"(?i)<br\s*/?>").unwrap();
    let tags = Regex::new(r"<[^>]*>").unwrap();
    let street = Regex::new(r"^\d{1,6}\s+[A-Za-z].*").unwrap();
    for cap in paragraphs.captures_iter(html) {
        let lines: Vec<_> = breaks
            .split(&cap[1])
            .map(|line| decode_basic_html_entities(&tags.replace_all(line, " ")))
            .filter(|line| !line.is_empty())
            .collect();
        if lines.len() < 3 || !street.is_match(&lines[0]) {
            continue;
        }
        let Some((locality, _region)) = lines[1].split_once(',') else {
            continue;
        };
        let Some((_postal, country)) = lines.last().unwrap().rsplit_once(',') else {
            continue;
        };
        if locality
            .trim()
            .chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '-')
            && country
                .trim()
                .chars()
                .all(|c| c.is_alphabetic() || c == ' ')
            && !locality.trim().is_empty()
            && !country.trim().is_empty()
        {
            return Some((
                lines.join(", "),
                locality.trim().into(),
                country.trim().into(),
            ));
        }
    }
    None
}

fn inspect_json_ld_node(val: &Value, profile: &mut FirmographicProfile) {
    match val {
        Value::Object(map) => {
            // Check @graph array
            if let Some(graph) = map.get("@graph").and_then(|g| g.as_array()) {
                for item in graph {
                    inspect_json_ld_node(item, profile);
                }
            }

            let type_str = map
                .get("@type")
                .map(Value::to_string)
                .unwrap_or_default()
                .to_lowercase();

            let is_org = type_str.contains("organization")
                || type_str.contains("localbusiness")
                || type_str.contains("corporation")
                || type_str.contains("company")
                || type_str.contains("store")
                || type_str.contains("service")
                || type_str.contains("website")
                || map.get("address").is_some_and(Value::is_object);

            if is_org {
                if profile.legal_name.is_empty() {
                    if let Some(legal) = map.get("legalName").and_then(|v| v.as_str()) {
                        profile.legal_name = decode_basic_html_entities(legal);
                        profile.legal_name_explicit = true;
                    } else if let Some(name) = map.get("name").and_then(|v| v.as_str()) {
                        profile.legal_name = decode_basic_html_entities(name);
                    }
                }

                if profile.description.is_empty() {
                    if let Some(desc) = map.get("description").and_then(|v| v.as_str()) {
                        profile.description = decode_basic_html_entities(desc);
                    }
                }

                if profile.phone.is_empty() {
                    if let Some(tel) = map.get("telephone").and_then(|v| v.as_str()) {
                        profile.phone = tel.trim().to_string();
                    }
                }

                if profile.email.is_empty() {
                    if let Some(em) = map.get("email").and_then(|v| v.as_str()) {
                        profile.email = em.trim().to_lowercase();
                    }
                }

                if profile.address.is_empty() {
                    if let Some(addr_val) = map.get("address") {
                        if let Some(addr_str) = addr_val.as_str() {
                            profile.address = decode_basic_html_entities(addr_str);
                        } else if let Some(addr_obj) = addr_val.as_object() {
                            if let Some(country) = addr_obj.get("addressCountry") {
                                profile.country = country
                                    .as_str()
                                    .or_else(|| country.get("name").and_then(Value::as_str))
                                    .unwrap_or("")
                                    .to_string();
                            }
                            let locality = addr_obj
                                .get("addressLocality")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            profile.locality = decode_basic_html_entities(locality);
                            let street = addr_obj
                                .get("streetAddress")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let region = addr_obj
                                .get("addressRegion")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let mut parts = Vec::new();
                            if !region.is_empty() {
                                parts.push(region);
                            }
                            if !locality.is_empty() {
                                parts.push(locality);
                            }
                            if !street.is_empty() {
                                parts.push(street);
                            }
                            if !profile.country.is_empty() {
                                parts.push(&profile.country);
                            }
                            if !parts.is_empty() {
                                profile.address = decode_basic_html_entities(&parts.join(", "));
                            }
                        }
                    }
                }

                if let Some(same_as) = map.get("sameAs") {
                    if let Some(arr) = same_as.as_array() {
                        for s in arr {
                            if let Some(url_str) = s.as_str() {
                                let trimmed = url_str.trim().to_string();
                                if !trimmed.is_empty() && !profile.socials.contains(&trimmed) {
                                    profile.socials.push(trimmed);
                                }
                            }
                        }
                    } else if let Some(single) = same_as.as_str() {
                        let trimmed = single.trim().to_string();
                        if !trimmed.is_empty() && !profile.socials.contains(&trimmed) {
                            profile.socials.push(trimmed);
                        }
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                inspect_json_ld_node(item, profile);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_local_business_subtypes_and_structured_country() {
        for kind in [r#""Dentist""#, r#"["MedicalBusiness", "Dentist"]"#] {
            let html = format!(
                r#"<script type="application/ld+json">{{"@type":{kind},"name":"Berlin Clinic","address":{{"@type":"PostalAddress","addressLocality":"Berlin","addressCountry":{{"@type":"Country","name":"DE"}},"streetAddress":"Clinic Street 1"}}}}</script>"#
            );
            let result = extract_firmographics(&html);
            assert!(
                result.address.contains("Berlin"),
                "Missing business address: {result:?}"
            );
            assert!(result.address.contains("DE"), "Missing country: {result:?}");
        }
    }

    #[test]
    fn test_extract_json_ld() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "Organization",
                    "name": "ООО СпецСтройМонтаж",
                    "legalName": "Общество с ограниченной ответственностью СпецСтройМонтаж",
                    "description": "Производство высокоточных металлоконструкций и каркасов",
                    "telephone": "+7 (391) 200-11-22",
                    "email": "info@specstroy.ru",
                    "address": {
                        "@type": "PostalAddress",
                        "addressLocality": "Красноярск",
                        "streetAddress": "ул. Телевизорная, 1"
                    },
                    "sameAs": [
                        "https://vk.com/specstroy",
                        "https://t.me/specstroy_krsk"
                    ]
                }
                </script>
            </head>
            <body></body>
            </html>
        "#;

        let res = extract_firmographics(html);
        assert_eq!(
            res.legal_name,
            "Общество с ограниченной ответственностью СпецСтройМонтаж"
        );
        assert_eq!(
            res.description,
            "Производство высокоточных металлоконструкций и каркасов"
        );
        assert_eq!(res.phone, "+7 (391) 200-11-22");
        assert_eq!(res.email, "info@specstroy.ru");
        assert!(res.address.contains("Красноярск"));
        assert!(
            res.socials
                .contains(&"https://vk.com/specstroy".to_string())
        );
    }

    #[test]
    fn test_extract_opengraph_fallback() {
        let html = r#"
            <html>
            <head>
                <meta property="og:site_name" content="Модельное Агентство Люкс">
                <meta property="og:description" content="Организация коммерческих съемок и лукбуков для брендов">
            </head>
            <body></body>
            </html>
        "#;

        let res = extract_firmographics(html);
        assert_eq!(res.legal_name, "Модельное Агентство Люкс");
        assert_eq!(
            res.description,
            "Организация коммерческих съемок и лукбуков для брендов"
        );
    }
}
