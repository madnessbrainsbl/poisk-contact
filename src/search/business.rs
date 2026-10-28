use super::{
    SearchProgress, crawler, doh::MxVerifier, location::Location,
    waterfall_merge::deduplicate_and_merge, web,
};
use crate::models::{ContactRecord, SearchFilter};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};

pub const RU_SECTORS: &[&str] = &[
    "стоматология",
    "типография",
    "автосервис",
    "ресторан",
    "фотостудия",
    "мебель на заказ",
    "бухгалтерские услуги",
    "ветеринарная клиника",
    "гостиница",
    "строительная компания",
    "юридическая компания",
    "салон красоты",
    "грузоперевозки",
    "рекламное агентство",
    "фитнес клуб",
    "кондитерская",
    "туристическое агентство",
    "окна",
    "двери",
    "натяжные потолки",
    "клининг",
    "школа иностранных языков",
    "компьютерный сервис",
    "медицинская клиника",
    "пекарня",
    "цветочный магазин",
    "металлоконструкции",
    "аренда спецтехники",
    "производство мебели",
    "веб студия",
    "детский центр",
    "швейное производство",
    "электромонтаж",
    "сантехнические услуги",
    "ремонт квартир",
    "охранное предприятие",
    "пожарная безопасность",
    "проектное бюро",
    "изготовление вывесок",
    "кофейня",
    "банкетный зал",
    "сауна",
    "шиномонтаж",
    "автозапчасти",
    "строительные материалы",
    "пиломатериалы",
    "спортивный клуб",
    "прокат инструмента",
    "сервис бытовой техники",
    "ландшафтный дизайн",
    "вентиляция",
    "автошкола",
    "оптика",
    "химчистка",
    "доставка воды",
    "упаковка",
    "сварочные работы",
    "производство продуктов питания",
    "агентство недвижимости",
    "магазин освещения",
];
const EN_SECTORS: &[&str] = &[
    "dentist",
    "printing services",
    "car repair",
    "restaurant",
    "photography studio",
    "furniture maker",
    "accounting firm",
    "veterinary clinic",
    "hotel",
    "construction company",
    "law firm",
    "beauty salon",
    "freight transport",
    "advertising agency",
    "fitness club",
    "bakery",
    "travel agency",
    "window installer",
    "door supplier",
    "cleaning company",
    "language school",
    "computer repair",
    "medical clinic",
    "florist",
    "metal fabrication",
    "equipment rental",
    "web design agency",
    "childcare",
    "electrical contractor",
    "plumber",
];

#[derive(Default, Serialize, Deserialize)]
pub struct BusinessCheckpoint {
    pub city: String,
    pub query: String,
    #[serde(default)]
    pub role_filter: String,
    pub all_industries: bool,
    pub target: usize,
    pub records: Vec<ContactRecord>,
    pub completed_queries: Vec<String>,
    pub checked_urls: HashSet<String>,
    pub complete: bool,
}

impl BusinessCheckpoint {
    fn save(&self, path: &Option<PathBuf>) -> Result<(), String> {
        if let Some(path) = path {
            // Same atomic replacement as the contact database, with a separate schema.
            let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
            let temp = path.with_extension("json.next");
            std::fs::write(&temp, data)
                .map_err(|e| format!("Could not save collection progress: {e}"))?;
            std::fs::rename(&temp, path)
                .map_err(|e| format!("Could not replace collection progress: {e}"))?;
        }
        Ok(())
    }
}

fn digits(text: &str) -> String {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 11 && digits.starts_with('8') {
        format!("7{}", &digits[1..])
    } else {
        digits
    }
}

pub(super) fn published_record(mut record: ContactRecord) -> Option<ContactRecord> {
    if !crawler::is_company_website(&record.url)
        || !record.evidence.iter().any(|e| e.location_matches)
    {
        return None;
    }
    if !record.evidence.iter().any(|e| {
        e.emails
            .iter()
            .any(|mail| mail.eq_ignore_ascii_case(&record.email))
    }) {
        record.email.clear();
    }
    if !record
        .evidence
        .iter()
        .any(|e| e.phones.iter().any(|p| digits(p) == digits(&record.phone)))
    {
        record.phone.clear();
    }
    let handle = record.telegram.trim_start_matches('@').to_lowercase();
    if !record.evidence.iter().any(|e| {
        e.telegrams
            .iter()
            .any(|tg| tg.trim_start_matches('@').to_lowercase() == handle)
    }) {
        record.telegram.clear();
    }
    if !record
        .evidence
        .iter()
        .any(|e| e.vk_urls.contains(&record.vk_url))
    {
        record.vk_url.clear();
    }
    if record.email.is_empty()
        && record.phone.is_empty()
        && record.telegram.is_empty()
        && record.vk_url.is_empty()
    {
        return None;
    }
    record.selected = false;
    // Phone-derived convenience URLs do not prove that a messaging account exists.
    record.telegram_url.clear();
    record.whatsapp_url.clear();
    Some(record)
}

