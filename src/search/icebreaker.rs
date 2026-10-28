use crate::models::ContactRecord;

pub fn generate_dynamic_icebreaker(contact: &ContactRecord) -> String {
    // 1. Priority: Active hiring signals
    if !contact.hiring_roles.is_empty() {
        let roles = contact.hiring_roles.join(", ");
        if !contact.city.is_empty() {
            return format!(
                "Обратил внимание на ваши открытые вакансии ({}) в г. {} и активный рост команды",
                roles, contact.city
            );
        } else {
            return format!(
                "Обратил внимание на ваши открытые вакансии ({}) и масштабирование команды",
                roles
            );
        }
    }

    // 2. Priority: GitHub Developer profile
    if !contact.github_url.is_empty() {
        return format!(
            "Заметил ваш профиль разработчика на GitHub и технологические наработки компании {}",
            if contact.company.is_empty() {
                "вашей команды"
            } else {
                &contact.company
            }
        );
    }

    // 3. Priority: Specific CMS / CRM / Framework
    let cms = contact
        .tech_stack
        .iter()
        .find(|t| t.starts_with("CMS: "))
        .map(|t| t.trim_start_matches("CMS: "));
    let crm = contact
        .tech_stack
        .iter()
        .find(|t| t.starts_with("CRM: "))
        .map(|t| t.trim_start_matches("CRM: "));

    if let Some(c) = crm {
        return format!(
            "Обратил внимание на использование {} в цифровых процессах компании {}",
            c,
            if contact.company.is_empty() {
                "вашей команды"
            } else {
                &contact.company
            }
        );
    }

    if let Some(c) = cms {
        return format!(
            "Заметил развитие вашего веб-проекта на платформе {} у компании {}",
            c,
            if contact.company.is_empty() {
                "вашей команды"
            } else {
                &contact.company
            }
        );
    }

    // 4. Priority: Company description excerpt
    if !contact.company_description.is_empty() {
        let desc_clean = contact.company_description.trim();
        let short_desc: String = desc_clean.chars().take(80).collect();
        return format!("Впечатлен вашим фокусом на развитии: {}...", short_desc);
    }

    // 5. Fallback: Company + City
    if !contact.company.is_empty() && !contact.city.is_empty() {
        return format!(
            "Слежу за развитием проектов компании {} в г. {}",
            contact.company, contact.city
        );
    }

    // 6. Generic Fallback
    if !contact.company.is_empty() {
        format!(
            "Слежу за развитием и успехами компании {} на рынке",
            contact.company
        )
    } else {
        "С интересом ознакомился с деятельностью и проектами вашей команды".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LeadScore;

    #[test]
    fn test_icebreaker_with_hiring() {
        let mut c = ContactRecord::new(
            "Иван",
            "hr@acme.ru",
            "Acme Corp",
            "HR",
            "Красноярск",
            "IT",
            "",
            "",
            "Site",
            "https://acme.ru",
            LeadScore::Hot,
            true,
            "mx.acme.ru",
        );
        c.hiring_roles.push("Rust Разработчик".to_string());
        let ice = generate_dynamic_icebreaker(&c);
        assert!(ice.contains("Rust Разработчик"));
        assert!(ice.contains("Красноярск"));
    }

    #[test]
    fn test_icebreaker_with_crm() {
        let mut c = ContactRecord::new(
            "Ольга",
            "director@shop.ru",
            "Shop LLC",
            "CEO",
            "Москва",
            "Ecommerce",
            "",
            "",
            "Site",
            "https://shop.ru",
            LeadScore::Warm,
            true,
            "mx.shop.ru",
        );
        c.tech_stack.push("CRM: Bitrix24".to_string());
        let ice = generate_dynamic_icebreaker(&c);
        assert!(ice.contains("Bitrix24"));
    }
}
