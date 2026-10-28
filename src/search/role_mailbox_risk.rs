use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleRiskCategory {
    IndividualDecisionMaker,
    FunctionalInbound,
    HighRiskGeneric,
    TechnicalSupportTrap,
}

impl std::fmt::Display for RoleRiskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleRiskCategory::IndividualDecisionMaker => {
                write!(f, "Individual Decision-Maker (Tier 1 Priority)")
            }
            RoleRiskCategory::FunctionalInbound => {
                write!(f, "Functional Inbound (Commercial / Inbound Channel)")
            }
            RoleRiskCategory::HighRiskGeneric => {
                write!(f, "High-Risk Generic Catch-All (Low Engagement Risk)")
            }
            RoleRiskCategory::TechnicalSupportTrap => {
                write!(f, "Technical Support Trap (High Spam Penalty Risk)")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRiskReport {
    pub email: String,
    pub local_part: String,
    pub category: RoleRiskCategory,
    pub risk_penalty: u8,
    pub outreach_recommendation: String,
    pub should_deprioritize: bool,
}

const SUPPORT_TRAP_PREFIXES: &[&str] = &[
    "support",
    "helpdesk",
    "help",
    "billing",
    "invoices",
    "invoice",
    "accounting",
    "buh",
    "buhgalter",
    "tickets",
    "ticket",
    "service",
    "security",
    "privacy",
    "legal",
    "compliance",
    "dpo",
    "abuse",
    "postmaster",
    "hostmaster",
    "noc",
];

const GENERIC_HIGH_RISK_PREFIXES: &[&str] = &[
    "info",
    "contact",
    "contacts",
    "hello",
    "hi",
    "admin",
    "administrator",
    "office",
    "mail",
    "general",
    "inbox",
    "reception",
    "team",
    "corp",
    "corporate",
    "client",
    "clients",
];

const FUNCTIONAL_INBOUND_PREFIXES: &[&str] = &[
    "sales",
    "marketing",
    "partners",
    "partner",
    "partnership",
    "pr",
    "press",
    "media",
    "tender",
    "zakupki",
    "commerce",
    "commercial",
    "order",
    "orders",
    "zakaz",
    "lead",
    "leads",
    "hr",
    "jobs",
    "career",
    "careers",
    "talent",
    "recruiting",
    "recruitment",
    "dev",
    "tech",
];

pub fn audit_role_mailbox_risk(email: &str) -> RoleRiskReport {
    let clean_email = email.trim().to_lowercase();
    let local_part = clean_email.split('@').next().unwrap_or("").to_string();

    if SUPPORT_TRAP_PREFIXES.iter().any(|&p| {
        local_part == p
            || local_part.starts_with(&format!("{}.", p))
            || local_part.starts_with(&format!("{}_", p))
    }) {
        return RoleRiskReport {
            email: clean_email,
            local_part,
            category: RoleRiskCategory::TechnicalSupportTrap,
            risk_penalty: 70,
            outreach_recommendation: "Critical spam risk: Address routes into ticketing system or compliance queue. Avoid outbound cold pitches to prevent spam-flagging.".to_string(),
            should_deprioritize: true,
        };
    }

    if GENERIC_HIGH_RISK_PREFIXES.iter().any(|&p| {
        local_part == p
            || local_part.starts_with(&format!("{}.", p))
            || local_part.starts_with(&format!("{}_", p))
    }) {
        return RoleRiskReport {
            email: clean_email,
            local_part,
            category: RoleRiskCategory::HighRiskGeneric,
            risk_penalty: 40,
            outreach_recommendation: "High generic risk: Address is an unmonitored catch-all or reception desk with low reply rates. Recommend discovering specific personal decision-maker email.".to_string(),
            should_deprioritize: true,
        };
    }

    if FUNCTIONAL_INBOUND_PREFIXES.iter().any(|&p| {
        local_part == p
            || local_part.starts_with(&format!("{}.", p))
            || local_part.starts_with(&format!("{}_", p))
    }) {
        return RoleRiskReport {
            email: clean_email,
            local_part,
            category: RoleRiskCategory::FunctionalInbound,
            risk_penalty: 15,
            outreach_recommendation: "Commercial inbound mailbox: Monitored by commercial or procurement staff. Acceptable for B2B commercial proposals and vendor inquiries.".to_string(),
            should_deprioritize: false,
        };
    }

    RoleRiskReport {
        email: clean_email,
        local_part,
        category: RoleRiskCategory::IndividualDecisionMaker,
        risk_penalty: 0,
        outreach_recommendation: "Direct personal mailbox: Highest priority target for personalized 1-on-1 cold outreach with optimal inbox deliverability.".to_string(),
        should_deprioritize: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_individual_decision_maker() {
        let rep = audit_role_mailbox_risk("alexey.smirnov@company.ru");
        assert_eq!(rep.category, RoleRiskCategory::IndividualDecisionMaker);
        assert_eq!(rep.risk_penalty, 0);
        assert!(!rep.should_deprioritize);
    }

    #[test]
    fn test_functional_sales_inbound() {
        let rep = audit_role_mailbox_risk("sales@enterprise.com");
        assert_eq!(rep.category, RoleRiskCategory::FunctionalInbound);
        assert_eq!(rep.risk_penalty, 15);
        assert!(!rep.should_deprioritize);
    }

    #[test]
    fn test_high_risk_generic_info() {
        let rep = audit_role_mailbox_risk("info@agency.org");
        assert_eq!(rep.category, RoleRiskCategory::HighRiskGeneric);
        assert_eq!(rep.risk_penalty, 40);
        assert!(rep.should_deprioritize);
    }

    #[test]
    fn test_support_ticket_trap() {
        let rep = audit_role_mailbox_risk("support@saas.io");
        assert_eq!(rep.category, RoleRiskCategory::TechnicalSupportTrap);
        assert_eq!(rep.risk_penalty, 70);
        assert!(rep.should_deprioritize);
    }
}
