use crate::models::{ContactRecord, LeadScore, SendStatus};
use crate::search::phone_intel::parse_and_enrich_phone;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    pub initial_count: usize,
    pub final_count: usize,
    pub duplicates_merged: usize,
    pub fields_enriched: usize,
}

fn is_generic_domain(domain: &str) -> bool {
    let d = domain.trim().to_lowercase();
    let d = d.trim_start_matches("www.");
    d.is_empty()
        || d == "t.me"
        || d == "telegram.me"
        || d == "vk.com"
        || d == "2gis.ru"
        || d == "yandex.ru"
        || d == "google.com"
        || d == "hh.ru"
        || d == "github.com"
        || d == "habr.com"
        || d == "facebook.com"
        || d == "instagram.com"
        || d == "linkedin.com"
        || d == "twitter.com"
        || d == "x.com"
        || d == "youtube.com"
        || [
            "remotive.com",
            "arbeitnow.com",
            "remoteok.com",
            "remoteok.io",
            "jobicy.com",
            "news.ycombinator.com",
        ]
        .iter()
        .any(|host| d == *host || d.ends_with(&format!(".{host}")))
}

fn extract_clean_domain(url_or_domain: &str) -> String {
    let raw = url_or_domain.trim().to_lowercase();
    if raw.is_empty() {
        return String::new();
    }
    let without_proto = raw
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_proto.split('/').next().unwrap_or("").trim();
    let host = host.split(':').next().unwrap_or("").trim();
    let clean = host.trim_start_matches("www.").to_string();
    if is_generic_domain(&clean) {
        String::new()
    } else {
        clean
    }
}

fn extract_clean_telegram(tg: &str) -> String {
    let s = tg.trim().to_lowercase();
    let s = s
        .trim_start_matches("https://t.me/")
        .trim_start_matches("http://t.me/")
        .trim_start_matches("t.me/")
        .trim_start_matches('@');
    let s = s.split('/').next().unwrap_or("").trim();
    if s.len() >= 3 && !s.contains(' ') {
        s.to_string()
    } else {
        String::new()
    }
}

