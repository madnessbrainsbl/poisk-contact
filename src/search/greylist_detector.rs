use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmtpTransientCategory {
    Greylisting,
    RateLimitThrottle,
    MailboxTemporarilyBusy,
    ConnectionTimeout,
    HardRejection,
    SuccessAcceptance,
    UnknownResponse,
}

impl std::fmt::Display for SmtpTransientCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmtpTransientCategory::Greylisting => write!(f, "Greylisting (Anti-Spam Verification)"),
            SmtpTransientCategory::RateLimitThrottle => write!(f, "Rate Limiting / IP Throttling"),
            SmtpTransientCategory::MailboxTemporarilyBusy => {
                write!(f, "Mailbox Busy / Temporary Storage")
            }
            SmtpTransientCategory::ConnectionTimeout => {
                write!(f, "Connection Timeout / Network Blip")
            }
            SmtpTransientCategory::HardRejection => write!(f, "Permanent Failure (Not Transient)"),
            SmtpTransientCategory::SuccessAcceptance => write!(f, "Success / 250 OK Accepted"),
            SmtpTransientCategory::UnknownResponse => write!(f, "Unknown SMTP Response"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreylistAnalysisReport {
    pub raw_response: String,
    pub is_transient_delay: bool,
    pub is_greylisting_detected: bool,
    pub category: SmtpTransientCategory,
    pub recommended_retry_minutes: u32,
    pub confidence_percent: u8,
    pub handling_recommendation: String,
}

pub fn audit_smtp_greylisting(response: &str) -> GreylistAnalysisReport {
    let clean = response.trim();
    if clean.is_empty() {
        return GreylistAnalysisReport {
            raw_response: "".to_string(),
            is_transient_delay: false,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::UnknownResponse,
            recommended_retry_minutes: 0,
            confidence_percent: 0,
            handling_recommendation: "Response is empty. Provide SMTP banner or bounce message."
                .to_string(),
        };
    }

    let lower = clean.to_lowercase();

    // Check for permanent 5xx failures first
    if lower.starts_with("5")
        || lower.contains(" 5.")
        || lower.contains("user unknown")
        || lower.contains("mailbox unavailable")
    {
        return GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: false,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::HardRejection,
            recommended_retry_minutes: 0,
            confidence_percent: 95,
            handling_recommendation:
                "Permanent failure (5xx). Do not retry. Suppress recipient address immediately."
                    .to_string(),
        };
    }

    // Check for success 250
    if lower.starts_with("250") || lower.contains("250 ok") || lower.contains("2.0.0 ok") {
        return GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: false,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::SuccessAcceptance,
            recommended_retry_minutes: 0,
            confidence_percent: 100,
            handling_recommendation: "Message accepted by recipient MTA. No retry required."
                .to_string(),
        };
    }

    // 1. Direct Greylisting cues
    let is_greylist_keyword = lower.contains("greylist")
        || lower.contains("graylist")
        || lower.contains("greylisting in action")
        || lower.contains("please come back later")
        || lower.contains("try again later")
        || lower.contains("service temporarily unavailable")
        || (lower.contains("451") && lower.contains("4.7.1"));

    if is_greylist_keyword {
        return GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: true,
            is_greylisting_detected: true,
            category: SmtpTransientCategory::Greylisting,
            recommended_retry_minutes: 10,
            confidence_percent: 95,
            handling_recommendation: "Standard greylisting defense detected (RFC 6647). Retrying after 10-15 minutes from the same sending IP will result in successful delivery and whitelist caching.".to_string(),
        };
    }

    // 2. Rate limiting and throttling cues
    let is_throttled = lower.contains("rate limit")
        || lower.contains("too many connections")
        || lower.contains("too many recipients")
        || lower.contains("try again in")
        || lower.contains("exceeded rate")
        || (lower.contains("421") && lower.contains("4.7.0"));

    if is_throttled {
        return GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: true,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::RateLimitThrottle,
            recommended_retry_minutes: 30,
            confidence_percent: 90,
            handling_recommendation: "Sending rate throttled by target mail gateway. Reduce dispatch frequency and wait at least 30 minutes before queuing retries.".to_string(),
        };
    }

    // 3. Mailbox temporarily busy or full
    let is_busy = lower.contains("mailbox busy")
        || lower.contains("storage full")
        || lower.contains("temporary failure")
        || lower.starts_with("450")
        || lower.starts_with("452");

    if is_busy {
        return GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: true,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::MailboxTemporarilyBusy,
            recommended_retry_minutes: 20,
            confidence_percent: 85,
            handling_recommendation: "Target mailbox is temporarily locked or exceeding quotas. Queue retry for 20-30 minutes.".to_string(),
        };
    }

    // 4. General 4xx transient code
    let is_general_4xx = lower.starts_with("4") || lower.contains(" 4.");
    if is_general_4xx {
        GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: true,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::ConnectionTimeout,
            recommended_retry_minutes: 15,
            confidence_percent: 75,
            handling_recommendation:
                "Generic 4xx transient delay. Attempt delivery retry with exponential backoff."
                    .to_string(),
        }
    } else {
        GreylistAnalysisReport {
            raw_response: clean.to_string(),
            is_transient_delay: false,
            is_greylisting_detected: false,
            category: SmtpTransientCategory::UnknownResponse,
            recommended_retry_minutes: 0,
            confidence_percent: 50,
            handling_recommendation: "Unrecognized response pattern. Manual review advised."
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_standard_postfix_greylisting() {
        let rep = audit_smtp_greylisting("451 4.7.1 Greylisting in action, please come back later");
        assert!(rep.is_transient_delay);
        assert!(rep.is_greylisting_detected);
        assert_eq!(rep.category, SmtpTransientCategory::Greylisting);
        assert_eq!(rep.recommended_retry_minutes, 10);
    }

    #[test]
    fn test_detect_rate_limiting_throttle() {
        let rep = audit_smtp_greylisting(
            "421 4.7.0 [TSS04] Message rejected due to exceeding rate limits",
        );
        assert!(rep.is_transient_delay);
        assert!(!rep.is_greylisting_detected);
        assert_eq!(rep.category, SmtpTransientCategory::RateLimitThrottle);
        assert_eq!(rep.recommended_retry_minutes, 30);
    }

    #[test]
    fn test_hard_bounce_not_greylisted() {
        let rep = audit_smtp_greylisting("550 5.1.1 User unknown");
        assert!(!rep.is_transient_delay);
        assert!(!rep.is_greylisting_detected);
        assert_eq!(rep.category, SmtpTransientCategory::HardRejection);
        assert_eq!(rep.recommended_retry_minutes, 0);
    }
}
