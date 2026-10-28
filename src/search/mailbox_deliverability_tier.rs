use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailboxArchetype {
    ExecutivePrimary,
    FunctionalInbound,
    GroupDistributionList,
    SystemGateway,
    GenericInbound,
}

impl std::fmt::Display for MailboxArchetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailboxArchetype::ExecutivePrimary => {
                write!(f, "Executive / Personal Decision-Maker Inbox")
            }
            MailboxArchetype::FunctionalInbound => {
                write!(f, "Functional Commercial Channel (Sales/Tender)")
            }
            MailboxArchetype::GroupDistributionList => {
                write!(f, "Group Distribution List (Multi-Recipient Alias)")
            }
            MailboxArchetype::SystemGateway => {
                write!(f, "Automated System Gateway (Noreply / Daemon)")
            }
            MailboxArchetype::GenericInbound => {
                write!(f, "Generic Company Inbound (Info / Office)")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxArchetypeReport {
    pub email: String,
    pub local_part: String,
    pub domain: String,
    pub archetype: MailboxArchetype,
    pub group_complaint_risk: String,
    pub deliverability_index: u8,
    pub is_outreach_safe: bool,
    pub strategy_advice: String,
}

const SYSTEM_PREFIXES: &[&str] = &[
    "noreply",
    "no-reply",
    "notifications",
    "mailer-daemon",
    "postmaster",
    "alert",
    "alerts",
    "system",
    "donotreply",
    "auto",
    "bounce",
    "bounces",
];

const GROUP_LIST_PREFIXES: &[&str] = &[
    "team",
    "all",
    "dev",
    "developers",
    "engineering",
    "security",
    "everyone",
    "staff",
    "crew",
    "sre",
    "ops",
    "support",
    "helpdesk",
    "billing",
    "accounting",
    "jobs",
    "careers",
    "press",
];

const FUNCTIONAL_PREFIXES: &[&str] = &[
    "sales",
    "commercial",
    "tender",
    "partners",
    "b2b",
    "commerce",
    "lead",
    "leads",
];

const GENERIC_PREFIXES: &[&str] = &[
    "info", "contact", "office", "admin", "hello", "mail", "inbox",
];

pub use audit_mailbox_archetype as classify_mailbox_archetype;

pub fn audit_mailbox_archetype(email_str: &str) -> MailboxArchetypeReport {
    let clean = email_str.trim().to_lowercase();
    let (local, domain) = match clean.split_once('@') {
        Some((l, d)) => (l.to_string(), d.to_string()),
        None => (clean.clone(), String::new()),
    };

    if SYSTEM_PREFIXES.iter().any(|&p| {
        local == p || local.starts_with(&format!("{}+", p)) || local.starts_with(&format!("{}-", p))
    }) {
        return MailboxArchetypeReport {
            email: clean,
            local_part: local,
            domain,
            archetype: MailboxArchetype::SystemGateway,
            group_complaint_risk: "Fatal (Unmonitored Robot Gateway)".to_string(),
            deliverability_index: 0,
            is_outreach_safe: false,
            strategy_advice: "Automated outgoing address. Mail is dropped or bounces immediately. Do not dispatch.".to_string(),
        };
    }

    if GROUP_LIST_PREFIXES.iter().any(|&p| local == p) {
        return MailboxArchetypeReport {
            email: clean,
            local_part: local,
            domain,
            archetype: MailboxArchetype::GroupDistributionList,
            group_complaint_risk: "High (Shared Multi-Recipient List)".to_string(),
            deliverability_index: 35,
            is_outreach_safe: false,
            strategy_advice: "Broadcast list forwards to multiple inboxes. High probability of receiving spam complaints from group members. Find specific personal contact instead.".to_string(),
        };
    }

    if FUNCTIONAL_PREFIXES.iter().any(|&p| local == p) {
        return MailboxArchetypeReport {
            email: clean,
            local_part: local,
            domain,
            archetype: MailboxArchetype::FunctionalInbound,
            group_complaint_risk: "Low-Medium (Monitored Commercial Queue)".to_string(),
            deliverability_index: 70,
            is_outreach_safe: true,
            strategy_advice: "Monitored vendor inbox. Acceptable for RFP, partnerships, and commercial proposals.".to_string(),
        };
    }

    if GENERIC_PREFIXES.iter().any(|&p| local == p) {
        return MailboxArchetypeReport {
            email: clean,
            local_part: local,
            domain,
            archetype: MailboxArchetype::GenericInbound,
            group_complaint_risk: "Medium (Corporate General Front Desk)".to_string(),
            deliverability_index: 55,
            is_outreach_safe: true,
            strategy_advice: "General inbox often triaged by office assistants. Low conversion rate. Use only if personal decision-maker cannot be resolved.".to_string(),
        };
    }

    // Default: Personal / Executive Inbox
    MailboxArchetypeReport {
        email: clean,
        local_part: local,
        domain,
        archetype: MailboxArchetype::ExecutivePrimary,
        group_complaint_risk: "Minimal (1:1 Personal Direct Inbox)".to_string(),
        deliverability_index: 95,
        is_outreach_safe: true,
        strategy_advice: "Direct personal inbox of individual prospect. Maximum engagement, lowest spam complaint probability. Prime target for personalized cold outreach.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_executive_mailbox() {
        let rep = audit_mailbox_archetype("alexey.smirnov@kaspersky.ru");
        assert_eq!(rep.archetype, MailboxArchetype::ExecutivePrimary);
        assert!(rep.is_outreach_safe);
        assert_eq!(rep.deliverability_index, 95);
    }

    #[test]
    fn test_group_distribution_team() {
        let rep = audit_mailbox_archetype("dev@startup.io");
        assert_eq!(rep.archetype, MailboxArchetype::GroupDistributionList);
        assert!(!rep.is_outreach_safe);
        assert!(rep.group_complaint_risk.contains("High"));
    }

    #[test]
    fn test_noreply_system_gateway() {
        let rep = audit_mailbox_archetype("noreply@notification.service.com");
        assert_eq!(rep.archetype, MailboxArchetype::SystemGateway);
        assert!(!rep.is_outreach_safe);
        assert_eq!(rep.deliverability_index, 0);
    }
}