pub fn deduplicate_and_merge(contacts: Vec<ContactRecord>) -> (Vec<ContactRecord>, MergeStats) {
    let initial_count = contacts.len();
    if initial_count <= 1 {
        let final_count = contacts.len();
        return (
            contacts,
            MergeStats {
                initial_count,
                final_count,
                duplicates_merged: 0,
                fields_enriched: 0,
            },
        );
    }

    // Disjoint Set Union (DSU) to group connected contacts
    let mut parent: Vec<usize> = (0..initial_count).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] == i {
            i
        } else {
            let p = parent[i];
            parent[i] = find(parent, p);
            parent[i]
        }
    }

    fn union(parent: &mut [usize], i: usize, j: usize) {
        let root_i = find(parent, i);
        let root_j = find(parent, j);
        if root_i != root_j {
            parent[root_i] = root_j;
        }
    }

    // Map unique keys to contact indices
    let mut email_to_idx: HashMap<String, usize> = HashMap::new();
    let mut phone_to_idx: HashMap<String, usize> = HashMap::new();
    let mut domain_to_idx: HashMap<String, usize> = HashMap::new();
    let mut identity_to_idx: HashMap<String, usize> = HashMap::new();
    let mut tg_to_idx: HashMap<String, usize> = HashMap::new();

    for (i, c) in contacts.iter().enumerate() {
        let name = super::location::normalized(&c.company);
        let address = super::location::normalized(&c.address);
        let city = super::location::normalized(&c.city);
        let identity = if super::tax_id::validate_russian_inn(&c.inn) {
            format!("inn:{}:{city}", c.inn)
        } else if name.chars().count() >= 5
            && address.chars().count() >= 8
            && address.chars().any(|c| c.is_ascii_digit())
        {
            format!("address:{city}:{name}:{address}")
        } else {
            String::new()
        };
        if !identity.is_empty() {
            if let Some(&previous) = identity_to_idx.get(&identity) {
                union(&mut parent, i, previous);
            } else {
                identity_to_idx.insert(identity, i);
            }
        }
        // 1. Email key
        let em = c.email.trim().to_lowercase();
        if !em.is_empty() && em.contains('@') {
            if let Some(&prev_idx) = email_to_idx.get(&em) {
                union(&mut parent, i, prev_idx);
            } else {
                email_to_idx.insert(em.clone(), i);
            }
        }

        // 2. Phone key (E.164 normalization)
        if !c.phone.is_empty() {
            let p_info = parse_and_enrich_phone(&c.phone);
            if p_info.is_valid && !p_info.e164.is_empty() {
                if let Some(&prev_idx) = phone_to_idx.get(&p_info.e164) {
                    union(&mut parent, i, prev_idx);
                } else {
                    phone_to_idx.insert(p_info.e164, i);
                }
            }
        }

        // 3. Domain key (from URL or email domain if corporate)
        let domain_from_url = extract_clean_domain(&c.url);
        let domain_from_email = if let Some((_, d)) = em.split_once('@') {
            extract_clean_domain(d)
        } else {
            String::new()
        };
        let domain = if !domain_from_url.is_empty() {
            domain_from_url
        } else {
            domain_from_email
        };

        if !domain.is_empty() {
            if let Some(&prev_idx) = domain_to_idx.get(&domain) {
                union(&mut parent, i, prev_idx);
            } else {
                domain_to_idx.insert(domain, i);
            }
        }

        // 4. Telegram key
        let tg = extract_clean_telegram(&c.telegram);
        if !tg.is_empty() {
            if let Some(&prev_idx) = tg_to_idx.get(&tg) {
                union(&mut parent, i, prev_idx);
            } else {
                tg_to_idx.insert(tg, i);
            }
        }
    }

    // Group items by root
    let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..initial_count {
        let root = find(&mut parent, i);
        clusters.entry(root).or_default().push(i);
    }

    let mut merged_list: Vec<ContactRecord> = Vec::with_capacity(clusters.len());
    let mut total_fields_enriched = 0;

    for (_, indices) in clusters {
        if indices.len() == 1 {
            let mut single = contacts[indices[0]].clone();
            // Ensure phone fields are enriched
            if !single.phone.is_empty()
                && (single.phone_carrier.is_empty() || single.whatsapp_url.is_empty())
            {
                let p = parse_and_enrich_phone(&single.phone);
                if p.is_valid {
                    single.phone = p.formatted;
                    single.phone_carrier = p.carrier;
                    single.phone_region = p.region;
                    single.phone_type = p.line_type;
                    single.whatsapp_url = p.whatsapp_url.unwrap_or_default();
                    single.telegram_url = p.telegram_url.unwrap_or_default();
                }
            }
            merged_list.push(single);
        } else {
            // Merge cluster
            let cluster_records: Vec<&ContactRecord> =
                indices.iter().map(|&idx| &contacts[idx]).collect();
            let (merged, enriched) = merge_cluster(&cluster_records);
            total_fields_enriched += enriched;
            merged_list.push(merged);
        }
    }

    // Stable sort by lead_score desc, then company name
    merged_list.sort_by(|a, b| {
        b.lead_score
            .cmp(&a.lead_score)
            .then_with(|| a.company.cmp(&b.company))
    });

    let final_count = merged_list.len();
    let duplicates_merged = initial_count.saturating_sub(final_count);

    (
        merged_list,
        MergeStats {
            initial_count,
            final_count,
            duplicates_merged,
            fields_enriched: total_fields_enriched,
        },
    )
}

