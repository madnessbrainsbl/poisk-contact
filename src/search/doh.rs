use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct MxVerifier {
    cache: Arc<Mutex<HashMap<String, (bool, String)>>>,
}

impl MxVerifier {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check_domain(&self, client: &Client, domain: &str) -> (bool, String) {
        let clean_domain = domain.trim().to_lowercase();
        if clean_domain.is_empty() || !clean_domain.contains('.') {
            return (false, String::new());
        }

        // Check cache first
        if let Ok(guard) = self.cache.lock() {
            if let Some(res) = guard.get(&clean_domain) {
                return res.clone();
            }
        }

        let url = format!(
            "https://cloudflare-dns.com/dns-query?name={}&type=MX",
            clean_domain
        );

        let res = client
            .get(&url)
            .header("Accept", "application/dns-json")
            .timeout(Duration::from_secs(4))
            .send()
            .await;

        let (valid, mx_data) = match res {
            Ok(resp) => {
                if let Ok(json) = resp.json::<Value>().await {
                    let status = json.get("Status").and_then(|v| v.as_i64()).unwrap_or(-1);
                    if status == 0 {
                        if let Some(answers) = json.get("Answer").and_then(|v| v.as_array()) {
                            let mut first_mx = String::new();
                            for ans in answers {
                                let ans_type =
                                    ans.get("type").and_then(|v| v.as_i64()).unwrap_or(0);
                                if ans_type == 15 {
                                    let data =
                                        ans.get("data").and_then(|v| v.as_str()).unwrap_or("");
                                    let clean_data = data.trim().trim_end_matches('.');
                                    if !clean_data.is_empty() {
                                        first_mx = clean_data.to_string();
                                        break;
                                    }
                                }
                            }
                            if !first_mx.is_empty() {
                                (true, first_mx)
                            } else {
                                (false, String::new())
                            }
                        } else {
                            (false, String::new())
                        }
                    } else {
                        (false, String::new())
                    }
                } else {
                    (false, String::new())
                }
            }
            Err(_) => (false, String::new()),
        };

        if let Ok(mut guard) = self.cache.lock() {
            guard.insert(clean_domain, (valid, mx_data.clone()));
        }

        (valid, mx_data)
    }

    pub async fn check_email(&self, client: &Client, email: &str) -> (bool, String) {
        if let Some((_, domain)) = email.split_once('@') {
            self.check_domain(client, domain).await
        } else {
            (false, String::new())
        }
    }

