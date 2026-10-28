use poisk_contact::{
    models::ContactRecord,
    search::waterfall_merge::deduplicate_and_merge,
    storage::{export_contacts_to_csv, load_contacts_from_json, save_contacts_to_json},
};
use std::collections::HashSet;

// Adds newly collected companies to the main database without touching existing records:
// a collected company is skipped when its email, website host or phone is already known.
// (deduplicate_and_merge over the whole database would fold sibling records sharing a
// domain and drop their emails and send history.) New records stay unselected.
// Usage: cargo run --release --example merge_contacts -- <contacts.json> <collected.json>
fn host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', ':'])
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_lowercase()
}

fn phone(p: &str) -> String {
    let d: String = p.chars().filter(char::is_ascii_digit).collect();
    d[d.len().saturating_sub(10)..].to_string()
}

fn keys(c: &ContactRecord) -> Vec<String> {
    [
        format!("e:{}", c.email.trim().to_lowercase()),
        format!("h:{}", host(&c.url)),
        format!("p:{}", phone(&c.phone)),
    ]
    .into_iter()
    .filter(|k| k.len() > 2)
    .collect()
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [main_path, new_path] = args.as_slice() else {
        return Err("usage: merge_contacts <contacts.json> <collected.json>".into());
    };
    let mut existing = load_contacts_from_json(main_path)?;
    let (collected, _) = deduplicate_and_merge(load_contacts_from_json(new_path)?);

    let mut known: HashSet<String> = existing.iter().flat_map(keys).collect();
    let mut added = 0;
    for mut c in collected {
        let k = keys(&c);
        if k.iter().any(|k| known.contains(k)) {
            continue;
        }
        known.extend(k);
        c.selected = false;
        existing.push(c);
        added += 1;
    }
    if added == 0 {
        println!("nothing new to add");
        return Ok(());
    }
    let backup = format!(
        "backups/contacts-before-merge-{}.json",
        chrono::Local::now().format("%Y%m%dT%H%M%S")
    );
    std::fs::copy(main_path, &backup).map_err(|e| format!("backup failed: {e}"))?;
    save_contacts_to_json(&existing, main_path)?;
    export_contacts_to_csv(
        &existing,
        std::path::Path::new(main_path).with_extension("csv"),
    )?;
    println!(
        "added {added} new companies -> {} records. Backup: {backup}",
        existing.len()
    );
    Ok(())
}