fn merge_cluster(records: &[&ContactRecord]) -> (ContactRecord, usize) {
    // Pick base record: highest lead_score or most filled name
    let mut base = records[0].clone();
    for &r in &records[1..] {
        if r.lead_score > base.lead_score
            || (r.lead_score == base.lead_score && r.name.len() > base.name.len())
        {
            base = r.clone();
        }
    }

    let mut enriched_count = 0;
    // The first record is the existing database entry during collection updates.
    // Discovery must not reset recipient selection, notes or correspondence history.
    base.id = records[0].id.clone();
    base.selected = records[0].selected;
    base.created_at = records[0].created_at.clone();
    let mut notes = Vec::new();
    let mut evidence = Vec::new();
    for record in records {
        if !record.email.is_empty() && record.email != base.email {
            notes.push(format!("Additional saved email: {}", record.email));
        }
        if !record.phone.is_empty() && record.phone != base.phone {
            notes.push(format!("Additional saved phone: {}", record.phone));
        }
        for line in record.notes.lines() {
            if !line.is_empty() && !notes.iter().any(|saved| saved == line) {
                notes.push(line.to_string());
            }
        }
        if record.url != base.url && !record.url.is_empty() {
            notes.push(format!("Additional contact source: {}", record.url));
        }
        for proof in &record.evidence {
            if !evidence.contains(proof) {
                evidence.push(proof.clone());
            }
        }
    }
    let mut unique_notes = HashSet::new();
    notes.retain(|note| unique_notes.insert(note.clone()));
    base.notes = notes.join("\n");
    base.evidence = evidence;
    base.follow_up_count = records.iter().map(|r| r.follow_up_count).max().unwrap_or(0);
    if let Some(record) = records
        .iter()
        .filter(|r| r.last_sent_at.is_some())
        .max_by_key(|r| &r.last_sent_at)
    {
        base.last_sent_at = record.last_sent_at.clone();
        base.last_message_id = record.last_message_id.clone();
    }
    base.status = records[0].status.clone();

    // Collect and merge values across all records in cluster
    let mut all_tags: HashSet<String> = base.tags.into_iter().collect();
    let mut all_tech: HashSet<String> = base.tech_stack.into_iter().collect();
    let mut all_analytics: HashSet<String> = base.analytics_ids.into_iter().collect();
    let mut all_hiring_roles: HashSet<String> = base.hiring_roles.into_iter().collect();

    for &r in records {
        // Name: pick longest non-empty
        if r.name.chars().count() > base.name.chars().count()
            && !r.name.to_lowercase().contains("не указано")
        {
            base.name = r.name.clone();
            enriched_count += 1;
        }

        // Email: if base is empty or generic and r is personal, upgrade
        if base.email.is_empty() && !r.email.is_empty() {
            base.email = r.email.clone();
            base.mx_valid = r.mx_valid;
            base.mx_record = r.mx_record.clone();
            enriched_count += 1;
        } else if !r.email.is_empty()
            && (base.email.starts_with("info@") || base.email.starts_with("contact@"))
            && !r.email.starts_with("info@")
            && !r.email.starts_with("contact@")
        {
            base.email = r.email.clone();
            base.mx_valid = r.mx_valid;
            base.mx_record = r.mx_record.clone();
            enriched_count += 1;
        }

        // Company
        if base.company.is_empty() && !r.company.is_empty() {
            base.company = r.company.clone();
            enriched_count += 1;
        } else if r.company.chars().count() > base.company.chars().count() && !r.company.is_empty()
        {
            base.company = r.company.clone();
        }

        // Role
        if base.role.is_empty() && !r.role.is_empty() {
            base.role = r.role.clone();
            enriched_count += 1;
        }

        // City
        if base.city.is_empty() && !r.city.is_empty() {
            base.city = r.city.clone();
            enriched_count += 1;
        }

        // Stack
        if base.stack.is_empty() && !r.stack.is_empty() {
            base.stack = r.stack.clone();
            enriched_count += 1;
        }

        // Phone
        if base.phone.is_empty() && !r.phone.is_empty() {
            base.phone = r.phone.clone();
            enriched_count += 1;
        }

        // Telegram
        if base.telegram.is_empty() && !r.telegram.is_empty() {
            base.telegram = r.telegram.clone();
            enriched_count += 1;
        }

        // VK
        if base.vk_url.is_empty() && !r.vk_url.is_empty() {
            base.vk_url = r.vk_url.clone();
            enriched_count += 1;
        }

        // WhatsApp
        if base.whatsapp.is_empty() && !r.whatsapp.is_empty() {
            base.whatsapp = r.whatsapp.clone();
            enriched_count += 1;
        }

        // Legal info
        if base.legal_name.is_empty() && !r.legal_name.is_empty() {
            base.legal_name = r.legal_name.clone();
            enriched_count += 1;
        }
        if base.company_description.is_empty() && !r.company_description.is_empty() {
            base.company_description = r.company_description.clone();
            enriched_count += 1;
        }
        if base.address.is_empty() && !r.address.is_empty() {
            base.address = r.address.clone();
            enriched_count += 1;
        }
        if base.inn.is_empty() && !r.inn.is_empty() {
            base.inn = r.inn.clone();
            enriched_count += 1;
        }
        if base.ogrn.is_empty() && !r.ogrn.is_empty() {
            base.ogrn = r.ogrn.clone();
            enriched_count += 1;
        }
        if base.vat_id.is_empty() && !r.vat_id.is_empty() {
            base.vat_id = r.vat_id.clone();
            enriched_count += 1;
        }

        // GitHub
        if base.github_url.is_empty() && !r.github_url.is_empty() {
            base.github_url = r.github_url.clone();
            base.github_bio = r.github_bio.clone();
            enriched_count += 1;
        }

        // Hiring
        if r.hiring_active {
            base.hiring_active = true;
        }
        for role in &r.hiring_roles {
            all_hiring_roles.insert(role.clone());
        }

        // Collections
        for t in &r.tags {
            all_tags.insert(t.clone());
        }
        for t in &r.tech_stack {
            all_tech.insert(t.clone());
        }
        for a in &r.analytics_ids {
            all_analytics.insert(a.clone());
        }

        // Maximize scores & status
        if r.lead_score > base.lead_score {
            base.lead_score = r.lead_score;
        }
        if r.score == LeadScore::Hot {
            base.score = LeadScore::Hot;
        } else if base.score == LeadScore::Cold && r.score == LeadScore::Warm {
            base.score = LeadScore::Warm;
        }

        if matches!(r.status, SendStatus::Replied) {
            base.status = SendStatus::Replied;
        } else if matches!(r.status, SendStatus::Sent) && matches!(base.status, SendStatus::Pending)
        {
            base.status = SendStatus::Sent;
        }
    }

    base.tags = all_tags.into_iter().collect();
    base.tags.sort();
    base.tech_stack = all_tech.into_iter().collect();
    base.tech_stack.sort();
    base.analytics_ids = all_analytics.into_iter().collect();
    base.analytics_ids.sort();
    base.hiring_roles = all_hiring_roles.into_iter().collect();
    base.hiring_roles.sort();

    // Re-enrich phone via phone_intel
    if !base.phone.is_empty() {
        let p = parse_and_enrich_phone(&base.phone);
        if p.is_valid {
            base.phone = p.formatted;
            base.phone_carrier = p.carrier;
            base.phone_region = p.region;
            base.phone_type = p.line_type;
            base.whatsapp_url = p.whatsapp_url.unwrap_or_default();
            base.telegram_url = p.telegram_url.unwrap_or_default();
        }
    }

    (base, enriched_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_board_urls_do_not_merge_unrelated_companies() {
        let contacts: Vec<_> = ["remotive.com", "www.arbeitnow.com", "jobs.remoteok.com"]
            .into_iter()
            .flat_map(|host| {
                (0..3).map(move |index| ContactRecord {
                    company: format!("Employer {host} {index}"),
                    email: format!("office@employer-{index}-{}.example", host.replace('.', "-")),
                    url: format!("https://{host}/jobs/{index}"),
                    ..Default::default()
                })
            })
            .collect();
        let (merged, stats) = deduplicate_and_merge(contacts);
        assert_eq!(merged.len(), 9);
        assert_eq!(stats.duplicates_merged, 0);
    }

    #[test]
    fn collection_merge_preserves_notes_selection_and_mail_history() {
        let mut saved = ContactRecord {
            id: "saved-id".into(),
            url: "https://studio.org/".into(),
            email: "office@studio.org".into(),
            notes: "Private customer note".into(),
            selected: false,
            status: SendStatus::Sent,
            follow_up_count: 2,
            last_sent_at: Some("2026-10-17 12:00:00".into()),
            last_message_id: Some("<sent@sender.org>".into()),
            lead_score: 20,
            ..ContactRecord::default()
        };
        saved.evidence.push(crate::models::ContactEvidence {
            url: saved.url.clone(),
            ..Default::default()
        });
        let incoming = ContactRecord {
            id: "new-id".into(),
            url: "https://studio.org/contacts".into(),
            email: "director@studio.org".into(),
            selected: true,
            lead_score: 95,
            notes: "New website proof".into(),
            ..ContactRecord::default()
        };
        let (records, _) = deduplicate_and_merge(vec![saved, incoming]);
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.id, "saved-id");
        assert!(!record.selected);
        assert_eq!(record.status, SendStatus::Sent);
        assert_eq!(record.follow_up_count, 2);
        assert_eq!(record.last_message_id.as_deref(), Some("<sent@sender.org>"));
        assert!(
            record.notes.contains("Private customer note")
                && record.notes.contains("New website proof")
                && record.notes.contains("office@studio.org")
        );
        assert!(!record.evidence.is_empty());
    }

    #[test]
    fn merges_separate_domains_only_when_brand_city_and_address_agree() {
        let first = ContactRecord {
            company: "KraiMotors".into(),
            city: "Красноярск".into(),
            address: "ул. Маерчака, д. 50, стр. 6".into(),
            url: "https://kraimotors.qizi.ru/contacts".into(),
            ..Default::default()
        };
        let second = ContactRecord {
            company: "Kraimotors".into(),
            url: "https://kraimotors.ru/".into(),
            ..first.clone()
        };
        assert_eq!(
            deduplicate_and_merge(vec![first.clone(), second]).0.len(),
            1
        );
        let unrelated = ContactRecord {
            url: "https://unrelated.org/".into(),
            address: "ул. Маерчака, д. 51".into(),
            ..first.clone()
        };
        assert_eq!(deduplicate_and_merge(vec![first, unrelated]).0.len(), 2);
    }

    #[test]
    fn test_merge_duplicates_by_phone_and_domain() {
        let mut r1 = ContactRecord::new(
            "Иван",
            "info@acme.ru",
            "Acme Corp",
            "Директор",
            "Красноярск",
            "Rust",
            "+7 (913) 555-01-02",
            "",
            "Site Crawler",
            "https://acme.ru",
            LeadScore::Warm,
            true,
            "mx.acme.ru",
        );
        r1.tech_stack.push("CMS: Bitrix".to_string());

        let mut r2 = ContactRecord::new(
            "Иван Иванов",
            "ivan@acme.ru",
            "ООО АКМЕ",
            "CEO",
            "Красноярск",
            "Rust, Go",
            "89135550102",
            "@ivan_acme",
            "2GIS",
            "https://acme.ru/contacts",
            LeadScore::Hot,
            true,
            "mx.acme.ru",
        );
        r2.inn = "2466000000".to_string();

        let list = vec![r1, r2];
        let (merged, stats) = deduplicate_and_merge(list);

        assert_eq!(stats.initial_count, 2);
        assert_eq!(stats.final_count, 1);
        assert_eq!(stats.duplicates_merged, 1);

        let final_rec = &merged[0];
        assert_eq!(final_rec.name, "Иван Иванов");
        assert_eq!(final_rec.telegram, "@ivan_acme");
        assert_eq!(final_rec.inn, "2466000000");
        assert_eq!(final_rec.score, LeadScore::Hot);
        assert_eq!(final_rec.tech_stack, vec!["CMS: Bitrix"]);
        assert!(final_rec.phone_carrier.contains("МТС"));
        assert!(final_rec.whatsapp_url.contains("79135550102"));
    }
}
