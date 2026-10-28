use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedSeniority {
    CLevel,
    VicePresident,
    DirectorOrHead,
    ManagerOrLead,
    IndividualContributor,
    UnknownSeniority,
}

impl std::fmt::Display for NormalizedSeniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizedSeniority::CLevel => write!(f, "C-Level (Chief / Executive / Founder)"),
            NormalizedSeniority::VicePresident => write!(f, "VP (Vice President / Executive Tier)"),
            NormalizedSeniority::DirectorOrHead => write!(f, "Director / Head of Department"),
            NormalizedSeniority::ManagerOrLead => write!(f, "Manager / Team Lead / Principal"),
            NormalizedSeniority::IndividualContributor => {
                write!(f, "Individual Contributor (Engineer / Specialist)")
            }
            NormalizedSeniority::UnknownSeniority => write!(f, "General / Unspecified Seniority"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizedDepartment {
    Security,
    Engineering,
    Product,
    SalesAndCommercial,
    Marketing,
    HumanResources,
    FinanceAndLegal,
    Operations,
    GeneralManagement,
}

impl std::fmt::Display for NormalizedDepartment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizedDepartment::Security => write!(f, "Security & Compliance"),
            NormalizedDepartment::Engineering => {
                write!(f, "Software Engineering & IT Infrastructure")
            }
            NormalizedDepartment::Product => write!(f, "Product Management"),
            NormalizedDepartment::SalesAndCommercial => write!(f, "Sales & Commercial Inbound"),
            NormalizedDepartment::Marketing => write!(f, "Marketing & Growth"),
            NormalizedDepartment::HumanResources => {
                write!(f, "Human Resources & Talent Acquisition")
            }
            NormalizedDepartment::FinanceAndLegal => write!(f, "Finance, Accounting & Legal"),
            NormalizedDepartment::Operations => write!(f, "Operations & Supply Chain"),
            NormalizedDepartment::GeneralManagement => write!(f, "Executive & General Management"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTitleReport {
    pub raw_title: String,
    pub clean_standard_title: String,
    pub seniority_level: NormalizedSeniority,
    pub department: NormalizedDepartment,
    pub influence_weight: u8,
    pub is_decision_maker: bool,
    pub recommended_approach: String,
}

pub fn normalize_job_title(raw: &str) -> NormalizedTitleReport {
    let clean = raw.trim();
    if clean.is_empty() {
        return NormalizedTitleReport {
            raw_title: "".to_string(),
            clean_standard_title: "Unspecified Title".to_string(),
            seniority_level: NormalizedSeniority::UnknownSeniority,
            department: NormalizedDepartment::GeneralManagement,
            influence_weight: 1,
            is_decision_maker: false,
            recommended_approach: "Title is missing. Qualify role during first outreach touch."
                .to_string(),
        };
    }

    let lower = clean.to_lowercase();

    // 1. Determine Department first (following Invariant 16: specialized before general)
    let department = if lower.contains("ciso")
        || lower.contains("security")
        || lower.contains("infosec")
        || lower.contains("appsec")
        || lower.contains("информационн")
        || lower.contains("безопасност")
        || lower.contains("иб")
    {
        NormalizedDepartment::Security
    } else if lower.contains("hr")
        || lower.contains("recruiter")
        || lower.contains("talent")
        || lower.contains("кадр")
        || lower.contains("найм")
        || lower.contains("персонал")
    {
        NormalizedDepartment::HumanResources
    } else if lower.contains("legal")
        || lower.contains("lawyer")
        || lower.contains("юрист")
        || lower.contains("cfo")
        || lower.contains("finance")
        || lower.contains("бухгалтер")
        || lower.contains("финанс")
    {
        NormalizedDepartment::FinanceAndLegal
    } else if lower.contains("product") || lower.contains("cpo") || lower.contains("продукт")
    {
        NormalizedDepartment::Product
    } else if lower.contains("sales")
        || lower.contains("account executive")
        || lower.contains("коммерческ")
        || lower.contains("продаж")
        || lower.contains("business development")
    {
        NormalizedDepartment::SalesAndCommercial
    } else if lower.contains("marketing")
        || lower.contains("cmo")
        || lower.contains("маркетинг")
        || lower.contains("growth")
        || lower.contains("pr")
    {
        NormalizedDepartment::Marketing
    } else if lower.contains("engineer")
        || lower.contains("developer")
        || lower.contains("devops")
        || lower.contains("cto")
        || lower.contains("backend")
        || lower.contains("frontend")
        || lower.contains("разработчик")
        || lower.contains("программист")
        || lower.contains("архитектор")
    {
        NormalizedDepartment::Engineering
    } else if lower.contains("coo") || lower.contains("operations") || lower.contains("операционн")
    {
        NormalizedDepartment::Operations
    } else {
        NormalizedDepartment::GeneralManagement
    };

    // 2. Determine Seniority Level
    let (seniority, influence, is_dm) = if lower.contains("ceo")
        || lower.contains("cto")
        || lower.contains("ciso")
        || lower.contains("cfo")
        || lower.contains("cmo")
        || lower.contains("coo")
        || lower.contains("cpo")
        || lower.contains("chief")
        || lower.contains("founder")
        || lower.contains("сооснователь")
        || lower.contains("основатель")
        || lower.contains("генеральный директор")
        || lower.contains("гендир")
        || lower.contains("президент")
    {
        (NormalizedSeniority::CLevel, 10u8, true)
    } else if lower.contains("vp")
        || lower.contains("vice president")
        || lower.contains("вице-президент")
        || lower.contains("evp")
        || lower.contains("svp")
    {
        (NormalizedSeniority::VicePresident, 9u8, true)
    } else if lower.contains("director")
        || lower.contains("директор")
        || lower.contains("head of")
        || lower.contains("руководитель")
        || lower.contains("начальник")
    {
        (NormalizedSeniority::DirectorOrHead, 8u8, true)
    } else if lower.contains("manager")
        || lower.contains("lead")
        || lower.contains("менеджер")
        || lower.contains("тимлид")
        || lower.contains("ведущий")
        || lower.contains("principal")
    {
        (NormalizedSeniority::ManagerOrLead, 6u8, false)
    } else {
        (NormalizedSeniority::IndividualContributor, 3u8, false)
    };

    // 3. Generate Clean Standardized Title
    let clean_standard_title = match (&seniority, &department) {
        (NormalizedSeniority::CLevel, NormalizedDepartment::Security) => {
            "Chief Information Security Officer (CISO)".to_string()
        }
        (NormalizedSeniority::CLevel, NormalizedDepartment::Engineering) => {
            "Chief Technology Officer (CTO)".to_string()
        }
        (NormalizedSeniority::CLevel, NormalizedDepartment::FinanceAndLegal) => {
            "Chief Financial Officer (CFO)".to_string()
        }
        (NormalizedSeniority::CLevel, NormalizedDepartment::Marketing) => {
            "Chief Marketing Officer (CMO)".to_string()
        }
        (NormalizedSeniority::CLevel, NormalizedDepartment::Operations) => {
            "Chief Operating Officer (COO)".to_string()
        }
        (NormalizedSeniority::CLevel, NormalizedDepartment::Product) => {
            "Chief Product Officer (CPO)".to_string()
        }
        (NormalizedSeniority::CLevel, _) => "Chief Executive Officer / Founder".to_string(),

        (NormalizedSeniority::VicePresident, dept) => format!("Vice President of {}", dept),
        (NormalizedSeniority::DirectorOrHead, dept) => format!("Director / Head of {}", dept),
        (NormalizedSeniority::ManagerOrLead, dept) => {
            format!("Engineering & Operations Lead ({})", dept)
        }
        (NormalizedSeniority::IndividualContributor, dept) => {
            format!("Senior Specialist / Contributor ({})", dept)
        }
        (NormalizedSeniority::UnknownSeniority, dept) => format!("Professional ({})", dept),
    };

    let recommended_approach = if is_dm {
        "High-priority decision-maker: Pitch business outcomes, ROI, and strategic risk reduction directly. Avoid low-level technical jargon.".to_string()
    } else if seniority == NormalizedSeniority::ManagerOrLead {
        "Influencer / Team Lead: Pitch operational efficiency, workflow time savings, and developer experience. Leverage as an internal champion.".to_string()
    } else {
        "Individual Contributor: Bottom-up adoption. Share open documentation, GitHub repos, and self-serve technical trials.".to_string()
    };

    NormalizedTitleReport {
        raw_title: clean.to_string(),
        clean_standard_title,
        seniority_level: seniority,
        department,
        influence_weight: influence,
        is_decision_maker: is_dm,
        recommended_approach,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_c_level_security() {
        let rep = normalize_job_title("Senior CISO & Head of Cybersecurity");
        assert_eq!(rep.seniority_level, NormalizedSeniority::CLevel);
        assert_eq!(rep.department, NormalizedDepartment::Security);
        assert!(rep.is_decision_maker);
        assert_eq!(rep.influence_weight, 10);
    }

    #[test]
    fn test_normalize_vp_sales() {
        let rep = normalize_job_title("VP of Global Sales and Business Development");
        assert_eq!(rep.seniority_level, NormalizedSeniority::VicePresident);
        assert_eq!(rep.department, NormalizedDepartment::SalesAndCommercial);
        assert!(rep.is_decision_maker);
        assert_eq!(rep.influence_weight, 9);
    }

    #[test]
    fn test_normalize_russian_director() {
        let rep = normalize_job_title("Генеральный директор и сооснователь");
        assert_eq!(rep.seniority_level, NormalizedSeniority::CLevel);
        assert!(rep.is_decision_maker);
        assert_eq!(rep.influence_weight, 10);
    }
}
