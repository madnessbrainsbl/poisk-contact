use crate::models::MailInfraSubdomain;
use reqwest::Client;

pub const COMMON_MAIL_SUBDOMAINS: &[(&str, &str)] = &[
    ("mail", "Основной почтовый сервер / Webmail"),
    ("smtp", "Выделенный SMTP шлюз / Релей"),
    ("webmail", "Почтовый веб-интерфейс (Roundcube / cPanel)"),
    ("autodiscover", "Microsoft Exchange Autodiscover / EWS"),
    ("imap", "Входящий IMAP сервер"),
    ("pop3", "Входящий POP3 сервер"),
];

pub async fn recon_mail_infrastructure(client: &Client, domain: &str) -> Vec<MailInfraSubdomain> {
    let clean_domain = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();

    if clean_domain.is_empty() {
        return Vec::new();
    }

    let mut discovered = Vec::new();

    for &(prefix, desc) in COMMON_MAIL_SUBDOMAINS {
        let candidate = format!("{}.{}", prefix, clean_domain);
        let doh_url = format!(
            "https://cloudflare-dns.com/dns-query?name={}&type=A",
            candidate
        );

        if let Ok(resp) = client
            .get(&doh_url)
            .header("Accept", "application/dns-json")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                    if let Some(answers) = json_val.get("Answer").and_then(|a| a.as_array()) {
                        for ans in answers {
                            let data = ans.get("data").and_then(|d| d.as_str()).unwrap_or("");
                            if !data.is_empty() {
                                discovered.push(MailInfraSubdomain {
                                    subdomain: candidate.clone(),
                                    resolved_record: data.to_string(),
                                    service_type: desc.to_string(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    discovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_subdomain_prefixes_count() {
        assert!(COMMON_MAIL_SUBDOMAINS.len() >= 5);
        assert!(
            COMMON_MAIL_SUBDOMAINS
                .iter()
                .any(|(p, _)| *p == "autodiscover")
        );
        assert!(COMMON_MAIL_SUBDOMAINS.iter().any(|(p, _)| *p == "mail"));
    }
}
