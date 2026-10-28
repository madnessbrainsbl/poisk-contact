use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpDeliverability {
    Deliverable,
    Undeliverable,
    CatchAll,
    Greylisted,
    PortBlockedOrTimeout,
}

impl SmtpDeliverability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deliverable => "Deliverable",
            Self::Undeliverable => "Undeliverable",
            Self::CatchAll => "CatchAll",
            Self::Greylisted => "Greylisted",
            Self::PortBlockedOrTimeout => "PortBlockedOrTimeout",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SmtpProbeResult {
    pub email: String,
    pub mx_host: String,
    pub deliverability: SmtpDeliverability,
    pub status_code: u16,
    pub raw_response: String,
}

pub fn clean_mx_host(raw_mx: &str) -> String {
    let parts: Vec<&str> = raw_mx.split_whitespace().collect();
    let host = if parts.len() > 1 {
        parts[1]
    } else {
        parts.first().copied().unwrap_or("")
    };
    host.trim_end_matches('.').trim().to_string()
}

pub fn parse_smtp_status_code(line: &str) -> Option<u16> {
    if line.len() >= 3 {
        line[..3].parse::<u16>().ok()
    } else {
        None
    }
}

pub async fn probe_email_deliverability(
    email: &str,
    mx_host: &str,
    connect_timeout_sec: u64,
) -> SmtpProbeResult {
    let clean_host = clean_mx_host(mx_host);
    let target_addr = format!("{}:25", clean_host);

    let connect_res = timeout(
        Duration::from_secs(connect_timeout_sec),
        TcpStream::connect(&target_addr),
    )
    .await;

    let mut stream = match connect_res {
        Ok(Ok(s)) => s,
        _ => {
            return SmtpProbeResult {
                email: email.to_string(),
                mx_host: clean_host,
                deliverability: SmtpDeliverability::PortBlockedOrTimeout,
                status_code: 0,
                raw_response: "TCP connect to port 25 timed out or blocked by network".to_string(),
            };
        }
    };

    let handshake = async {
        let mut buf = vec![0u8; 1024];

        // 1. Read greeting 220
        let n = stream.read(&mut buf).await?;
        let greeting = String::from_utf8_lossy(&buf[..n]).to_string();
        if !greeting.starts_with("220") {
            return Ok((
                SmtpDeliverability::PortBlockedOrTimeout,
                parse_smtp_status_code(&greeting).unwrap_or(0),
                greeting,
            ));
        }

        // 2. Send EHLO
        stream.write_all(b"EHLO mail.verify.local\r\n").await?;
        let n = stream.read(&mut buf).await?;
        let ehlo_resp = String::from_utf8_lossy(&buf[..n]).to_string();
        if !ehlo_resp.contains("250") {
            return Ok((
                SmtpDeliverability::PortBlockedOrTimeout,
                parse_smtp_status_code(&ehlo_resp).unwrap_or(0),
                ehlo_resp,
            ));
        }

        // 3. Send MAIL FROM
        stream
            .write_all(b"MAIL FROM:<probe@mail.verify.local>\r\n")
            .await?;
        let n = stream.read(&mut buf).await?;
        let mail_resp = String::from_utf8_lossy(&buf[..n]).to_string();
        if !mail_resp.starts_with("250") {
            return Ok((
                SmtpDeliverability::PortBlockedOrTimeout,
                parse_smtp_status_code(&mail_resp).unwrap_or(0),
                mail_resp,
            ));
        }

        // 4. Send RCPT TO
        let rcpt_cmd = format!("RCPT TO:<{}>\r\n", email);
        stream.write_all(rcpt_cmd.as_bytes()).await?;
        let n = stream.read(&mut buf).await?;
        let rcpt_resp = String::from_utf8_lossy(&buf[..n]).to_string();
        let code = parse_smtp_status_code(&rcpt_resp).unwrap_or(0);

        // 5. Clean teardown: RSET and QUIT
        let _ = stream.write_all(b"RSET\r\nQUIT\r\n").await;

        let deliverability = match code {
            250..=252 => SmtpDeliverability::Deliverable,
            550..=554 => SmtpDeliverability::Undeliverable,
            421 | 450..=452 => SmtpDeliverability::Greylisted,
            _ => SmtpDeliverability::PortBlockedOrTimeout,
        };

        Ok::<_, std::io::Error>((deliverability, code, rcpt_resp))
    };

    match timeout(Duration::from_secs(4), handshake).await {
        Ok(Ok((deliv, code, raw))) => SmtpProbeResult {
            email: email.to_string(),
            mx_host: clean_host,
            deliverability: deliv,
            status_code: code,
            raw_response: raw.trim().to_string(),
        },
        _ => SmtpProbeResult {
            email: email.to_string(),
            mx_host: clean_host,
            deliverability: SmtpDeliverability::PortBlockedOrTimeout,
            status_code: 0,
            raw_response: "SMTP dialogue timed out".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_mx_host() {
        assert_eq!(
            clean_mx_host("10 aspmx.l.google.com."),
            "aspmx.l.google.com"
        );
        assert_eq!(clean_mx_host("mail.canonical.com"), "mail.canonical.com");
        assert_eq!(
            clean_mx_host("  5  mx1.protonmail.ch. "),
            "mx1.protonmail.ch"
        );
    }

    #[test]
    fn test_parse_smtp_status_code() {
        assert_eq!(parse_smtp_status_code("250 2.1.5 Recipient OK"), Some(250));
        assert_eq!(parse_smtp_status_code("550 5.1.1 User unknown"), Some(550));
        assert_eq!(
            parse_smtp_status_code("451 Temporary local problem"),
            Some(451)
        );
        assert_eq!(parse_smtp_status_code(""), None);
    }

    #[test]
    fn test_deliverability_mapping() {
        assert_eq!(SmtpDeliverability::Deliverable.as_str(), "Deliverable");
        assert_eq!(SmtpDeliverability::Undeliverable.as_str(), "Undeliverable");
        assert_eq!(
            SmtpDeliverability::PortBlockedOrTimeout.as_str(),
            "PortBlockedOrTimeout"
        );
    }
}
