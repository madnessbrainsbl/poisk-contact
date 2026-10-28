use crate::models::MxFingerprint;

pub fn fingerprint_mx(mx_host: &str) -> MxFingerprint {
    let host_lower = mx_host.trim().to_lowercase();

    if host_lower.contains("pphosted.com") || host_lower.contains("proofpoint") {
        MxFingerprint {
            host: host_lower,
            provider_name: "Proofpoint Enterprise Protection".to_string(),
            gateway_type: "Enterprise Security Gateway (SEG)".to_string(),
            strictness_level: "Maximum (Strict Sandboxing / URL Rewrite)".to_string(),
            deliverability_advice: "Используйте plain-text или чистый HTML. Proofpoint агрессивно сканирует ссылки и блокирует подозрительные редиректы.".to_string(),
        }
    } else if host_lower.contains("mimecast.com") {
        MxFingerprint {
            host: host_lower,
            provider_name: "Mimecast Secure Email Gateway".to_string(),
            gateway_type: "Enterprise Security Gateway (SEG)".to_string(),
            strictness_level: "Maximum (Targeted Threat Protection)".to_string(),
            deliverability_advice: "Требуется идеальная подпись DKIM и соответствие SPF. Избегайте агрессивных маркетинговых стоп-слов.".to_string(),
        }
    } else if host_lower.contains("barracudanetworks.com") || host_lower.contains("barracuda") {
        MxFingerprint {
            host: host_lower,
            provider_name: "Barracuda Email Security Gateway".to_string(),
            gateway_type: "Enterprise Security Gateway (SEG)".to_string(),
            strictness_level: "High (Barracuda Reputation Network)".to_string(),
            deliverability_advice: "Проверьте IP в BRBL (Barracuda Reputation). Избегайте шаблонов с высокой плотностью внешних ссылок.".to_string(),
        }
    } else if host_lower.contains("ironport")
        || host_lower.contains("iphmx.com")
        || host_lower.contains("cisco")
    {
        MxFingerprint {
            host: host_lower,
            provider_name: "Cisco Secure Email (IronPort)".to_string(),
            gateway_type: "Enterprise Security Gateway (SEG)".to_string(),
            strictness_level: "High (SenderBase Reputation)".to_string(),
            deliverability_advice: "Критично наличие валидного PTR (Reverse DNS) и нейтральная/высокая репутация в Cisco SenderBase.".to_string(),
        }
    } else if host_lower.contains("aspmx.l.google.com")
        || host_lower.contains("google.com")
        || host_lower.contains("googlemail.com")
    {
        MxFingerprint {
            host: host_lower,
            provider_name: "Google Workspace / Gmail".to_string(),
            gateway_type: "Cloud Productivity Suite".to_string(),
            strictness_level: "High (RFC 8058 & DMARC Enforcement)".to_string(),
            deliverability_advice: "Обязателен DMARC и List-Unsubscribe. Google блокирует отправителей при превышении 0.3% спам-жалоб.".to_string(),
        }
    } else if host_lower.contains("mail.protection.outlook.com")
        || host_lower.contains("outlook.com")
    {
        MxFingerprint {
            host: host_lower,
            provider_name: "Microsoft 365 / Exchange Online Protection (EOP)".to_string(),
            gateway_type: "Cloud Productivity Suite".to_string(),
            strictness_level: "High (EOP & Defender for Office 365)".to_string(),
            deliverability_advice: "Microsoft чувствителен к резким всплескам трафика. Прогревайте ящик с шагом не более 5-10 писем в день.".to_string(),
        }
    } else if host_lower.contains("mx.yandex.ru") || host_lower.contains("mx.yandex.net") {
        MxFingerprint {
            host: host_lower,
            provider_name: "Яндекс 360 для бизнеса".to_string(),
            gateway_type: "Cloud Productivity Suite".to_string(),
            strictness_level: "Medium-High (Спамооборона Яндекс)".to_string(),
            deliverability_advice: "Требуется корректная SPF и DKIM подпись. Поддерживайте чистый домен в Постмастере Яндекса.".to_string(),
        }
    } else if host_lower.contains("mxs.mail.ru") || host_lower.contains("biz.mail.ru") {
        MxFingerprint {
            host: host_lower,
            provider_name: "VK WorkMail (Mail.ru для бизнеса)".to_string(),
            gateway_type: "Cloud Productivity Suite".to_string(),
            strictness_level: "Medium-High (Mail.ru Postmaster FBL)".to_string(),
            deliverability_advice: "Зарегистрируйте домен в Postmaster Mail.ru для мониторинга репутации и FBL (Feedback Loop).".to_string(),
        }
    } else {
        MxFingerprint {
            host: host_lower,
            provider_name: "Self-Hosted / Generic MTA".to_string(),
            gateway_type: "Standard Mail Server (Postfix / Exim)".to_string(),
            strictness_level: "Standard (DNSBL & Greylisting)".to_string(),
            deliverability_advice: "Стандартные проверки DNSBL и Greylisting. Повторная отправка при задержке 451 обычно проходит успешно.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mx_fingerprinting() {
        let fp_google = fingerprint_mx("aspmx.l.google.com");
        assert_eq!(fp_google.provider_name, "Google Workspace / Gmail");
        assert!(fp_google.deliverability_advice.contains("List-Unsubscribe"));

        let fp_ms = fingerprint_mx("company-com.mail.protection.outlook.com");
        assert_eq!(
            fp_ms.provider_name,
            "Microsoft 365 / Exchange Online Protection (EOP)"
        );

        let fp_proofpoint = fingerprint_mx("mx1-us1.pphosted.com");
        assert_eq!(
            fp_proofpoint.provider_name,
            "Proofpoint Enterprise Protection"
        );
        assert!(fp_proofpoint.strictness_level.contains("Maximum"));

        let fp_yandex = fingerprint_mx("mx.yandex.net");
        assert_eq!(fp_yandex.provider_name, "Яндекс 360 для бизнеса");

        let fp_custom = fingerprint_mx("mail.custom-server.ru");
        assert_eq!(fp_custom.provider_name, "Self-Hosted / Generic MTA");
    }
}