pub async fn run_business_search(
    filter: SearchFilter,
    all_industries: bool,
    checkpoint_path: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    tx: Sender<SearchProgress>,
) {
    let result = collect(filter, all_industries, checkpoint_path, cancel, &tx).await;
    match result {
        Ok(records) => {
            let _ = tx.send(SearchProgress::Done(records));
        }
        Err(error) => {
            let _ = tx.send(SearchProgress::Error(error));
        }
    }
}

async fn collect(
    filter: SearchFilter,
    all_industries: bool,
    checkpoint_path: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    tx: &Sender<SearchProgress>,
) -> Result<Vec<ContactRecord>, String> {
    if filter.city.trim().is_empty() || filter.max_items == 0 {
        return Err(
            "Automatic business collection requires a location and a positive company target."
                .into(),
        );
    }
    if !all_industries && filter.stack.trim().is_empty() {
        return Err("Enter a business or service, or select Different industries.".into());
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let location = Location::resolve(&client, &filter.city)
        .await
        .unwrap_or_else(|_| Location::raw(&filter.city));
    let russian = location.country_code == "RU"
        || (location.country_code.is_empty()
            && filter
                .city
                .chars()
                .any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)));
    let city = if location.native.is_empty() {
        &filter.city
    } else {
        &location.native
    };
    let sectors: Vec<String> = if all_industries {
        if russian { RU_SECTORS } else { EN_SECTORS }
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![filter.stack.trim().to_string()]
    };
    let mut state = if let Some(path) = &checkpoint_path {
        if path.exists() {
            serde_json::from_slice::<BusinessCheckpoint>(&std::fs::read(path).map_err(|e| e.to_string())?).map_err(|e| format!("Collection progress is unreadable; preserve or rename {} before restarting: {e}", path.display()))?
        } else {
            BusinessCheckpoint::default()
        }
    } else {
        BusinessCheckpoint::default()
    };
    // A finished run restarts only at the same target; a larger target continues it,
    // skipping the queries and websites already checked so no company is fetched twice.
    if (state.complete && filter.max_items <= state.records.len())
        || state.city != filter.city
        || state.query != filter.stack
        || state.role_filter != filter.role_filter
        || state.all_industries != all_industries
    {
        state = BusinessCheckpoint {
            city: filter.city.clone(),
            query: filter.stack.clone(),
            role_filter: filter.role_filter.clone(),
            all_industries,
            ..BusinessCheckpoint::default()
        };
    }
    state.target = filter.max_items;
    state
        .records
        .retain(|record| published_record(record.clone()).is_some());
    if !state.records.is_empty() {
        let _ = tx.send(SearchProgress::Collected(state.records.clone()));
    }
    let verifier = MxVerifier::new();
    let mut failures = 0;
    let suffixes: &[&str] = if russian {
        &["контакты официальный сайт", "адрес телефон email", "сайт"]
    } else {
        &["contact official website", "address phone email", "website"]
    };
    // One category needs several phrasings to reach the target; industry mode already varies the sector.
    let suffixes = if all_industries {
        &suffixes[..1]
    } else {
        suffixes
    };
    let queries = sectors
        .iter()
        .flat_map(|sector| suffixes.iter().map(move |suffix| (sector.clone(), *suffix)))
        .collect::<Vec<_>>();
    for (sector, suffix) in queries {
        if state.records.len() >= state.target || cancel.load(Ordering::Relaxed) {
            break;
        }
        let query = format!("{sector} {city} {suffix}");
        if state.completed_queries.contains(&query) {
            continue;
        }
        let _ = tx.send(SearchProgress::Log(
            "INFO".into(),
            format!(
                "Business collection: {}/{} companies; {query}",
                state.records.len(),
                state.target
            ),
        ));
        // OpenStreetMap is queried once per category (first phrasing only).
        let mut hits = Vec::new();
        if let (Some(area), true) = (location.osm_area, suffix == suffixes[0]) {
            match super::osm::business_websites(&client, area, &sector).await {
                Ok(found) => {
                    let _ = tx.send(SearchProgress::Log(
                        "INFO".into(),
                        format!(
                            "OpenStreetMap: {} '{sector}' websites in {city}",
                            found.len()
                        ),
                    ));
                    hits = found;
                }
                Err(error) => {
                    let _ = tx.send(SearchProgress::Log("WARN".into(), error));
                }
            }
        }
        match web::search_web(&client, &query, 60).await {
            Ok(found) => {
                failures = 0;
                hits.extend(found);
            }
            Err(error) => {
                let _ = tx.send(SearchProgress::Log(
                    "WARN".into(),
                    format!("{query}: {error}"),
                ));
            }
        }
        if hits.is_empty() {
            failures += 1;
            state.save(&checkpoint_path)?;
            if failures >= 3 {
                return Err(format!(
                    "Search providers are unavailable. {} companies were collected and saved. Start again to resume the saved progress after the provider cooldown.",
                    state.records.len()
                ));
            }
            continue;
        }
        let mut pending = hits.into_iter();
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            if cancel.load(Ordering::Relaxed) {
                tasks.abort_all();
                break;
            }
            while tasks.len() < 12 && state.records.len() < state.target {
                let Some(hit) = pending.next() else {
                    break;
                };
                if state.checked_urls.contains(&hit.url) || !crawler::is_company_website(&hit.url) {
                    continue;
                }
                let client = client.clone();
                let location = location.clone();
                let city = filter.city.clone();
                let sector = sector.clone();
                tasks.spawn(async move {
                    let result = tokio::time::timeout(
                        Duration::from_secs(30),
                        crawler::crawl_with_location(&client, &hit.url, &city, &sector, &location),
                    )
                    .await
                    .unwrap_or_else(|_| Err("Website time budget exceeded (30 seconds)".into()));
                    (hit.url, result)
                });
            }
            let Some(task) = tasks.join_next().await else {
                break;
            };
            let (url, result) = task.map_err(|e| format!("Collection worker failed: {e}"))?;
            state.checked_urls.insert(url.clone());
            match result {
                Ok(found) => {
                    let accepted: Vec<_> = found
                        .into_iter()
                        .filter_map(published_record)
                        .filter(|r| super::matches_role(&r.role, &filter.role_filter))
                        .collect();
                    if !accepted.is_empty() {
                        state.records.extend(accepted);
                        state.records = deduplicate_and_merge(std::mem::take(&mut state.records)).0;
                        for record in &mut state.records {
                            record.selected = false;
                            record.telegram_url.clear();
                            record.whatsapp_url.clear();
                        }
                        state.records.truncate(state.target);
                        if filter.verify_mx {
                            for record in &mut state.records {
                                if !record.email.is_empty() && record.mx_record.is_empty() {
                                    let (valid, server) =
                                        verifier.check_email(&client, &record.email).await;
                                    record.mx_valid = valid;
                                    record.mx_record = server;
                                }
                            }
                        }
                        state.save(&checkpoint_path)?;
                        let _ = tx.send(SearchProgress::Collected(state.records.clone()));
                        let _ = tx.send(SearchProgress::Log(
                            "SUCCESS".into(),
                            format!(
                                "Collected {}/{} unique companies: {url}",
                                state.records.len(),
                                state.target
                            ),
                        ));
                    }
                }
                Err(error) => {
                    let _ = tx.send(SearchProgress::Log(
                        "WARN".into(),
                        format!("{url}: {error}"),
                    ));
                }
            }
            state.save(&checkpoint_path)?;
        }
        if !cancel.load(Ordering::Relaxed) {
            state.completed_queries.push(query);
        }
        state.save(&checkpoint_path)?;
    }
    state.complete = state.records.len() >= state.target;
    state.save(&checkpoint_path)?;
    let _ = tx.send(SearchProgress::Log(if state.complete { "SUCCESS" } else { "WARN" }.into(), format!("Business collection {}: {}/{} companies. Published contacts only; MX is not mailbox delivery confirmation.", if state.complete { "complete" } else { "paused or sources exhausted" }, state.records.len(), state.target)));
    Ok(state.records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ContactEvidence;

    #[test]
    fn rejects_unproven_contacts_and_directory_records() {
        let mut record = ContactRecord::default();
        record.url = "https://local-business.org/contacts".into();
        record.email = "invented@local-business.org".into();
        record.phone = "+73912222222".into();
        record.evidence.push(ContactEvidence {
            url: record.url.clone(),
            checked_at: "2026-10-18T00:00:00Z".into(),
            emails: vec!["office@local-business.org".into()],
            phones: vec!["8 (391) 222-22-22".into()],
            location_matches: true,
            ..ContactEvidence::default()
        });
        let valid = published_record(record.clone()).unwrap();
        assert!(valid.email.is_empty());
        assert_eq!(valid.phone, record.phone);
        assert!(!valid.selected);
        record.url = "https://krasnoyarsk.spravker.ru/".into();
        assert!(published_record(record).is_none());
    }
}
