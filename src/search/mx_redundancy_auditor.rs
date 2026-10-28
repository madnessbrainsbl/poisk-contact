use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxHostInfo {
    pub preference: u16,
    pub hostname: String,
    pub resolved_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxRedundancyReport {
    pub domain: String,
    pub total_mx_records: usize,
    pub distinct_priority_tiers: usize,
    pub distinct_c_class_subnets: usize,
    pub redundancy_grade: String,
    pub has_failover_host: bool,
    pub has_subnet_spof: bool,
    pub deliverability_verdict: String,
    pub hosts: Vec<MxHostInfo>,
    pub recommendation: String,
}

pub fn extract_c_class_subnet(ip: &str) -> Option<String> {
    let parts: Vec<&str> = ip.trim().split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        Some(format!("{}.{}.{}.0/24", parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

pub fn evaluate_mx_redundancy_offline(domain: &str, hosts: &[MxHostInfo]) -> MxRedundancyReport {
    let total_mx = hosts.len();

    let mut priorities: Vec<u16> = hosts.iter().map(|h| h.preference).collect();
    priorities.sort_unstable();
    priorities.dedup();
    let distinct_priorities = priorities.len();

    let mut subnets = Vec::new();
    for h in hosts {
        for ip in &h.resolved_ips {
            if let Some(subnet) = extract_c_class_subnet(ip) {
                subnets.push(subnet);
            }
        }
    }
    subnets.sort();
    subnets.dedup();
    let distinct_subnets = subnets.len();

    let has_failover = total_mx >= 2;
    let has_subnet_spof = total_mx >= 2 && distinct_subnets <= 1;

    let (grade, verdict, rec) = if total_mx == 0 {
        (
            "GRADE F: No MX Records Configured".to_string(),
            "Domain cannot receive emails. Mail will bounce with 5.1.2 DNS failure.".to_string(),
            "Configure at least two authoritative MX gateways with distinct preference tiers."
                .to_string(),
        )
    } else if total_mx == 1 {
        (
            "GRADE C: Single MX Host (Zero Failover)".to_string(),
            "Single point of failure (SPOF). Any network glitch or server reboot will reject incoming emails.".to_string(),
            "Add a secondary MX backup relay with higher numerical preference (RFC 5321).".to_string(),
        )
    } else if has_subnet_spof {
        (
            "GRADE B: Multiple MX in Single /24 Subnet".to_string(),
            "Failover hosts exist, but all map to the same /24 network. Common rack, switch, or upstream ISP failure drops all mail.".to_string(),
            "Distribute backup MX servers across distinct autonomous networks or cloud providers.".to_string(),
        )
    } else {
        (
            "GRADE A+: Enterprise Multi-Datacenter High Availability".to_string(),
            "Full RFC 5321 redundancy: multiple MX gateways distributed across distinct network blocks.".to_string(),
            "Optimal inbound infrastructure. Cold outreach replies are safely received without loss.".to_string(),
        )
    };

    MxRedundancyReport {
        domain: domain.trim().to_string(),
        total_mx_records: total_mx,
        distinct_priority_tiers: distinct_priorities,
        distinct_c_class_subnets: distinct_subnets,
        redundancy_grade: grade,
        has_failover_host: has_failover,
        has_subnet_spof,
        deliverability_verdict: verdict,
        hosts: hosts.to_vec(),
        recommendation: rec,
    }
}

pub async fn audit_domain_mx_redundancy(client: &Client, domain: &str) -> MxRedundancyReport {
    let clean_domain = domain
        .trim()
        .to_lowercase()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_matches('/')
        .to_string();

    let mx_url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=MX",
        clean_domain
    );

    let mut hosts = Vec::new();

    if let Ok(resp) = client
        .get(&mx_url)
        .header("Accept", "application/dns-json")
        .send()
        .await
    {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(answers) = json["Answer"].as_array() {
                for ans in answers {
                    if ans["type"].as_u64() == Some(15) {
                        if let Some(data) = ans["data"].as_str() {
                            let parts: Vec<&str> = data.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let pref = parts[0].parse::<u16>().unwrap_or(10);
                                let hostname = parts[1].trim_end_matches('.').to_string();

                                // Resolve A records for this MX host
                                let a_url = format!(
                                    "https://cloudflare-dns.com/dns-query?name={}&type=A",
                                    hostname
                                );
                                let mut ips = Vec::new();
                                if let Ok(a_resp) = client
                                    .get(&a_url)
                                    .header("Accept", "application/dns-json")
                                    .send()
                                    .await
                                {
                                    if let Ok(a_json) = a_resp.json::<Value>().await {
                                        if let Some(a_ans) = a_json["Answer"].as_array() {
                                            for ip_val in a_ans {
                                                if ip_val["type"].as_u64() == Some(1) {
                                                    if let Some(ip_str) = ip_val["data"].as_str() {
                                                        ips.push(ip_str.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                hosts.push(MxHostInfo {
                                    preference: pref,
                                    hostname,
                                    resolved_ips: ips,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    hosts.sort_by_key(|h| h.preference);
    evaluate_mx_redundancy_offline(&clean_domain, &hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_ha_redundancy() {
        let hosts = vec![
            MxHostInfo {
                preference: 10,
                hostname: "mx1.enterprise.com".to_string(),
                resolved_ips: vec!["77.88.21.249".to_string()],
            },
            MxHostInfo {
                preference: 20,
                hostname: "mx2.enterprise.com".to_string(),
                resolved_ips: vec!["87.250.250.249".to_string()],
            },
        ];

        let rep = evaluate_mx_redundancy_offline("enterprise.com", &hosts);
        assert_eq!(rep.total_mx_records, 2);
        assert_eq!(rep.distinct_c_class_subnets, 2);
        assert!(rep.has_failover_host);
        assert!(!rep.has_subnet_spof);
        assert!(rep.redundancy_grade.contains("GRADE A+"));
    }

    #[test]
    fn test_single_subnet_spof() {
        let hosts = vec![
            MxHostInfo {
                preference: 10,
                hostname: "mx1.smallcorp.ru".to_string(),
                resolved_ips: vec!["192.168.1.10".to_string()],
            },
            MxHostInfo {
                preference: 20,
                hostname: "mx2.smallcorp.ru".to_string(),
                resolved_ips: vec!["192.168.1.11".to_string()],
            },
        ];

        let rep = evaluate_mx_redundancy_offline("smallcorp.ru", &hosts);
        assert_eq!(rep.total_mx_records, 2);
        assert_eq!(rep.distinct_c_class_subnets, 1);
        assert!(rep.has_subnet_spof);
        assert!(rep.redundancy_grade.contains("GRADE B"));
    }

    #[test]
    fn test_zero_redundancy_single_mx() {
        let hosts = vec![MxHostInfo {
            preference: 10,
            hostname: "mail.lone.com".to_string(),
            resolved_ips: vec!["10.0.0.1".to_string()],
        }];

        let rep = evaluate_mx_redundancy_offline("lone.com", &hosts);
        assert_eq!(rep.total_mx_records, 1);
        assert!(!rep.has_failover_host);
        assert!(rep.redundancy_grade.contains("GRADE C"));
    }
}
