use crate::models::MailboxTier;

const DISPOSABLE_DOMAINS: &[&str] = &[
    "0-mail.com",
    "0clickemail.com",
    "10mail.org",
    "10minutemail.com",
    "10minutemail.net",
    "20minutemail.com",
    "anonymouse.org",
    "armyspy.com",
    "binkmail.com",
    "bobmail.info",
    "burnermail.io",
    "chacuo.net",
    "cool.fr.nf",
    "crazymailing.com",
    "cuvox.de",
    "dayrep.com",
    "dispostable.com",
    "dropmail.me",
    "einrot.com",
    "emailondeck.com",
    "fakemailgenerator.com",
    "fleckens.hu",
    "generator.email",
    "generator-email.com",
    "getairmail.com",
    "getnada.com",
    "grr.la",
    "guerrillamail.biz",
    "guerrillamail.com",
    "guerrillamail.net",
    "guerrillamail.org",
    "guerrillamailblock.com",
    "gustr.com",
    "inboxkitten.com",
    "instant-email.org",
    "jetable.fr.nf",
    "jourrapide.com",
    "mailcatch.com",
    "maildrop.cc",
    "mailinator.com",
    "mailnesia.com",
    "mailtemp.net",
    "mohmal.com",
    "mytemp.email",
    "mytempmail.com",
    "nada.ltd",
    "rhyta.com",
    "sharklasers.com",
    "spambox.us",
    "superrito.com",
    "teleworm.us",
    "temp-mail.org",
    "tempail.com",
    "temporary-mail.net",
    "tempinbox.com",
    "tempmail.com",
    "tempmail.ninja",
    "throwawaymail.com",
    "trash-mail.at",
    "trashmail.com",
    "trashmail.net",
    "yopmail.com",
    "yopmail.fr",
    "yopmail.net",
];

const FREE_WEBMAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "yandex.ru",
    "ya.ru",
    "yandex.by",
    "yandex.kz",
    "yandex.com",
    "mail.ru",
    "bk.ru",
    "inbox.ru",
    "list.ru",
    "internet.ru",
    "rambler.ru",
    "lenta.ru",
    "ro.ru",
    "autorambler.ru",
    "myrambler.ru",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "msn.com",
    "yahoo.com",
    "yahoo.co.uk",
    "icloud.com",
    "me.com",
    "mac.com",
    "proton.me",
    "protonmail.com",
    "zoho.com",
    "aol.com",
    "gmx.com",
    "gmx.net",
    "web.de",
];

pub fn is_disposable_domain(domain: &str) -> bool {
    let clean = domain.trim().to_lowercase();
    DISPOSABLE_DOMAINS
        .iter()
        .any(|&d| clean == d || clean.ends_with(&format!(".{}", d)))
}

pub fn is_free_webmail_domain(domain: &str) -> bool {
    let clean = domain.trim().to_lowercase();
    FREE_WEBMAIL_DOMAINS.iter().any(|&d| clean == d)
}

pub fn classify_mailbox_tier(email: &str) -> (MailboxTier, &'static str) {
    let clean = email.trim().to_lowercase();
    let domain = match clean.split_once('@') {
        Some((_, d)) => d,
        None => return (MailboxTier::Disposable, "Некорректный синтаксис email"),
    };

    if is_disposable_domain(domain) {
        (
            MailboxTier::Disposable,
            "Одноразовый почтовый ящик (высокий риск баунса/спама)",
        )
    } else if is_free_webmail_domain(domain) {
        (
            MailboxTier::FreeWebmail,
            "Бесплатный потребительский почтовый сервис (B2C)",
        )
    } else {
        (
            MailboxTier::CorporateB2b,
            "Корпоративный B2B почтовый домен компании",
        )
    }
}

pub fn apply_mailbox_tier_to_contact(contact: &mut crate::models::ContactRecord) {
    let (tier, _) = classify_mailbox_tier(&contact.email);
    contact.mailbox_tier = tier.to_string();
    if tier == MailboxTier::Disposable {
        contact.is_disposable = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disposable_detection() {
        assert!(is_disposable_domain("mailinator.com"));
        assert!(is_disposable_domain("temp-mail.org"));
        assert!(is_disposable_domain("sub.sharklasers.com"));
        assert!(!is_disposable_domain("google.com"));
        assert!(!is_disposable_domain("yandex.ru"));

        let (tier, _) = classify_mailbox_tier("user@tempmail.com");
        assert_eq!(tier, MailboxTier::Disposable);
    }

    #[test]
    fn test_webmail_and_b2b_detection() {
        let (tier_gmail, _) = classify_mailbox_tier("ceo@gmail.com");
        assert_eq!(tier_gmail, MailboxTier::FreeWebmail);

        let (tier_yandex, _) = classify_mailbox_tier("director@yandex.ru");
        assert_eq!(tier_yandex, MailboxTier::FreeWebmail);

        let (tier_b2b, _) = classify_mailbox_tier("cto@cybersec-corp.com");
        assert_eq!(tier_b2b, MailboxTier::CorporateB2b);
    }
}
