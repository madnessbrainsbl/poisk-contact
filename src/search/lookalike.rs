use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LookalikeDomainResult {
    pub candidate_domain: String,
    pub mutation_type: String,
    pub is_registered: bool,
    pub has_mx: bool,
    pub dns_record: String,
}

pub fn generate_lookalike_domains(domain: &str) -> Vec<(String, &'static str)> {
    let clean = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    let (name, tld) = match clean.split_once('.') {
        Some((n, t)) => (n, t),
        None => return Vec::new(),
    };

    let mut results: Vec<(String, &'static str)> = Vec::new();
    let mut seen = HashSet::new();
    seen.insert(clean.clone());

    let chars: Vec<char> = name.chars().collect();
    let len = chars.len();

    // 1. Omission (пропуск одной буквы)
    if len > 3 {
        for i in 0..len {
            let mut omitted = String::new();
            for (idx, &c) in chars.iter().enumerate() {
                if idx != i {
                    omitted.push(c);
                }
            }
            let cand = format!("{}.{}", omitted, tld);
            if seen.insert(cand.clone()) {
                results.push((cand, "Omission (Пропуск буквы)"));
            }
        }
    }

    // 2. Transposition (перестановка соседних букв)
    if len > 2 {
        for i in 0..len - 1 {
            let mut swapped = chars.clone();
            swapped.swap(i, i + 1);
            let s_name: String = swapped.into_iter().collect();
            let cand = format!("{}.{}", s_name, tld);
            if seen.insert(cand.clone()) {
                results.push((cand, "Transposition (Опечатка соседних букв)"));
            }
        }
    }

    // 3. Substitution & Homoglyphs (замена похожих символов)
    let substitutions: &[(char, &[char])] = &[
        ('o', &['0', 'a']),
        ('l', &['1', 'i']),
        ('i', &['1', 'l']),
        ('e', &['3']),
        ('a', &['4', 'o']),
        ('s', &['5']),
        ('c', &['k', 's']),
        ('v', &['w']),
        ('w', &['v']),
    ];

    for (from, to_list) in substitutions {
        if name.contains(*from) {
            for &replacement in *to_list {
                let replaced = name.replace(*from, &replacement.to_string());
                let cand = format!("{}.{}", replaced, tld);
                if seen.insert(cand.clone()) {
                    results.push((cand, "Homoglyph / Substitution (Визуальная подмена)"));
                }
            }
        }
    }

    // 4. Hyphenation (добавление дефиса)
    if len > 4 {
        for i in 1..len - 1 {
            let mut hyp = String::new();
            for (idx, &c) in chars.iter().enumerate() {
                if idx == i {
                    hyp.push('-');
                }
                hyp.push(c);
            }
            let cand = format!("{}.{}", hyp, tld);
            if seen.insert(cand.clone()) {
                results.push((cand, "Hyphenation (Вставка дефиса)"));
            }
        }
    }

    results
}

pub async fn audit_lookalikes(
    client: &reqwest::Client,
    verifier: &crate::search::doh::MxVerifier,
    domain: &str,
    max_check: usize,
) -> Vec<LookalikeDomainResult> {
    let candidates = generate_lookalike_domains(domain);
    let mut results = Vec::new();

    let check_slice = if candidates.len() > max_check {
        &candidates[..max_check]
    } else {
        &candidates[..]
    };

    for (cand, m_type) in check_slice {
        let (has_mx, mx_data) = verifier.check_domain(client, cand).await;
        let is_reg = has_mx || !mx_data.is_empty();

        results.push(LookalikeDomainResult {
            candidate_domain: cand.clone(),
            mutation_type: m_type.to_string(),
            is_registered: is_reg,
            has_mx,
            dns_record: if is_reg {
                mx_data
            } else {
                "Свободен (DNS NXDOMAIN)".to_string()
            },
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookalike_generation() {
        let list = generate_lookalike_domains("yandex.ru");
        assert!(!list.is_empty());

        let candidates: Vec<String> = list.iter().map(|(c, _)| c.clone()).collect();
        assert!(
            candidates
                .iter()
                .any(|c| c == "yadnex.ru" || c == "ynadex.ru")
        );
        assert!(
            candidates
                .iter()
                .any(|c| c == "yandx.ru" || c == "andex.ru")
        );
        assert!(!candidates.iter().any(|c| c == "yandex.ru"));
    }
}
