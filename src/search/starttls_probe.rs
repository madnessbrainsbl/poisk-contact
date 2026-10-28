use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarttlsProbeResult {
    pub host: String,
    pub port_open: bool,
    pub supports_starttls: bool,
    pub supported_extensions: Vec<String>,
    pub raw_banner: String,
    pub security_rating: String,
}

pub fn parse_ehlo_response(response: &str) -> (bool, Vec<String>) {
    let mut supports_starttls = false;
    let mut extensions = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        // Strip 250- or 250 prefix
        let content = if trimmed.starts_with("250-") || trimmed.starts_with("250 ") {
            &trimmed[4..]
        } else {
            trimmed
        };

        let upper = content.to_uppercase();
        if upper.contains("STARTTLS") {
            supports_starttls = true;
            extensions.push("STARTTLS (RFC 3207 Encrypted Transit)".to_string());
        } else if upper.starts_with("8BITMIME") {
            extensions.push("8BITMIME (RFC 1652)".to_string());
        } else if upper.starts_with("SMTPUTF8") {
            extensions.push("SMTPUTF8 (RFC 6531 Internationalized Email)".to_string());
        } else if upper.starts_with("PIPELINING") {
            extensions.push("PIPELINING (RFC 2920 Command Batching)".to_string());
        } else if upper.starts_with("SIZE") {
            extensions.push(format!("SIZE ({})", content));
        } else if upper.starts_with("CHUNKING") {
            extensions.push("CHUNKING (RFC 3030 BDAT)".to_string());
        } else if upper.starts_with("ENHANCEDSTATUSCODES") {
            extensions.push("ENHANCEDSTATUSCODES (RFC 2034)".to_string());
        } else if upper.starts_with("DSN") {
            extensions.push("DSN (RFC 3461 Delivery Status)".to_string());
        }
    }

    (supports_starttls, extensions)
}

pub async fn probe_mx_starttls(host: &str, timeout_secs: u64) -> StarttlsProbeResult {
    let clean_host = host.trim().trim_end_matches('.');
    let addr = format!("{}:25", clean_host);

    let connect_res = timeout(Duration::from_secs(timeout_secs), TcpStream::connect(&addr)).await;
    let mut stream = match connect_res {
        Ok(Ok(s)) => s,
        _ => {
            return StarttlsProbeResult {
                host: clean_host.to_string(),
                port_open: false,
                supports_starttls: false,
                supported_extensions: Vec::new(),
                raw_banner: "".to_string(),
                security_rating: "Port 25 Closed or Filtered (Firewall / ISP Egress Block)"
                    .to_string(),
            };
        }
    };

    // Read greeting banner
    let mut banner_buf = [0u8; 1024];
    let banner_read = timeout(Duration::from_secs(3), stream.read(&mut banner_buf)).await;
    let raw_banner = match banner_read {
        Ok(Ok(n)) if n > 0 => String::from_utf8_lossy(&banner_buf[..n]).trim().to_string(),
        _ => "".to_string(),
    };

    // Send EHLO
    let ehlo_cmd = b"EHLO mail.poisk-contact.internal\r\n";
    if stream.write_all(ehlo_cmd).await.is_err() {
        return StarttlsProbeResult {
            host: clean_host.to_string(),
            port_open: true,
            supports_starttls: false,
            supported_extensions: Vec::new(),
            raw_banner,
            security_rating: "Failed to send EHLO handshake".to_string(),
        };
    }

    // Read EHLO response
    let mut ehlo_buf = [0u8; 4096];
    let ehlo_read = timeout(Duration::from_secs(3), stream.read(&mut ehlo_buf)).await;
    let ehlo_resp = match ehlo_read {
        Ok(Ok(n)) if n > 0 => String::from_utf8_lossy(&ehlo_buf[..n]).to_string(),
        _ => "".to_string(),
    };

    // Polite QUIT
    let _ = stream.write_all(b"QUIT\r\n").await;

    let (supports_starttls, extensions) = parse_ehlo_response(&ehlo_resp);
    let security_rating = if supports_starttls {
        "High: STARTTLS Enabled (Enforces RFC 3207 Opportunistic Encryption)".to_string()
    } else {
        "Low: Plaintext Cleartext Only (Vulnerable to Passive Wiretapping)".to_string()
    };

    StarttlsProbeResult {
        host: clean_host.to_string(),
        port_open: true,
        supports_starttls,
        supported_extensions: extensions,
        raw_banner,
        security_rating,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ehlo_with_starttls() {
        let resp = "250-mx.google.com at your service\r\n250-SIZE 157286400\r\n250-8BITMIME\r\n250-STARTTLS\r\n250-ENHANCEDSTATUSCODES\r\n250-PIPELINING\r\n250-CHUNKING\r\n250 SMTPUTF8\r\n";
        let (has_tls, exts) = parse_ehlo_response(resp);
        assert!(has_tls);
        assert!(exts.iter().any(|e| e.contains("STARTTLS")));
        assert!(exts.iter().any(|e| e.contains("8BITMIME")));
        assert!(exts.iter().any(|e| e.contains("SMTPUTF8")));
        assert!(exts.iter().any(|e| e.contains("PIPELINING")));
    }

    #[test]
    fn test_parse_ehlo_without_starttls() {
        let resp =
            "250-old-relay.local Hello\r\n250-PIPELINING\r\n250-SIZE 20480000\r\n250 8BITMIME\r\n";
        let (has_tls, exts) = parse_ehlo_response(resp);
        assert!(!has_tls);
        assert!(!exts.iter().any(|e| e.contains("STARTTLS")));
        assert_eq!(exts.len(), 3);
    }
}
