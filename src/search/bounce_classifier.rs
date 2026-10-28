use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BounceType {
    HardBounce,
    SoftBounce,
    SpamBlock,
    AuthFailure,
    Success,
    Unknown,
}

impl std::fmt::Display for BounceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BounceType::HardBounce => write!(f, "Hard Bounce (Permanent Failure)"),
            BounceType::SoftBounce => write!(f, "Soft Bounce (Transient Failure)"),
            BounceType::SpamBlock => write!(f, "Spam Block (Reputation Policy Block)"),
            BounceType::AuthFailure => write!(f, "Auth Failure (SPF / DMARC / Relay Denied)"),
            BounceType::Success => write!(f, "Success (Delivered / OK)"),
            BounceType::Unknown => write!(f, "Unknown SMTP Status"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BounceAnalysis {
    pub bounce_type: BounceType,
    pub smtp_code: u16,
    pub enhanced_code: String,
    pub diagnostic_message: String,
    pub recommended_action: String,
    pub should_suppress: bool,
}

pub fn classify_smtp_bounce(raw_response: &str) -> BounceAnalysis {
    let text = raw_response.trim();
    let lower = text.to_lowercase();

    // 1. Extract 3-digit SMTP code (e.g., 550, 451, 250)
    let re_code = Regex::new(r"\b([245]\d{2})\b").ok();
    let smtp_code = re_code
        .and_then(|r| r.captures(text))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u16>().ok())
        .unwrap_or(0);

    // 2. Extract RFC 3464 enhanced status code (e.g., 5.1.1, 4.2.2)
    let re_enh = Regex::new(r"\b([245]\.\d{1,3}\.\d{1,3})\b").ok();
    let enhanced_code = re_enh
        .and_then(|r| r.captures(text))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "N/A".to_string());

    // 3. Success status
    if smtp_code == 250
        || enhanced_code.starts_with("2.")
        || lower.contains("250 2.0.0 ok")
        || lower.contains("queued for delivery")
    {
        return BounceAnalysis {
            bounce_type: BounceType::Success,
            smtp_code: if smtp_code != 0 { smtp_code } else { 250 },
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: "No action needed: Message successfully accepted by recipient MTA."
                .to_string(),
            should_suppress: false,
        };
    }

    // 4. Spam Block detection (Reputation, DNSBL, Spamhaus, SORBS)
    let is_spam_block = lower.contains("spamhaus")
        || lower.contains("dnsbl")
        || lower.contains("sorbs")
        || lower.contains("barracuda")
        || lower.contains("spamcop")
        || lower.contains("blocked using")
        || lower.contains("reputation")
        || lower.contains("content rejected")
        || lower.contains("message looks like spam")
        || lower.contains("policy rejection")
        || enhanced_code == "5.7.0"
        || lower.contains("spam");

    if is_spam_block {
        return BounceAnalysis {
            bounce_type: BounceType::SpamBlock,
            smtp_code,
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: "Pause outreach campaign immediately: Sender IP or domain is flagged by recipient spam filter or DNSBL.".to_string(),
            should_suppress: false,
        };
    }

    // Recipient-gone wording wins over the code: Yandex answers "5.7.1 No such user!",
    // mail.ru errors surfaced by lettre carry no numeric code at all.
    let is_mailbox_gone = [
        "user unknown",
        "unknown user",
        "no such user",
        "user not found",
        "user is terminated",
        "invalid mailbox",
        "mailbox not found",
        "does not exist",
        "account disabled",
        "account is disabled",
        "mailbox disabled",
        "no mx records",
        "does not accept mail",
    ]
    .iter()
    .any(|p| lower.contains(p));

    if is_mailbox_gone {
        return BounceAnalysis {
            bounce_type: BounceType::HardBounce,
            smtp_code,
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: "Permanent suppression: Purge contact from database and add to global unsubscribe blacklist to protect sender reputation.".to_string(),
            should_suppress: true,
        };
    }

    // 5. Auth / Relay Failure (SPF, DKIM, DMARC, Relay Access Denied)
    let is_auth_fail = lower.contains("relay access denied")
        || lower.contains("relaying denied")
        || lower.contains("relay not permitted")
        || lower.contains("dmarc")
        || lower.contains("spf fail")
        || lower.contains("spf check failed")
        || lower.contains("authentication required")
        || enhanced_code == "5.7.1"
        || enhanced_code == "5.7.26";

    if is_auth_fail {
        return BounceAnalysis {
            bounce_type: BounceType::AuthFailure,
            smtp_code,
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: "Audit DNS records: Verify SPF TXT, DKIM public key and DMARC alignment for the sender domain.".to_string(),
            should_suppress: false,
        };
    }

    // 6. Hard Bounce (Permanent Failure: Mailbox non-existent, user unknown, host unknown)
    let is_hard_bounce = enhanced_code == "5.1.1"
        || enhanced_code == "5.1.2"
        || enhanced_code == "5.2.1"
        || lower.contains("user unknown")
        || lower.contains("mailbox not found")
        || lower.contains("no such user")
        || lower.contains("recipient address rejected")
        || lower.contains("address does not exist")
        || lower.contains("host or domain name not found")
        || lower.contains("account disabled")
        || lower.contains("mailbox disabled")
        || (smtp_code == 550 && !lower.contains("greylist"))
        || smtp_code == 551
        || smtp_code == 553;

    if is_hard_bounce {
        return BounceAnalysis {
            bounce_type: BounceType::HardBounce,
            smtp_code,
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: "Permanent suppression: Purge contact from database and add to global unsubscribe blacklist to protect sender reputation.".to_string(),
            should_suppress: true,
        };
    }

    // 7. Soft Bounce (Transient Failure: Mailbox full, rate limit, greylisting, temporary timeout)
    let is_soft_bounce = enhanced_code == "4.2.2"
        || enhanced_code == "4.4.1"
        || enhanced_code == "4.4.2"
        || enhanced_code == "4.7.1"
        || lower.contains("mailbox full")
        || lower.contains("over quota")
        || lower.contains("greylisted")
        || lower.contains("try again later")
        || lower.contains("rate limit exceeded")
        || lower.contains("connection timed out")
        || lower.contains("temporary failure")
        || smtp_code == 421
        || smtp_code == 450
        || smtp_code == 451
        || smtp_code == 452;

    if is_soft_bounce {
        let retry_hours = if lower.contains("greylisted") { 1 } else { 24 };
        return BounceAnalysis {
            bounce_type: BounceType::SoftBounce,
            smtp_code,
            enhanced_code,
            diagnostic_message: text.to_string(),
            recommended_action: format!(
                "Temporary delivery issue: Reschedule message dispatch in {} hours.",
                retry_hours
            ),
            should_suppress: false,
        };
    }

    // Fallback unknown
    BounceAnalysis {
        bounce_type: BounceType::Unknown,
        smtp_code,
        enhanced_code,
        diagnostic_message: text.to_string(),
        recommended_action: "Review raw SMTP logs manually for custom provider diagnostic codes."
            .to_string(),
        should_suppress: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hard_bounce_user_unknown() {
        let resp = "550 5.1.1 <recipient@example.invalid>: Recipient address rejected: User unknown in virtual mailbox table";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::HardBounce);
        assert_eq!(b.smtp_code, 550);
        assert_eq!(b.enhanced_code, "5.1.1");
        assert!(b.should_suppress);
        assert!(b.recommended_action.contains("Purge contact"));
    }

    #[test]
    fn test_soft_bounce_mailbox_full() {
        let resp = "452 4.2.2 Mailbox full, unable to accept messages at this time";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::SoftBounce);
        assert_eq!(b.smtp_code, 452);
        assert_eq!(b.enhanced_code, "4.2.2");
        assert!(!b.should_suppress);
        assert!(b.recommended_action.contains("Reschedule"));
    }

    #[test]
    fn test_soft_bounce_greylisted() {
        let resp = "451 4.7.1 Greylisted, please try again in 300 seconds";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::SoftBounce);
        assert_eq!(b.smtp_code, 451);
        assert_eq!(b.enhanced_code, "4.7.1");
        assert!(!b.should_suppress);
        assert!(b.recommended_action.contains("Reschedule"));
    }

    #[test]
    fn test_spamblock_dnsbl() {
        let resp =
            "554 5.7.0 Service unavailable; Client host [1.2.3.4] blocked using zen.spamhaus.org";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::SpamBlock);
        assert_eq!(b.smtp_code, 554);
        assert_eq!(b.enhanced_code, "5.7.0");
        assert!(!b.should_suppress);
        assert!(b.recommended_action.contains("Pause outreach"));
    }

    #[test]
    fn test_auth_failure_relay() {
        let resp = "554 5.7.1 <recipient@example.invalid>: Relay access denied";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::AuthFailure);
        assert_eq!(b.smtp_code, 554);
        assert_eq!(b.enhanced_code, "5.7.1");
        assert!(b.recommended_action.contains("Audit DNS"));
    }

    #[test]
    fn test_provider_response_formats() {
        let hard = [
            "Direct MX TLS/SMTP failed for mx.yandex.net: 5.7.1 No such user! 1700000000",
            "source: \"Message was not accepted -- invalid mailbox.  Local mailbox recipient@example.invalid is unavailable: user not found\"",
            "Local mailbox recipient@example.invalid is unavailable: user is terminated",
            "Local mailbox recipient@example.invalid is unavailable: account is disabled",
            "5.1.1 The email account that you tried to reach does not exist.",
            "No MX records found for domain",
        ];
        for resp in hard {
            let b = classify_smtp_bounce(resp);
            assert_eq!(b.bounce_type, BounceType::HardBounce, "{resp}");
            assert!(b.should_suppress, "{resp}");
        }
        let spam = [
            "5.7.1 Message rejected under suspicion of SPAM; https://example.invalid/support",
            "Message discarded as high-probability spam. Contact recipient@example.invalid",
            "spam message rejected. Please visit https://help.mail.ru/notspam-support",
        ];
        for resp in spam {
            let b = classify_smtp_bounce(resp);
            assert_eq!(b.bounce_type, BounceType::SpamBlock, "{resp}");
            assert!(!b.should_suppress, "{resp}");
        }
        assert_eq!(
            classify_smtp_bounce("relay not permitted").bounce_type,
            BounceType::AuthFailure
        );
    }

    #[test]
    fn test_success_250_ok() {
        let resp = "250 2.0.0 OK 1739281729 d92819283-postfix queued for delivery";
        let b = classify_smtp_bounce(resp);
        assert_eq!(b.bounce_type, BounceType::Success);
        assert_eq!(b.smtp_code, 250);
        assert!(!b.should_suppress);
    }
}
