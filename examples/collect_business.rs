use poisk_contact::{
    models::SearchFilter,
    search::{
        SearchProgress, business::run_business_search, waterfall_merge::deduplicate_and_merge,
    },
};
use std::sync::{Arc, atomic::AtomicBool, mpsc::channel};

// Headless run of the GUI "Automatic business collection".
// Usage: cargo run --release --example collect_business -- <city> <business|all> <count> <out.json>
// `all` is the GUI "Different industries" mode.
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [city, stack, count, out] = args.as_slice() else {
        eprintln!("usage: collect_business <city> <business> <count> <out.json>");
        std::process::exit(2);
    };
    let all_industries = stack == "all";
    let filter = SearchFilter {
        city: city.clone(),
        stack: if all_industries {
            String::new()
        } else {
            stack.clone()
        },
        max_items: count.parse().expect("count must be a number"),
        ..Default::default()
    };
    let (tx, rx) = channel();
    let run = tokio::spawn(run_business_search(
        filter,
        all_industries,
        // Saved next to the output; a rerun with the same arguments resumes from it.
        Some(std::path::Path::new(out).with_extension("progress.json")),
        Arc::new(AtomicBool::new(false)),
        tx,
    ));

    let mut records = Vec::new();
    let save = |records: &Vec<_>| {
        let (records, _) = deduplicate_and_merge(records.clone());
        std::fs::write(out, serde_json::to_string_pretty(&records).unwrap()).unwrap();
        records.len()
    };
    for msg in rx {
        match msg {
            SearchProgress::Log(level, text) => println!("[{level}] {text}"),
            SearchProgress::Collected(batch) | SearchProgress::Done(batch) => {
                records.extend(batch);
                save(&records);
            }
            SearchProgress::Error(e) => eprintln!("[ERROR] {e}"),
        }
    }
    run.await.unwrap();
    println!("saved {} records to {out}", save(&records));
}
