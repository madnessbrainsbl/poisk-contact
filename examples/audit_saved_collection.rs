use poisk_contact::{models::ContactRecord, search::waterfall_merge::deduplicate_and_merge};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("Pass contact JSON files to audit".into());
    }
    for path in paths {
        let records: Vec<ContactRecord> = serde_json::from_slice(&fs::read(&path)?)?;
        let (_, stats) = deduplicate_and_merge(records);
        println!(
            "{}",
            serde_json::json!({"path": path, "records": stats.initial_count,
            "unique": stats.final_count, "duplicates": stats.duplicates_merged})
        );
    }
    Ok(())
}
