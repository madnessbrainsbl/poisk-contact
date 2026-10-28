use crate::models::SecurityPostureReport;
use reqwest::Client;
use reqwest::header::HeaderMap;
use std::time::Duration;

pub fn analyze_security_headers(headers: &HeaderMap, target_url: &str) -> SecurityPostureReport {
    let mut report = SecurityPostureReport {
        url: target_url.to_string(),
        ..Default::default()
    };

    let mut score: u8 = 0;

    // 1. Strict-Transport-Security (HSTS)
    if let Some(hsts) = headers.get("strict-transport-security") {
        if let Ok(val) = hsts.to_str() {
            let lower = val.to_lowercase();
            report.has_hsts = true;
            for part in lower.split(';') {
                let part = part.trim();
                if let Some((k, v)) = part.split_once('=') {
                    if k.trim() == "max-age" {
                        report.hsts_max_age = v.trim().parse().unwrap_or(0);
                    }
                }
            }
            if report.hsts_max_age >= 31536000 {
                score = score.saturating_add(30);
            } else if report.hsts_max_age > 0 {
                score = score.saturating_add(20);
                report.recommendations.push(
                    "HSTS max-age менее 1 года (рекомендуется max-age=31536000).".to_string(),
                );
            } else {
                score = score.saturating_add(15);
            }
        }
    } else {
        report.recommendations.push("Отсутствует заголовок Strict-Transport-Security (HSTS). Сайт уязвим к атакам SSL-Stripping.".to_string());
    }

    // 2. Content-Security-Policy (CSP)
    if let Some(csp) = headers.get("content-security-policy") {
        if let Ok(val) = csp.to_str() {
            if !val.trim().is_empty() {
                report.has_csp = true;
                score = score.saturating_add(25);
            }
        }
    } else {
        report.recommendations.push("Отсутствует Content-Security-Policy (CSP). Рекомендуется настроить политики загрузки скриптов и ресурсов.".to_string());
    }

    // 3. X-Frame-Options
    if let Some(xfo) = headers.get("x-frame-options") {
        if let Ok(val) = xfo.to_str() {
            let lower = val.to_lowercase();
            if lower.contains("deny") || lower.contains("sameorigin") {
                report.has_xfo = true;
                score = score.saturating_add(15);
            }
        }
    } else {
        report.recommendations.push(
            "Отсутствует X-Frame-Options (защита от Clickjacking атак во фреймах).".to_string(),
        );
    }

    // 4. X-Content-Type-Options
    if let Some(xcto) = headers.get("x-content-type-options") {
        if let Ok(val) = xcto.to_str() {
            if val.to_lowercase().contains("nosniff") {
                report.has_nosniff = true;
                score = score.saturating_add(15);
            }
        }
    } else {
        report.recommendations.push(
            "Отсутствует заголовок X-Content-Type-Options: nosniff (защита от подмены MIME-типов)."
                .to_string(),
        );
    }

    // 5. Referrer-Policy
    if let Some(rp) = headers.get("referrer-policy") {
        if let Ok(val) = rp.to_str() {
            if !val.trim().is_empty() {
                report.has_referrer_policy = true;
                score = score.saturating_add(15);
            }
        }
    }

    report.score = score.min(100);
    report.is_enterprise_ready = report.score >= 70;

    report.grade = if report.score >= 90 && report.has_hsts && report.has_csp {
        "A+".to_string()
    } else if report.score >= 80 {
        "A".to_string()
    } else if report.score >= 60 {
        "B".to_string()
    } else if report.score >= 40 {
        "C".to_string()
    } else {
        "F".to_string()
    };

    report
}

pub async fn audit_security_posture(client: &Client, raw_target: &str) -> SecurityPostureReport {
    let clean = raw_target.trim().trim_end_matches('/');
    let target_url = if clean.starts_with("http://") || clean.starts_with("https://") {
        clean.to_string()
    } else {
        format!("https://{}", clean)
    };

    let res = client
        .get(&target_url)
        .timeout(Duration::from_secs(5))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) OSINT Security Audit/1.0",
        )
        .send()
        .await;

    match res {
        Ok(resp) => analyze_security_headers(resp.headers(), &target_url),
        Err(e) => {
            let mut report = SecurityPostureReport {
                url: target_url,
                grade: "F".to_string(),
                ..Default::default()
            };
            report
                .recommendations
                .push(format!("Не удалось подключиться к целевому узлу: {}", e));
            report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_enterprise_security_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
        );
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static("default-src 'self'"),
        );
        headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
        headers.insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            "referrer-policy",
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );

        let report = analyze_security_headers(&headers, "https://secure-enterprise.com");
        assert_eq!(report.grade, "A+");
        assert!(report.score >= 90);
        assert!(report.is_enterprise_ready);
        assert!(report.has_hsts);
        assert!(report.has_csp);
        assert!(report.has_xfo);
        assert!(report.has_nosniff);
    }

    #[test]
    fn test_missing_headers() {
        let headers = HeaderMap::new();
        let report = analyze_security_headers(&headers, "https://insecure.com");
        assert_eq!(report.grade, "F");
        assert_eq!(report.score, 0);
        assert!(!report.is_enterprise_ready);
        assert!(!report.recommendations.is_empty());
    }
}