    pub async fn check_txt_records(&self, client: &Client, name: &str) -> Vec<String> {
        let clean_name = name.trim().to_lowercase();
        if clean_name.is_empty() || !clean_name.contains('.') {
            return Vec::new();
        }

        let url = format!(
            "https://cloudflare-dns.com/dns-query?name={}&type=TXT",
            clean_name
        );

        let res = client
            .get(&url)
            .header("Accept", "application/dns-json")
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        let mut txt_records = Vec::new();
        if let Ok(resp) = res {
            if let Ok(json) = resp.json::<Value>().await {
                let status = json.get("Status").and_then(|v| v.as_i64()).unwrap_or(-1);
                if status == 0 {
                    if let Some(answers) = json.get("Answer").and_then(|v| v.as_array()) {
                        for ans in answers {
                            let ans_type = ans.get("type").and_then(|v| v.as_i64()).unwrap_or(0);
                            if ans_type == 16 {
                                if let Some(data) = ans.get("data").and_then(|v| v.as_str()) {
                                    let cleaned = data.trim_matches('"').to_string();
                                    if !cleaned.is_empty() {
                                        txt_records.push(cleaned);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        txt_records
    }

    pub async fn audit_mta_sts(&self, client: &Client, domain: &str) -> crate::models::MtaStsAudit {
        let clean = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_lowercase();
        let mut audit = crate::models::MtaStsAudit {
            domain: clean.clone(),
            ..Default::default()
        };
        if clean.is_empty() || !clean.contains('.') {
            audit
                .recommendations
                .push("Некорректный домен для проверки MTA-STS.".to_string());
            return audit;
        }

        // 1. DoH TXT for _mta-sts.{domain}
        let sts_host = format!("_mta-sts.{}", clean);
        let txts = self.check_txt_records(client, &sts_host).await;
        for txt in &txts {
            if txt.contains("v=STSv1") {
                audit.dns_txt_valid = true;
                audit.txt_record = txt.clone();
                break;
            }
        }

        // 2. HTTP probe for policy file: https://mta-sts.{domain}/.well-known/mta-sts.txt
        let policy_url = format!("https://mta-sts.{}/.well-known/mta-sts.txt", clean);
        if let Ok(resp) = client
            .get(&policy_url)
            .timeout(Duration::from_secs(4))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    audit.policy_fetched = true;
                    for line in text.lines() {
                        let line = line.trim();
                        if let Some((k, v)) = line.split_once(':') {
                            let key = k.trim().to_lowercase();
                            let val = v.trim();
                            match key.as_str() {
                                "mode" => audit.mode = val.to_lowercase(),
                                "max_age" => audit.max_age = val.parse().unwrap_or(0),
                                "mx" => audit.mx_patterns.push(val.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // 3. DoH TXT for TLS-RPT: _smtp._tls.{domain}
        let tls_rpt_host = format!("_smtp._tls.{}", clean);
        let rpt_txts = self.check_txt_records(client, &tls_rpt_host).await;
        for txt in &rpt_txts {
            if txt.contains("v=TLSRPTv1") {
                audit.has_tls_rpt = true;
                audit.tls_rpt_record = txt.clone();
                break;
            }
        }

        // Recommendations
        if !audit.dns_txt_valid {
            audit.recommendations.push(format!(
                "Добавьте DNS TXT запись '_mta-sts.{}' со значением 'v=STSv1; id=...' для включения MTA-STS.",
                clean
            ));
        } else if !audit.policy_fetched {
            audit.recommendations.push(format!(
                "DNS запись MTA-STS найдена, но файл политики недоступен по адресу: {}",
                policy_url
            ));
        } else if audit.mode == "testing" {
            audit.recommendations.push("MTA-STS настроен в режиме 'testing'. Переведите в 'mode: enforce' для защиты от атак понижения шифрования (STARTTLS stripping).".to_string());
        }

        if !audit.has_tls_rpt {
            audit.recommendations.push(format!(
                "Для получения отчетов об ошибках TLS-доставки добавьте DNS TXT '_smtp._tls.{}' (RFC 8460).",
                clean
            ));
        }

        audit
    }

    pub async fn audit_bimi(&self, client: &Client, domain: &str) -> crate::models::BimiAudit {
        let clean = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_lowercase();
        let mut audit = crate::models::BimiAudit {
            domain: clean.clone(),
            ..Default::default()
        };
        if clean.is_empty() || !clean.contains('.') {
            audit
                .recommendations
                .push("Некорректный домен для проверки BIMI.".to_string());
            return audit;
        }

        let bimi_host = format!("default._bimi.{}", clean);
        let txts = self.check_txt_records(client, &bimi_host).await;
        for txt in &txts {
            if txt.starts_with("v=BIMI1") {
                audit.valid_bimi = true;
                audit.raw_record = txt.clone();
                for part in txt.split(';') {
                    let part = part.trim();
                    if let Some((k, v)) = part.split_once('=') {
                        let k = k.trim().to_lowercase();
                        let v = v.trim().to_string();
                        if k == "l" {
                            audit.svg_url = v;
                        } else if k == "a" {
                            audit.vmc_cert_url = v;
                        }
                    }
                }
                break;
            }
        }

        audit.has_vmc = !audit.vmc_cert_url.is_empty();

        if !audit.valid_bimi {
            audit.recommendations.push(format!(
                "Добавьте DNS TXT запись 'default._bimi.{}' со ссылкой на SVG-логотип для отображения аватара бренда в почтовых клиентах (Gmail, Apple Mail, Yahoo).",
                clean
            ));
        } else {
            if audit.svg_url.is_empty() {
                audit.recommendations.push(
                    "Укажите тег 'l=https://.../logo.svg' со ссылкой на SVG логотип бренда."
                        .to_string(),
                );
            }
            if !audit.has_vmc {
                audit.recommendations.push("Для получения синей галочки верификации в Gmail требуется VMC/CMC сертификат (тег 'a=https://.../cert.pem').".to_string());
            }
        }

        audit
    }
}
