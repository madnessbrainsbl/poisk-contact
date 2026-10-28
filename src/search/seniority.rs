use crate::models::ContactRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeniorityLevel {
    CLevel,
    VPHead,
    Director,
    Manager,
    Specialist,
    Unknown,
}

impl SeniorityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeniorityLevel::CLevel => "C-Level / Фаундер",
            SeniorityLevel::VPHead => "VP / Head / Руководитель направления",
            SeniorityLevel::Director => "Директор / Управляющий",
            SeniorityLevel::Manager => "Менеджер / Лид",
            SeniorityLevel::Specialist => "Специалист / Исполнитель",
            SeniorityLevel::Unknown => "Не определен",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Department {
    Executive,
    Engineering,
    SalesMarketing,
    HumanResources,
    FinanceLegal,
    Operations,
    General,
}

impl Department {
    pub fn as_str(&self) -> &'static str {
        match self {
            Department::Executive => "Руководство / Высший менеджмент",
            Department::Engineering => "IT / Разработка / Инженерия",
            Department::SalesMarketing => "Продажи / Маркетинг / Рост",
            Department::HumanResources => "HR / Рекрутинг / Таланты",
            Department::FinanceLegal => "Финансы / Юриспруденция",
            Department::Operations => "Операции / Логистика / Поддержка",
            Department::General => "Общий профиль",
        }
    }
}

pub fn classify_title(raw_title: &str) -> (SeniorityLevel, Department) {
    let lower = raw_title.to_lowercase();

    // 1. Seniority Level Detection
    let seniority = if lower.contains("ceo")
        || lower.contains("cto")
        || lower.contains("cfo")
        || lower.contains("cmo")
        || lower.contains("coo")
        || lower.contains("cio")
        || lower.contains("cpo")
        || lower.contains("founder")
        || lower.contains("co-founder")
        || lower.contains("сооснователь")
        || lower.contains("основатель")
        || lower.contains("учредитель")
        || lower.contains("генеральный директор")
        || lower.contains("гендиректор")
        || lower.contains("президент")
        || lower.contains("владелец")
        || lower.contains("owner")
    {
        SeniorityLevel::CLevel
    } else if lower.contains("vp")
        || lower.contains("vice president")
        || lower.contains("вице-президент")
        || lower.contains("head of")
        || lower.contains("head ")
        || lower.contains("руководитель направления")
        || lower.contains("руководитель отдела")
        || lower.contains("глава ")
    {
        SeniorityLevel::VPHead
    } else if lower.contains("director")
        || lower.contains("директор")
        || lower.contains("управляющий")
    {
        SeniorityLevel::Director
    } else if lower.contains("team lead")
        || lower.contains("teamlead")
        || lower.contains("тимлид")
        || lower.contains("lead ")
        || lower.contains("ведущий")
        || lower.contains("старший")
        || lower.contains("senior")
        || lower.contains("менеджер")
        || lower.contains("manager")
    {
        SeniorityLevel::Manager
    } else if lower.contains("developer")
        || lower.contains("разработчик")
        || lower.contains("engineer")
        || lower.contains("инженер")
        || lower.contains("designer")
        || lower.contains("дизайнер")
        || lower.contains("специалист")
        || lower.contains("specialist")
        || lower.contains("аналитик")
        || lower.contains("analyst")
        || lower.contains("рекрутер")
        || lower.contains("recruiter")
        || lower.contains("копирайтер")
    {
        SeniorityLevel::Specialist
    } else {
        SeniorityLevel::Unknown
    };

    // 2. Department Detection
    let department = if lower.contains("ceo")
        || lower.contains("генеральный")
        || lower.contains("founder")
        || lower.contains("основатель")
        || lower.contains("учредитель")
        || lower.contains("президент")
        || lower.contains("владелец")
        || lower.contains("руководство")
    {
        Department::Executive
    } else if lower.contains("hr")
        || lower.contains("кадр")
        || lower.contains("talent")
        || lower.contains("рекрут")
        || lower.contains("найм")
        || lower.contains("people")
        || lower.contains("персонал")
    {
        Department::HumanResources
    } else if lower.contains("cto")
        || lower.contains("developer")
        || lower.contains("разработчик")
        || lower.contains("программист")
        || lower.contains("engineer")
        || lower.contains("инженер")
        || lower.contains("архитектор")
        || lower.contains("devops")
        || lower.contains("qa")
        || lower.contains("it ")
        || lower.contains("разработк")
        || lower.contains("информацион")
        || lower.contains("software")
    {
        Department::Engineering
    } else if lower.contains("cmo")
        || lower.contains("market")
        || lower.contains("маркет")
        || lower.contains("growth")
        || lower.contains("sales")
        || lower.contains("продаж")
        || lower.contains("commercial")
        || lower.contains("коммерческ")
        || lower.contains("pr ")
        || lower.contains("smm")
        || lower.contains("контент")
        || lower.contains("трафик")
    {
        Department::SalesMarketing
    } else if lower.contains("cfo")
        || lower.contains("финанс")
        || lower.contains("бухгалтер")
        || lower.contains("юрист")
        || lower.contains("legal")
        || lower.contains("право")
        || lower.contains("lawyer")
        || lower.contains("комплаенс")
    {
        Department::FinanceLegal
    } else if lower.contains("coo")
        || lower.contains("операцион")
        || lower.contains("логистик")
        || lower.contains("закуп")
        || lower.contains("supply")
        || lower.contains("склад")
    {
        Department::Operations
    } else {
        Department::General
    };

    (seniority, department)
}

pub fn apply_seniority_to_contact(contact: &mut ContactRecord) {
    let (level, dept) = classify_title(&contact.role);
    contact.seniority = level.as_str().to_string();
    contact.department = dept.as_str().to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_level_executive_detection() {
        let (level, dept) = classify_title("Генеральный директор и сооснователь компании");
        assert_eq!(level, SeniorityLevel::CLevel);
        assert_eq!(dept, Department::Executive);

        let (level2, dept2) = classify_title("Chief Technology Officer (CTO)");
        assert_eq!(level2, SeniorityLevel::CLevel);
        assert_eq!(dept2, Department::Engineering);
    }

    #[test]
    fn test_head_of_marketing_detection() {
        let (level, dept) = classify_title("Head of Growth & Digital Marketing");
        assert_eq!(level, SeniorityLevel::VPHead);
        assert_eq!(dept, Department::SalesMarketing);
    }

    #[test]
    fn test_hr_recruiter_detection() {
        let (level, dept) = classify_title("Рекрутер / HR (Поиск IT талантов)");
        assert_eq!(level, SeniorityLevel::Specialist);
        assert_eq!(dept, Department::HumanResources);
    }

    #[test]
    fn test_finance_legal_detection() {
        let (level, dept) = classify_title("Финансовый директор (CFO)");
        assert_eq!(level, SeniorityLevel::CLevel);
        assert_eq!(dept, Department::FinanceLegal);
    }
}
