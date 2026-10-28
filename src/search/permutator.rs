use crate::models::{ContactRecord, LeadScore};
use crate::search::doh::MxVerifier;
use reqwest::Client;
use url::Url;

const AGGREGATOR_HOSTS: &[&str] = &[
    "remotive.com",
    "remoteok.com",
    "remoteok.io",
    "jobicy.com",
    "linkedin.com",
    "arbeitnow.com",
    "news.ycombinator.com",
    "t.me",
    "telegram.org",
    "google.com",
    "yandex.ru",
    "greenhouse.io",
    "lever.co",
    "ashbyhq.com",
    "workable.com",
    "smartrecruiters.com",
];

fn clean_company_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let cleaned = lower
        .replace("inc.", "")
        .replace("inc", "")
        .replace("llc", "")
        .replace("ltd", "")
        .replace("corp", "")
        .replace("gmbh", "")
        .replace("technologies", "")
        .replace("solutions", "")
        .replace("&", "")
        .replace("/", " ")
        .replace(".", "");

    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    parts.join("")
}

fn generate_candidate_domains(company_name: &str, url: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // 1. Try URL domain if not an aggregator
    if let Ok(parsed) = Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let host_clean = host.trim_start_matches("www.").to_lowercase();
            let is_agg = AGGREGATOR_HOSTS
                .iter()
                .any(|&agg| host_clean == agg || host_clean.ends_with(&format!(".{}", agg)));
            if !is_agg && host_clean.contains('.') {
                candidates.push(host_clean);
            }
        }
    }

    // 2. Generate slug-based domain candidates from company name
    let clean = clean_company_name(company_name);
    if clean.len() >= 2 {
        let tlds = ["com", "io", "ai", "co", "org", "tech", "digital"];
        for tld in tlds {
            candidates.push(format!("{}.{}", clean, tld));
        }

        let full_slug: String = company_name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        if full_slug != clean && full_slug.len() >= 3 {
            candidates.push(format!("{}.com", full_slug));
            candidates.push(format!("{}.io", full_slug));
        }
    }

    // Deduplicate
    let mut unique = Vec::new();
    for c in candidates {
        if !unique.contains(&c) {
            unique.push(c);
        }
    }
    unique
}

fn transliterate_russian(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let mapped = match lower {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'е' | 'ё' => "e",
            'ж' => "zh",
            'з' => "z",
            'и' => "i",
            'й' => "y",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "shch",
            'ъ' | 'ь' => "",
            'ы' => "y",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            other => {
                if other.is_ascii_alphanumeric() {
                    out.push(other);
                    continue;
                } else if other == ' ' || other == '-' {
                    out.push(' ');
                    continue;
                } else {
                    continue;
                }
            }
        };
        out.push_str(mapped);
    }
    out
}

