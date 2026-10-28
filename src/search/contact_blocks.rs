use super::{extractor, location::Location};
use regex::Regex;
use std::sync::LazyLock;

static BLOCKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<(/?)(div|section|article|li|p|address|main|body)\b[^>]*>").unwrap()
});
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());
static HEADINGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<h[1-6]\b").unwrap());

// Select the smallest enclosing office cards, keeping their email and phone together.
pub(super) fn local_contacts(html: &str, location: &Location) -> (Vec<String>, Vec<String>) {
    let html = extractor::strip_html_boilerplate(html);
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut candidates = Vec::new();
    for cap in BLOCKS.captures_iter(&html) {
        let token = cap.get(0).unwrap();
        let tag = cap[2].to_ascii_lowercase();
        if &cap[1] != "/" {
            if !token.as_str().ends_with("/>") {
                stack.push((tag, token.start()));
            }
            continue;
        }
        let Some(index) = stack.iter().rposition(|(name, _)| *name == tag) else {
            continue;
        };
        let start = stack[index].1;
        stack.truncate(index);
        let block = &html[start..token.end()];
        if block.len() > 50_000 || HEADINGS.find_iter(block).count() > 1 {
            continue;
        }
        let text = extractor::clean_html_and_entities(&TAGS.replace_all(block, " "));
        if text.chars().count() > 1200 || !location.matches(&text, "") {
            continue;
        }
        let emails = extractor::extract_emails(block);
        let phones = extractor::phones_for_country(block, &location.country_code);
        if !emails.is_empty() || !phones.is_empty() {
            candidates.push((start, token.end(), emails, phones));
        }
    }
    let have_email = candidates
        .iter()
        .any(|(_, _, emails, _)| !emails.is_empty());
    let eligible = |candidate: &(usize, usize, Vec<String>, Vec<String>)| {
        !have_email || !candidate.2.is_empty()
    };
    let mut emails = Vec::new();
    let mut phones = Vec::new();
    for candidate in &candidates {
        if !eligible(candidate)
            || candidates
                .iter()
                .any(|child| eligible(child) && child.0 > candidate.0 && child.1 < candidate.1)
        {
            continue;
        }
        for email in &candidate.2 {
            if !emails.contains(email) {
                emails.push(email.clone());
            }
        }
        for phone in &candidate.3 {
            if !phones.contains(phone) {
                phones.push(phone.clone());
            }
        }
    }
    (emails, phones)
}
