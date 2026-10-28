use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcrdnsResult {
    pub ip: String,
    pub arpa_domain: String,
    pub ptr_hostname: Option<String>,
    pub forward_ips: Vec<String>,
    pub is_valid_fcrdns: bool,
    pub risk_score: u32,
    pub verdict: String,
}

pub fn ip_to_arpa(ip_str: &str) -> Option<String> {
    if let Ok(ipv4) = Ipv4Addr::from_str(ip_str.trim()) {
        let octets = ipv4.octets();
        Some(format!(
            "{}.{}.{}.{}.in-addr.arpa",
            octets[3], octets[2], octets[1], octets[0]
        ))
    } else {
        None
    }
}

pub fn evaluate_fcrdns_offline(
    ip: &str,
    ptr_hostname: Option<&str>,
    forward_ips: &[String],
) -> FcrdnsResult {
    let clean_ip = ip.trim().to_string();
    let arpa = ip_to_arpa(&clean_ip).unwrap_or_default();

    match ptr_hostname {
        None => FcrdnsResult {
            ip: clean_ip,
            arpa_domain: arpa,
            ptr_hostname: None,
            forward_ips: Vec::new(),
            is_valid_fcrdns: false,
            risk_score: 90,
            verdict: "High Risk: Missing PTR reverse DNS record. Many mail servers will reject or greylist this relay.".to_string(),
        },
        Some(host) => {
            let host_clean = host.trim().trim_end_matches('.').to_string();
            let matches = forward_ips.iter().any(|fip| fip.trim() == clean_ip);

            if matches {
                FcrdnsResult {
                    ip: clean_ip,
                    arpa_domain: arpa,
                    ptr_hostname: Some(host_clean),
                    forward_ips: forward_ips.to_vec(),
                    is_valid_fcrdns: true,
                    risk_score: 0,
                    verdict: "Clean: Forward-Confirmed Reverse DNS (FCrDNS) valid. Reverse PTR matches forward A record.".to_string(),
                }
            } else {
                FcrdnsResult {
                    ip: clean_ip,
                    arpa_domain: arpa,
                    ptr_hostname: Some(host_clean),
                    forward_ips: forward_ips.to_vec(),
                    is_valid_fcrdns: false,
                    risk_score: 85,
                    verdict: "High Risk: FCrDNS validation failed. PTR hostname resolves to different forward IPs, indicating possible spoofing or misconfiguration.".to_string(),
                }
            }
        }
    }
}

pub async fn audit_fcrdns(client: &Client, ip_str: &str) -> FcrdnsResult {
    let clean_ip = ip_str.trim().to_string();
    let arpa = match ip_to_arpa(&clean_ip) {
        Some(a) => a,
        None => {
            return FcrdnsResult {
                ip: clean_ip,
                arpa_domain: String::new(),
                ptr_hostname: None,
                forward_ips: Vec::new(),
                is_valid_fcrdns: false,
                risk_score: 99,
                verdict: "Invalid IPv4 address format provided.".to_string(),
            };
        }
    };

    let ptr_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=PTR",
        arpa
    );

    let ptr_resp = client
        .get(&ptr_url)
        .header("Accept", "application/dns-json")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    let ptr_hostname = match ptr_resp {
        Ok(resp) => {
            if let Ok(json) = resp.json::<Value>().await {
                json.get("Answer")
                    .and_then(|ans| ans.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            let rtype = item.get("type").and_then(|t| t.as_i64()).unwrap_or(0);
                            if rtype == 12 {
                                item.get("data")
                                    .and_then(|d| d.as_str())
                                    .map(|s| s.trim_end_matches('.').to_string())
                            } else {
                                None
                            }
                        })
                    })
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let mut forward_ips = Vec::new();
    if let Some(ref host) = ptr_hostname {
        let a_url = format!("https://cloudflare-dns.com/dns-query?name={}&type=A", host);
        if let Ok(a_resp) = client
            .get(&a_url)
            .header("Accept", "application/dns-json")
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            if let Ok(a_json) = a_resp.json::<Value>().await {
                if let Some(answers) = a_json.get("Answer").and_then(|a| a.as_array()) {
                    for ans in answers {
                        let rtype = ans.get("type").and_then(|t| t.as_i64()).unwrap_or(0);
                        if rtype == 1 {
                            if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                                forward_ips.push(data.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    evaluate_fcrdns_offline(&clean_ip, ptr_hostname.as_deref(), &forward_ips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_to_arpa() {
        assert_eq!(ip_to_arpa("192.0.2.1").unwrap(), "1.2.0.192.in-addr.arpa");
        assert_eq!(ip_to_arpa("8.8.8.8").unwrap(), "8.8.8.8.in-addr.arpa");
        assert!(ip_to_arpa("invalid-ip").is_none());
    }

    #[test]
    fn test_valid_fcrdns_offline() {
        let ip = "142.250.180.206";
        let ptr = "lhr48s27-in-f14.1e100.net";
        let fips = vec!["142.250.180.206".to_string(), "142.250.180.207".to_string()];

        let res = evaluate_fcrdns_offline(ip, Some(ptr), &fips);
        assert!(res.is_valid_fcrdns);
        assert_eq!(res.risk_score, 0);
        assert!(res.verdict.contains("Clean"));
    }

    #[test]
    fn test_mismatched_fcrdns_offline() {
        let ip = "198.51.100.42";
        let ptr = "mail.spoofed-server.com";
        let fips = vec!["203.0.113.99".to_string()];

        let res = evaluate_fcrdns_offline(ip, Some(ptr), &fips);
        assert!(!res.is_valid_fcrdns);
        assert_eq!(res.risk_score, 85);
        assert!(res.verdict.contains("failed"));
    }

    #[test]
    fn test_missing_ptr_fcrdns_offline() {
        let ip = "192.0.2.100";
        let res = evaluate_fcrdns_offline(ip, None, &[]);
        assert!(!res.is_valid_fcrdns);
        assert_eq!(res.risk_score, 90);
        assert!(res.verdict.contains("Missing PTR"));
    }
}