pub fn generate_email_permutations(domain: &str, person_name: &str) -> Vec<String> {
    let clean_domain = domain
        .trim()
        .trim_start_matches('@')
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    if clean_domain.is_empty() {
        return Vec::new();
    }

    let mut perms = Vec::new();

    let trans = transliterate_russian(person_name);
    let parts: Vec<&str> = trans.split_whitespace().collect();

    // 1. If a person name is provided, generate high-prevalence B2B personal permutations first
    if parts.len() >= 2 {
        let first = parts[0].to_lowercase();
        let last = parts[1].to_lowercase();
        if !first.is_empty() && !last.is_empty() {
            let f_char = first
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            let l_char = last
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();

            // {first}.{last} (42% prevalence)
            perms.push(format!("{}.{}@{}", first, last, clean_domain));
            // {first} (28% prevalence - startups / executive)
            perms.push(format!("{}@{}", first, clean_domain));
            // {f}.{last} (14% prevalence)
            perms.push(format!("{}.{}@{}", f_char, last, clean_domain));
            // {first}_{last} (9% prevalence)
            perms.push(format!("{}_{}@{}", first, last, clean_domain));
            // {first}{last} (4% prevalence)
            perms.push(format!("{}{}@{}", first, last, clean_domain));
            // {f}{last}
            perms.push(format!("{}{}@{}", f_char, last, clean_domain));
            // {last}
            perms.push(format!("{}@{}", last, clean_domain));
            // {last}.{first}
            perms.push(format!("{}.{}@{}", last, first, clean_domain));
            // {first}.{l}
            perms.push(format!("{}.{}@{}", first, l_char, clean_domain));
        }
    } else if parts.len() == 1 && !parts[0].is_empty() && parts[0].len() >= 2 {
        let single = parts[0].to_lowercase();
        let is_role = [
            "info", "admin", "contact", "hr", "sales", "support", "team", "hello", "mail",
        ]
        .contains(&single.as_str());
        if !is_role {
            perms.push(format!("{}@{}", single, clean_domain));
        }
    }

    // 2. High-intent functional and recruitment role inboxes
    let common_roles = [
        "careers", "hr", "jobs", "hiring", "contact", "hello", "team", "info", "sales", "office",
        "mail",
    ];
    for role in common_roles {
        let candidate = format!("{}@{}", role, clean_domain);
        if !perms.contains(&candidate) {
            perms.push(candidate);
        }
    }

    perms
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PermutationVerificationResult {
    pub domain: String,
    pub is_catch_all: bool,
    pub verified_email: Option<String>,
    pub tried_permutations: Vec<(String, bool)>,
    pub best_guess: String,
    pub status: String,
}

pub async fn verify_candidate_permutations(
    first_name: &str,
    last_name: &str,
    domain: &str,
    mx_hint: Option<&str>,
) -> PermutationVerificationResult {
    let clean_dom = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    let full_name = format!("{} {}", first_name.trim(), last_name.trim());
    let perms = generate_email_permutations(&clean_dom, &full_name);

    let mut result = PermutationVerificationResult {
        domain: clean_dom.clone(),
        ..Default::default()
    };

    if perms.is_empty() {
        result.status = "Не удалось сгенерировать варианты email".to_string();
        return result;
    }

    result.best_guess = perms.first().cloned().unwrap_or_default();

    // 1. Check if domain is Catch-All
    match crate::mailer::detect_catch_all_domain(&clean_dom, mx_hint).await {
        Ok(true) => {
            result.is_catch_all = true;
            result.status =
                "Catch-All домен: принимает любые адреса. Использован статистический шаблон."
                    .to_string();
            return result;
        }
        Ok(false) => {
            result.is_catch_all = false;
        }
        Err(_) => {
            result.is_catch_all = false;
        }
    }

    // 2. Pre-flight test top-5 permutations via SMTP RCPT TO
    for email in perms.iter().take(6) {
        match crate::mailer::verify_mailbox_rcpt(email, mx_hint).await {
            Ok(true) => {
                result.tried_permutations.push((email.clone(), true));
                result.verified_email = Some(email.clone());
                result.best_guess = email.clone();
                result.status = format!("Верифицирован: ящик {} подтвердил прием (250 OK)", email);
                break;
            }
            Ok(false) => {
                result.tried_permutations.push((email.clone(), false));
            }
            Err(_) => {
                result.tried_permutations.push((email.clone(), false));
            }
        }
    }

    if result.verified_email.is_none() {
        result.status =
            "Ящики отклонены сервером (550 User Unknown) или порт 25 недоступен".to_string();
    }

    result
}

pub async fn enrich_contacts_with_permutator(
    client: &Client,
    mx_verifier: &MxVerifier,
    contacts: &mut [ContactRecord],
) {
    for contact in contacts.iter_mut() {
        if !contact.email.trim().is_empty() {
            continue;
        }

        let candidate_domains = generate_candidate_domains(&contact.company, &contact.url);
        for domain in candidate_domains {
            let (mx_ok, mx_rec) = mx_verifier.check_domain(client, &domain).await;
            if mx_ok && !mx_rec.is_empty() {
                // Try testing candidate permutations with RCPT TO verification
                let perms = generate_email_permutations(&domain, &contact.name);
                let mut found_rcpt = None;
                let mut found_is_personal = false;

                for p in perms.iter().take(6) {
                    if let Ok(true) = crate::mailer::verify_mailbox_rcpt(p, Some(&mx_rec)).await {
                        let is_personal = !p.starts_with("info@")
                            && !p.starts_with("contact@")
                            && !p.starts_with("hello@")
                            && !p.starts_with("mail@")
                            && !p.starts_with("office@");
                        found_rcpt = Some(p.clone());
                        found_is_personal = is_personal;
                        break;
                    }
                }

                if let Some(verified_email) = found_rcpt {
                    contact.email = verified_email;
                    contact.rcpt_verified = true;
                    contact.mx_valid = true;
                    contact.mx_record = mx_rec;
                    contact.score = if found_is_personal {
                        LeadScore::Hot
                    } else {
                        LeadScore::Warm
                    };
                }
                // An unverified guess is not a contact: leave the email empty.
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_person_permutation_priority() {
        let perms = generate_email_permutations("example.com", "Pavel Durov");
        assert!(!perms.is_empty());
        assert_eq!(perms[0], "pavel.durov@example.com");
        assert_eq!(perms[1], "pavel@example.com");
        assert_eq!(perms[2], "p.durov@example.com");
        // Functional roles should be present at the end
        assert!(perms.contains(&"careers@example.com".to_string()));
        assert!(perms.contains(&"info@example.com".to_string()));
    }

    #[test]
    fn test_cyrillic_person_permutation_priority() {
        let perms = generate_email_permutations("yandex.ru", "Иван Петров");
        assert!(!perms.is_empty());
        assert_eq!(perms[0], "ivan.petrov@yandex.ru");
        assert_eq!(perms[1], "ivan@yandex.ru");
    }

    #[test]
    fn test_unnamed_company_permutation_priority() {
        let perms = generate_email_permutations("acme.io", "");
        assert!(!perms.is_empty());
        // For empty name, high-intent hiring roles come first
        assert_eq!(perms[0], "careers@acme.io");
        assert_eq!(perms[1], "hr@acme.io");
    }
}
