use crate::models::ContactRecord;

pub fn generate_smart_hook(contact: &ContactRecord) -> String {
    let company_name = if !contact.company.is_empty() {
        &contact.company
    } else {
        "вашей компании"
    };

    // 1. Hiring Trigger Signal (Maximum relevance)
    if contact.hiring_active && !contact.hiring_roles.is_empty() {
        let role = &contact.hiring_roles[0];
        return format!(
            "Обратил внимание на открытую вакансию '{}' в {} — масштабирование команды сейчас наверняка в фокусе внимания.",
            role, company_name
        );
    }

    // 2. Tech Stack Signal
    if !contact.tech_stack.is_empty() {
        let tech = contact.tech_stack.join(", ");
        return format!(
            "Заметил отличную технологическую базу в {} ({}) — как раз специализируемся на решениях в этой экосистеме.",
            company_name, tech
        );
    }

    // 3. Seniority & Department persona tailoring
    let lower_role = contact.role.to_lowercase();
    let lower_seniority = contact.seniority.to_lowercase();

    if lower_seniority.contains("c-level")
        || lower_role.contains("директор")
        || lower_role.contains("ceo")
    {
        if !contact.city.is_empty() && contact.city != "Не указан" {
            format!(
                "Слежу за ростом бизнеса {} в г. {} — импонирует системный подход к развитию проектов.",
                company_name, contact.city
            )
        } else {
            format!(
                "Изучил траекторию развития {} — ценю взвешенный стратегический фокус руководства.",
                company_name
            )
        }
    } else if lower_seniority.contains("head")
        || lower_seniority.contains("vp")
        || lower_role.contains("руководитель")
    {
        format!(
            "Вижу качественную реализацию проектов в зоне ответственности {} — отличный уровень координации команды.",
            company_name
        )
    } else if lower_role.contains("hr") || lower_role.contains("рекрутер") {
        format!(
            "Отслеживаю активность HR-направления {} по привлечению сильных специалистов на рынке.",
            company_name
        )
    } else if !contact.city.is_empty() && contact.city != "Не указан" {
        format!(
            "Слежу за проектами компании {} на рынке г. {}.",
            company_name, contact.city
        )
    } else {
        format!(
            "Изучил последние обновления и проекты компании {}.",
            company_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_hook_with_hiring_signal() {
        let mut contact = ContactRecord::new(
            "Иван",
            "ivan@tech.ru",
            "TechLab",
            "CTO",
            "Москва",
            "Rust",
            "",
            "",
            "web",
            "https://tech.ru",
            crate::models::LeadScore::Hot,
            true,
            "mx.tech.ru",
        );
        contact.hiring_active = true;
        contact.hiring_roles = vec!["Senior Rust Developer".to_string()];

        let hook = generate_smart_hook(&contact);
        assert!(hook.contains("Senior Rust Developer"));
        assert!(hook.contains("TechLab"));
    }

    #[test]
    fn test_smart_hook_with_c_level_persona() {
        let mut contact = ContactRecord::new(
            "Анна",
            "anna@fashion.ru",
            "FashionBrand",
            "Генеральный директор",
            "Красноярск",
            "",
            "",
            "",
            "web",
            "https://fashion.ru",
            crate::models::LeadScore::Warm,
            true,
            "mx.fashion.ru",
        );
        contact.seniority = "C-Level / Фаундер".to_string();

        let hook = generate_smart_hook(&contact);
        assert!(hook.contains("FashionBrand"));
        assert!(hook.contains("Красноярск"));
    }
}
