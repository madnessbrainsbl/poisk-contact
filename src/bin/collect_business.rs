use poisk_contact::{
    models::ContactRecord,
    search::{
        crawler::crawl_website_for_contacts, waterfall_merge::deduplicate_and_merge,
        web::search_web,
    },
    storage::{export_contacts_to_csv, load_contacts_from_json, save_contacts_to_json},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::task::JoinSet;

use poisk_contact::search::business::RU_SECTORS as SECTORS;

#[derive(Default, Serialize, Deserialize)]
struct Progress {
    city: String,
    target: usize,
    completed_sectors: Vec<String>,
    checked_urls: Vec<String>,
    errors: Vec<String>,
    companies: usize,
    with_email: usize,
    with_phone: usize,
    updated_at: String,
    complete: bool,
}

fn host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_lowercase())
        })
        .unwrap_or_default()
}

fn checkpoint(
    dir: &Path,
    records: &[ContactRecord],
    progress: &mut Progress,
) -> Result<(), String> {
    progress.companies = records.len();
    progress.with_email = records.iter().filter(|r| !r.email.is_empty()).count();
    progress.with_phone = records.iter().filter(|r| !r.phone.is_empty()).count();
    progress.updated_at = chrono::Utc::now().to_rfc3339();
    let temporary = dir.join("contacts.next.json");
    save_contacts_to_json(records, &temporary)?;
    std::fs::rename(&temporary, dir.join("contacts.json")).map_err(|e| e.to_string())?;
    export_contacts_to_csv(records, dir.join("contacts.csv"))?;
    let temporary = dir.join("progress.next.json");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(progress).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, dir.join("progress.json")).map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let mut city = "Красноярск".to_string();
    let mut target = 100usize;
    let mut dir = PathBuf::from("output/krasnoyarsk-20261018");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--city" | "--max" | "--output-dir" if i + 1 < args.len() => {
                match args[i].as_str() {
                    "--city" => city = args[i + 1].clone(),
                    "--max" => target = args[i + 1].parse::<usize>().map_err(|e| e.to_string())?,
                    _ => dir = PathBuf::from(&args[i + 1]),
                }
                i += 2;
            }
            _ => return Err(format!("Unknown or incomplete argument: {}", args[i])),
        }
    }
    if city.trim().is_empty() || target == 0 {
        return Err("City and a positive target are required".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut progress: Progress = if dir.join("progress.json").exists() {
        serde_json::from_slice(
            &std::fs::read(dir.join("progress.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
    } else {
        Progress {
            city: city.clone(),
            target,
            ..Progress::default()
        }
    };
    if progress.city != city {
        return Err("The output directory belongs to another city".into());
    }
    progress.target = target;
    let mut records = if dir.join("contacts.json").exists() {
        load_contacts_from_json(dir.join("contacts.json"))?
    } else {
        Vec::new()
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let mut checked: HashSet<String> = progress.checked_urls.iter().cloned().collect();
    let mut companies: HashSet<String> = records.iter().map(|r| host(&r.url)).collect();
    let mut failed_searches = 0;
    for sector in SECTORS {
        if records.len() >= target {
            break;
        }
        if progress.completed_sectors.iter().any(|s| s == sector) {
            continue;
        }
        let query = format!("{sector} {city} контакты официальный сайт");
        println!("[QUERY] {query}; companies={}/{}", records.len(), target);
        let hits = match search_web(&client, &query, 30).await {
            Ok(hits) => {
                failed_searches = 0;
                hits
            }
            Err(error) => {
                failed_searches += 1;
                let error = format!("{query}: {error}");
                println!("[SEARCH ERROR] {error}");
                progress.errors.push(error);
                checkpoint(&dir, &records, &mut progress)?;
                if failed_searches >= 3 {
                    break;
                }
                continue;
            }
        };
        println!("[DISCOVERED] {} URLs", hits.len());
        let mut tasks = JoinSet::new();
        let mut pending = hits.into_iter();
        loop {
            while tasks.len() < 6 && records.len() < target {
                let Some(hit) = pending.next() else {
                    break;
                };
                if checked.contains(&hit.url) || companies.contains(&host(&hit.url)) {
                    continue;
                }
                // The shared crawler rejects listing/social sites before requesting them.
                let client = client.clone();
                let city = city.clone();
                let sector = sector.to_string();
                tasks.spawn(async move {
                    let result = tokio::time::timeout(
                        Duration::from_secs(45),
                        crawl_website_for_contacts(&client, &hit.url, &city, &sector),
                    )
                    .await;
                    let result = result.unwrap_or_else(|_| {
                        Err("Website time budget exceeded (45 seconds)".into())
                    });
                    (hit.url, result)
                });
            }
            let Some(task) = tasks.join_next().await else {
                break;
            };
            match task {
                Ok((url, result)) => {
                    checked.insert(url.clone());
                    progress.checked_urls.push(url.clone());
                    match result {
                        Ok(found) => {
                            let (found, _) = deduplicate_and_merge(found);
                            for mut record in found {
                                if records.len() >= target {
                                    break;
                                }
                                let domain = host(&record.url);
                                if domain.is_empty() || companies.contains(&domain) {
                                    continue;
                                }
                                if record.email.is_empty()
                                    && record.phone.is_empty()
                                    && record.telegram.is_empty()
                                    && record.vk_url.is_empty()
                                {
                                    continue;
                                }
                                record.selected = false;
                                record.notes = format!(
                                    "{}\nCollected at {}; discovery query: {}",
                                    record.notes,
                                    chrono::Utc::now().to_rfc3339(),
                                    query
                                )
                                .trim()
                                .to_string();
                                companies.insert(domain);
                                let (merged, _) = deduplicate_and_merge({
                                    let mut all = records.clone();
                                    all.push(record);
                                    all
                                });
                                records = merged;
                                println!("[COLLECTED] {}/{} {}", records.len(), target, url);
                            }
                        }
                        Err(error) => {
                            println!("[WEBSITE ERROR] {url}: {error}");
                            progress.errors.push(format!("{url}: {error}"));
                        }
                    }
                    checkpoint(&dir, &records, &mut progress)?;
                }
                Err(error) => return Err(format!("Crawler task failed: {error}")),
            }
        }
        progress.completed_sectors.push(sector.to_string());
        checkpoint(&dir, &records, &mut progress)?;
    }
    progress.complete = records.len() >= target;
    checkpoint(&dir, &records, &mut progress)?;
    println!(
        "[FINISHED] companies={}, email={}, phone={}, complete={}",
        progress.companies, progress.with_email, progress.with_phone, progress.complete
    );
    if !progress.complete {
        return Err(format!(
            "Collected {} of {} companies; progress saved for resumption",
            records.len(),
            target
        ));
    }
    Ok(())
}
