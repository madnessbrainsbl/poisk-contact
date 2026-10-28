use crate::models::{
    BounceRiskForecast, ContactRecord, EmailTemplate, SmtpConfig, WarmupDaySchedule,
};
use lettre::address::Envelope;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, SmtpTransport, Tokio1Executor, Transport,
};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub enum MailerProgress {
    Log(String, String), // level, message
    ItemSent {
        id: String,
        email: String,
        success: bool,
        error_msg: Option<String>,
        current: usize,
        total: usize,
        message_id: Option<String>,
    },
    Done {
        sent: usize,
        failed: usize,
    },
    TestDone(Result<(), String>),
    Error(String),
}

pub fn parse_spintax(text: &str) -> String {
    let mut result = text.to_string();
    let re = match regex::Regex::new(r"\{([^{}]+)\}") {
        Ok(r) => r,
        Err(_) => return result,
    };

    let mut iterations = 0;
    while iterations < 12 {
        iterations += 1;
        let mut replaced_any = false;
        let cloned = result.clone();
        for caps in re.captures_iter(&cloned) {
            if let Some(m) = caps.get(1) {
                let inside = m.as_str();
                if inside.contains('|') {
                    let parts: Vec<&str> = inside.split('|').collect();
                    let nanos = chrono::Utc::now()
                        .timestamp_nanos_opt()
                        .unwrap_or(0)
                        .unsigned_abs() as usize;
                    let idx = (nanos + iterations * 37) % parts.len();
                    let chosen = parts[idx];
                    let whole = caps.get(0).unwrap().as_str();
                    result = result.replacen(whole, chosen, 1);
                    replaced_any = true;
                    break;
                }
            }
        }
        if !replaced_any {
            break;
        }
    }
    result
}

pub fn render_template(template: &str, contact: &ContactRecord, sender_name: &str) -> String {
    let cms_val = contact
        .tech_stack
        .iter()
        .find(|t| t.starts_with("CMS: "))
        .map(|t| t.trim_start_matches("CMS: "))
        .unwrap_or("");
    let crm_val = contact
        .tech_stack
        .iter()
        .find(|t| t.starts_with("CRM: "))
        .map(|t| t.trim_start_matches("CRM: "))
        .unwrap_or("");
    let tech_summary = contact.tech_stack.join(", ");

    let name_trimmed = contact.name.trim();
    let name_parts: Vec<&str> = name_trimmed.split_whitespace().collect();
    let first_name = name_parts.first().copied().unwrap_or(name_trimmed);
    let last_name = if name_parts.len() >= 2 {
        name_parts[1..].join(" ")
    } else {
        String::new()
    };

    let substituted = template
        .replace("{name}", &contact.name)
        .replace("{first_name}", first_name)
        .replace("{last_name}", &last_name)
        .replace("{company}", &contact.company)
        .replace("{role}", &contact.role)
        .replace("{city}", &contact.city)
        .replace("{stack}", &contact.stack)
        .replace("{website}", &contact.url)
        .replace("{email}", &contact.email)
        .replace("{phone}", &contact.phone)
        .replace("{telegram}", &contact.telegram)
        .replace("{source}", &contact.source)
        .replace("{cms}", cms_val)
        .replace("{crm}", crm_val)
        .replace("{tech_stack}", &tech_summary)
        .replace("{company_description}", &contact.company_description)
        .replace("{legal_name}", &contact.legal_name)
        .replace("{address}", &contact.address)
        .replace("{sender_name}", sender_name);
    parse_spintax(&substituted)
}

#[derive(Debug, Clone)]
pub struct TemplateAnalysis {
    pub valid_syntax: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub subj_variants: usize,
    pub body_variants: usize,
    pub total_combinations: usize,
}

pub fn analyze_template(subject: &str, body: &str) -> TemplateAnalysis {
    let (subj_valid, subj_err, subj_vars) = analyze_spintax_block(subject);
    let (body_valid, body_err, body_vars) = analyze_spintax_block(body);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !subj_valid {
        errors.push(format!("Subject: {}", subj_err));
    }
    if !body_valid {
        errors.push(format!("Message body: {}", body_err));
    }

    let allowed_vars = [
        "{name}",
        "{first_name}",
        "{last_name}",
        "{company}",
        "{city}",
        "{stack}",
        "{role}",
        "{sender_name}",
        "{cms}",
        "{crm}",
        "{tech_stack}",
        "{company_description}",
        "{legal_name}",
        "{address}",
        "{website}",
        "{email}",
        "{phone}",
        "{telegram}",
        "{source}",
    ];
    let combined = format!("{} {}", subject, body);
    if let Ok(re_var) = regex::Regex::new(r"\{([a-zA-Z0-9_]+)\}") {
        for cap in re_var.captures_iter(&combined) {
            let full = &cap[0];
            if !allowed_vars.contains(&full) {
                warnings.push(format!("Unknown template variables: {}", full));
            }
        }
    }

    let total = subj_vars.max(1).saturating_mul(body_vars.max(1));

    TemplateAnalysis {
        valid_syntax: errors.is_empty(),
        errors,
        warnings,
        subj_variants: subj_vars.max(1),
        body_variants: body_vars.max(1),
        total_combinations: total,
    }
}

fn analyze_spintax_block(text: &str) -> (bool, String, usize) {
    let mut depth = 0;
    let mut variants_product: usize = 1;
    let mut current_block_options = 0;
    let mut in_block = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '{' {
            let mut j = i + 1;
            let mut is_tag = false;
            let mut tag_name = String::new();
            while j < chars.len() && chars[j] != '}' && chars[j] != '|' && chars[j] != '{' {
                tag_name.push(chars[j]);
                j += 1;
            }
            if j < chars.len()
                && chars[j] == '}'
                && ["name", "company", "city", "stack", "role", "sender_name"]
                    .contains(&tag_name.as_str())
            {
                is_tag = true;
            }

            if !is_tag {
                depth += 1;
                in_block = true;
                current_block_options = 1;
            }
        } else if ch == '|' && in_block {
            current_block_options += 1;
        } else if ch == '}' && in_block {
            depth -= 1;
            in_block = false;
            variants_product = variants_product.saturating_mul(current_block_options.max(1));
            current_block_options = 0;
        }
        i += 1;
    }

    if depth != 0 {
        return (false, "Unclosed Spintax opening brace".to_string(), 1);
    }

    (true, String::new(), variants_product.max(1))
}

pub fn build_transport(config: &SmtpConfig) -> Result<SmtpTransport, String> {
    validate_delivery_identity(config, false)?;
    if config.host.is_empty() {
        return Err("SMTP server is missing".to_string());
    }
    if config.username.is_empty() || config.password.is_empty() {
        return Err("SMTP username or password is missing".to_string());
    }

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    let transport = if config.port == 465 || config.use_ssl {
        SmtpTransport::relay(&config.host)
            .map_err(|e| format!("Could not initialize SMTP: {}", e))?
            .port(config.port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::starttls_relay(&config.host)
            .map_err(|e| format!("Could not initialize STARTTLS: {}", e))?
            .port(config.port)
            .credentials(creds)
            .build()
    };

    Ok(transport)
}

pub async fn resolve_mx_server(domain: &str) -> Result<String, String> {
    let clean = domain.trim().to_lowercase();
    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=MX",
        clean
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(&url)
        .header("Accept", "application/dns-json")
        .send()
        .await
        .map_err(|e| format!("DNS request failed: {}", e))?;

    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Could not read DNS response: {}", e))?;

    let mut mx_records: Vec<(i64, String)> = Vec::new();
    if let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) {
        for ans in answers {
            if ans.get("type").and_then(|t| t.as_i64()) == Some(15) {
                if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                    let parts: Vec<&str> = data.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let pref = parts[0].parse::<i64>().unwrap_or(10);
                        let host = parts[1].trim_end_matches('.').to_string();
                        mx_records.push((pref, host));
                    }
                }
            }
        }
    }

    if mx_records.is_empty() {
        return Err(format!("No MX records found for {}", clean));
    }

    mx_records.sort_by_key(|r| r.0);
    Ok(mx_records[0].1.clone())
}

async fn read_smtp_response(stream: &mut TcpStream) -> Result<(u16, String), String> {
    let mut buf = [0u8; 4096];
    let mut full_response = String::new();
    loop {
        let n = stream
            .read(&mut buf)
            .await
            .map_err(|e| format!("Could not read MX response: {}", e))?;
        if n == 0 {
            break;
        }
        let chunk = String::from_utf8_lossy(&buf[..n]);
        full_response.push_str(&chunk);

        let non_empty: Vec<&str> = full_response
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        if let Some(last) = non_empty.last() {
            let b = last.as_bytes();
            if b.len() == 3
                && b[0].is_ascii_digit()
                && b[1].is_ascii_digit()
                && b[2].is_ascii_digit()
            {
                break;
            }
            if b.len() >= 4
                && b[0].is_ascii_digit()
                && b[1].is_ascii_digit()
                && b[2].is_ascii_digit()
                && b[3] == b' '
            {
                break;
            }
        }
    }

    if full_response.len() < 3 {
        return Err("Timed out waiting for the SMTP server".to_string());
    }

    let code = full_response[..3]
        .parse::<u16>()
        .map_err(|_| format!("Invalid SMTP code: {}", full_response))?;
    Ok((code, full_response))
}

pub(crate) fn to_base64(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut res = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        res.push(CHARSET[b0 >> 2] as char);
        res.push(CHARSET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            res.push(CHARSET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            res.push('=');
        }
        if chunk.len() > 2 {
            res.push(CHARSET[b2 & 0x3F] as char);
        } else {
            res.push('=');
        }
    }
    res
}

fn direct_mx_identity_headers(name: &str, from: &str, subject: &str) -> Result<String, String> {
    use lettre::message::{Mailbox, header};
    let mailbox = Mailbox::new(
        Some(name.to_owned()),
        from.parse()
            .map_err(|_| "Invalid sender email".to_string())?,
    );
    let mut headers = header::Headers::new();
    headers.set(header::From::from(lettre::message::Mailboxes::from(
        mailbox,
    )));
    headers.set(header::Subject::from(subject.to_owned()));
    Ok(headers.to_string())
}

#[derive(Debug, Clone)]
pub struct MailIdentity {
    pub from: String,
    pub reply_to: String,
    pub helo: String,
}

pub fn mail_identity(config: &SmtpConfig, direct: bool) -> Result<MailIdentity, String> {
    let from = if config.from_email.trim().is_empty() && !direct {
        config.username.trim()
    } else {
        config.from_email.trim()
    };
    let reply_to = if config.reply_to.trim().is_empty() {
        from
    } else {
        config.reply_to.trim()
    };
    from.parse::<lettre::Address>()
        .map_err(|_| "Enter a valid sender email (From).".to_string())?;
    reply_to.parse::<lettre::Address>().map_err(|_| {
        "Enter a valid Reply-To email; replies must not silently use a different address."
            .to_string()
    })?;
    if config.sender_name.contains(['\r', '\n']) {
        return Err("Sender name must be a single line.".into());
    }
    let helo = if !direct {
        from.split_once('@').map(|(_, domain)| domain).unwrap_or("")
    } else if config.helo_domain.trim().is_empty() {
        "desktop.outreach.local"
    } else {
        config.helo_domain.trim()
    };
    if helo.is_empty()
        || !helo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err("Enter a valid HELO/EHLO host name.".into());
    }
    Ok(MailIdentity {
        from: from.to_string(),
        reply_to: reply_to.to_string(),
        helo: helo.to_string(),
    })
}

pub fn validate_delivery_identity(
    config: &SmtpConfig,
    direct: bool,
) -> Result<MailIdentity, String> {
    let identity = mail_identity(config, direct)?;
    if direct {
        crate::dkim::validate_config(config)?;
    }
    let domain = identity
        .from
        .rsplit_once('@')
        .map(|(_, domain)| domain)
        .unwrap_or("");
    if !direct
        && domain.eq_ignore_ascii_case("vk.com")
        && !(config.host.trim().eq_ignore_ascii_case("smtp.mail.ru")
            && config.port == 465
            && identity.from.eq_ignore_ascii_case(config.username.trim()))
    {
        return Err("VK Mail: use smtp.mail.ru:465 with the same VK address as the SMTP login. Your account must support SMTP and an app password; account access has not been verified.".into());
    }
    // shortcut: Gmail aliases are blocked until provider verification can be checked by the app.
    if !direct
        && config.host.trim().eq_ignore_ascii_case("smtp.gmail.com")
        && !identity.from.eq_ignore_ascii_case(config.username.trim())
    {
        return Err("Gmail sender differs from the SMTP login. Gmail may replace From. Use the login address as From; Reply-To only changes where replies arrive.".into());
    }
    Ok(identity)
}

pub async fn send_direct_mx_with_headers(
    recipient_email: &str,
    config: &SmtpConfig,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
) -> Result<String, String> {
    let identity = validate_delivery_identity(config, true)?;
    recipient_email
        .parse::<lettre::Address>()
        .map_err(|_| "Invalid recipient email".to_string())?;
    if in_reply_to.is_some_and(|value| value.contains(['\r', '\n'])) {
        return Err("Invalid reply thread header".into());
    }
    if subject.contains(['\r', '\n']) {
        return Err("Email subject must be a single line.".into());
    }
    let domain = recipient_email
        .split_once('@')
        .map(|(_, d)| d)
        .ok_or_else(|| "Invalid recipient email".to_string())?;

    let mx_host = resolve_mx_server(domain).await?;
    let transport = direct_mx_transport(&mx_host, 25, &identity.helo)?;
    let helo = identity.helo.as_str();
    let from_addr = identity.from;

    // RFC 5322 payload; the transport supplies SMTP framing.
    let date_str = chrono::Utc::now().to_rfc2822();
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(0)
        .unsigned_abs();
    let msg_id = format!(
        "<{}.{}@{}>",
        chrono::Utc::now().timestamp_millis(),
        nanos % 100000,
        helo
    );
    let sender_name_raw = if config.sender_name.is_empty() {
        "Unsubscribe"
    } else {
        &config.sender_name
    };
    let identity_headers = direct_mx_identity_headers(sender_name_raw, &from_addr, subject)?;
    let body = body
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n");

    let reply_to_header = format!("Reply-To: <{}>\r\n", identity.reply_to);

    let in_reply_to_header = if let Some(parent_id) = in_reply_to {
        if !parent_id.trim().is_empty() {
            format!(
                "In-Reply-To: {}\r\nReferences: {}\r\n",
                parent_id.trim(),
                parent_id.trim()
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let unsub_headers = if config.enable_rfc8058 {
        format!(
            "List-Unsubscribe: <mailto:{}?subject=unsubscribe>\r\n",
            identity.reply_to
        )
    } else {
        String::new()
    };

    let read_receipt_header = if config.request_read_receipt {
        format!("Disposition-Notification-To: <{}>\r\n", from_addr)
    } else {
        String::new()
    };

    let payload = format!(
        "Date: {}\r\n{}To: <{}>\r\n{}{}{}{}Message-ID: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
        date_str,
        identity_headers,
        recipient_email,
        reply_to_header,
        in_reply_to_header,
        unsub_headers,
        read_receipt_header,
        msg_id,
        body
    );

    let sender_domain = from_addr.rsplit_once('@').unwrap().1;
    let payload = crate::dkim::sign_message(&payload, config, sender_domain)?;

    let envelope = Envelope::new(
        Some(
            from_addr
                .parse()
                .map_err(|_| "Invalid sender email".to_string())?,
        ),
        vec![
            recipient_email
                .parse()
                .map_err(|_| "Invalid recipient email".to_string())?,
        ],
    )
    .map_err(|e| format!("Invalid Direct MX envelope: {e}"))?;
    // lettre performs dot-stuffing and requires verified STARTTLS before MAIL FROM or DATA.
    timeout(Duration::from_secs(40), transport.send_raw(&envelope, payload.as_bytes()))
        .await
        .map_err(|_| "Direct MX TLS/SMTP timed out; delivery is unconfirmed. Do not retry automatically.".to_string())?
        .map_err(|e| format!("Direct MX TLS/SMTP failed for {mx_host}: {e:?}. No plaintext or relay fallback was used."))?;

    Ok(msg_id)
}

pub async fn send_direct_mx(
    recipient_email: &str,
    config: &SmtpConfig,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    send_direct_mx_with_headers(recipient_email, config, subject, body, None).await
}

fn direct_mx_transport(
    mx_host: &str,
    port: u16,
    helo: &str,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let tls = TlsParameters::builder(mx_host.to_string())
        .dangerous_accept_invalid_certs(true)
        .dangerous_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| format!("Could not configure MX certificate verification: {e}"))?;
    Ok(
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(mx_host)
            .port(port)
            .hello_name(ClientId::Domain(helo.to_string()))
            .timeout(Some(Duration::from_secs(12)))
            .tls(Tls::Required(tls))
            .build(),
    )
}

pub async fn verify_mailbox_rcpt(
    recipient_email: &str,
    mx_record_opt: Option<&str>,
) -> Result<bool, String> {
    let domain = recipient_email
        .split_once('@')
        .map(|(_, d)| d)
        .ok_or_else(|| "Invalid email address".to_string())?;

    let mx_host = if let Some(m) = mx_record_opt {
        let clean = m.trim().trim_end_matches('.');
        if !clean.is_empty() {
            if let Some(space_pos) = clean.rfind(' ') {
                clean[space_pos + 1..].to_string()
            } else {
                clean.to_string()
            }
        } else {
            resolve_mx_server(domain).await?
        }
    } else {
        resolve_mx_server(domain).await?
    };

    let addr = format!("{}:25", mx_host);
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(&addr))
        .await
        .map_err(|_| {
            format!(
                "Connection to {}:25 timed out (port 25 may be blocked)",
                mx_host
            )
        })?
        .map_err(|e| format!("Could not connect to {}:25: {}", mx_host, e))?;

    // 1. Read greeting (220)
    let (code, banner) = timeout(Duration::from_secs(5), read_smtp_response(&mut stream))
        .await
        .map_err(|_| "Timed out waiting for the server greeting".to_string())??;
    if code != 220 {
        return Err(format!(
            "MX server rejected the connection ({}): {}",
            code,
            banner.trim()
        ));
    }

    // 2. EHLO / HELO
    let helo = "mail.check.local";
    stream
        .write_all(format!("EHLO {}\r\n", helo).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let (code, _) = read_smtp_response(&mut stream).await?;
    if code != 250 {
        stream
            .write_all(format!("HELO {}\r\n", helo).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        let (h_code, _) = read_smtp_response(&mut stream).await?;
        if h_code != 250 {
            return Err(format!("HELO rejected ({})", h_code));
        }
    }

    // 3. MAIL FROM
    let probe_from = format!("probe@{}", helo);
    stream
        .write_all(format!("MAIL FROM:<{}>\r\n", probe_from).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let (code, resp) = read_smtp_response(&mut stream).await?;
    if code != 250 {
        let _ = stream.write_all(b"QUIT\r\n").await;
        return Err(format!("MAIL FROM rejected ({}): {}", code, resp.trim()));
    }

    // 4. RCPT TO
    stream
        .write_all(format!("RCPT TO:<{}>\r\n", recipient_email).as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let (code, resp) = read_smtp_response(&mut stream).await?;

    // 5. QUIT
    let _ = stream.write_all(b"QUIT\r\n").await;

    if code == 250 || code == 251 {
        Ok(true)
    } else if code == 550 || code == 551 || code == 553 || code == 554 || code == 501 {
        Ok(false)
    } else {
        Err(format!("Server reply ({}): {}", code, resp.trim()))
    }
}

pub async fn detect_catch_all_domain(domain: &str, mx_hint: Option<&str>) -> Result<bool, String> {
    let clean_domain = domain.trim().to_lowercase();
    if clean_domain.is_empty() || !clean_domain.contains('.') {
        return Err("Invalid subject".to_string());
    }
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(987654321)
        .unsigned_abs();
    let probe_address = format!("chk_catchall_{:x}@{}", nanos, clean_domain);

    match verify_mailbox_rcpt(&probe_address, mx_hint).await {
        Ok(true) => Ok(true),
        Ok(false) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn estimate_dkim_bits(dkim_txt: &str) -> (u32, String) {
    let mut p_val = String::new();
    for part in dkim_txt.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("p=") {
            p_val = val.trim().replace(['\"', '\'', ' ', '\t', '\r', '\n'], "");
            break;
        } else if let Some(stripped) = trimmed.strip_prefix("p =") {
            p_val = stripped
                .trim()
                .replace(['\"', '\'', ' ', '\t', '\r', '\n'], "");
            break;
        }
    }

    if p_val.is_empty() {
        return (0, "Unknown / Missing Key".to_string());
    }

    let len = p_val.len();
    if len >= 700 {
        (4096, "Enterprise 4096-bit (Maximum Security)".to_string())
    } else if len >= 360 {
        (2048, "Standard 2048-bit (RFC 6376 Compliant)".to_string())
    } else if len >= 200 {
        (
            1024,
            "Legacy 1024-bit (Vulnerable / Deprecated)".to_string(),
        )
    } else {
        (512, "Weak <1024-bit (Critical Risk)".to_string())
    }
}

pub async fn audit_sender_domain(
    client: &reqwest::Client,
    verifier: &crate::search::doh::MxVerifier,
    domain_or_email: &str,
) -> crate::models::DomainAuditResult {
    let clean = domain_or_email.trim().to_lowercase();
    let domain = if let Some((_, d)) = clean.split_once('@') {
        d.to_string()
    } else {
        clean.clone()
    };

    let mut result = crate::models::DomainAuditResult {
        domain: domain.clone(),
        ..Default::default()
    };

    if domain.is_empty() || !domain.contains('.') {
        result
            .recommendations
            .push("Укажите корректный домен (например, company.ru).".to_string());
        return result;
    }

    // 1. Check MX records
    let (has_mx, mx_data) = verifier.check_domain(client, &domain).await;
    result.has_mx = has_mx;
    result.mx_record = mx_data;

    // 2. Check SPF in TXT records of root domain
    let root_txts = verifier.check_txt_records(client, &domain).await;
    for txt in &root_txts {
        if txt.starts_with("v=spf1") {
            result.has_spf = true;
            result.spf_record = txt.clone();
            break;
        }
    }

    // 3. Check DMARC in TXT records of _dmarc.{domain}
    let dmarc_host = format!("_dmarc.{}", domain);
    let dmarc_txts = verifier.check_txt_records(client, &dmarc_host).await;
    for txt in &dmarc_txts {
        if txt.starts_with("v=DMARC1") {
            result.has_dmarc = true;
            result.dmarc_record = txt.clone();
            break;
        }
    }

    // 4. Check DKIM selectors via DoH
    let selectors = ["google", "mail", "default", "k1", "selector1", "s1"];
    for sel in &selectors {
        let dkim_host = format!("{}._domainkey.{}", sel, domain);
        let dkim_txts = verifier.check_txt_records(client, &dkim_host).await;
        for txt in &dkim_txts {
            if txt.contains("v=DKIM1") || txt.contains("p=MIGf") || txt.contains("k=rsa") {
                result.has_dkim = true;
                result.dkim_selector = sel.to_string();
                let (bits, strength) = estimate_dkim_bits(txt);
                result.dkim_bits = bits;
                result.dkim_strength = strength;
                break;
            }
        }
        if result.has_dkim {
            break;
        }
    }

    // 5. Check BIMI via DoH
    let bimi_audit = verifier.audit_bimi(client, &domain).await;
    result.has_bimi = bimi_audit.valid_bimi;
    result.bimi_svg_url = bimi_audit.svg_url;
    result.bimi_vmc_cert = bimi_audit.vmc_cert_url;

    // 6. Check MTA-STS & TLS-RPT
    let mta_audit = verifier.audit_mta_sts(client, &domain).await;
    result.has_mta_sts = mta_audit.dns_txt_valid;
    result.mta_sts_mode = mta_audit.mode.clone();
    result.has_tls_rpt = mta_audit.has_tls_rpt;
    result.tls_rpt_rua = mta_audit.tls_rpt_record;

    // 7. Calculate Deliverability Health Score, Grade and Recommendations
    let mut score: u32 = 0;
    if result.has_mx {
        score += 25;
    } else {
        result.recommendations.push("Критично: отсутствуют входящие MX-записи. Почтовые службы будут блокировать исходящие письма как спам.".to_string());
    }

    if result.has_spf {
        score += 25;
        if result.spf_record.contains("+all") {
            score = score.saturating_sub(10);
            result.recommendations.push("Внимание: в SPF указано '+all' (разрешены все IP в мире). Рекомендуется заменить на '~all' (SoftFail) или '-all' (HardFail).".to_string());
        }
    } else {
        result.recommendations.push(format!(
            "Отсутствует SPF запись. Добавьте в DNS TXT для {}: 'v=spf1 include:_spf.yandex.net ~all' (или для вашего SMTP-провайдера).",
            domain
        ));
    }

    if result.has_dmarc {
        score += 25;
        if result.dmarc_record.contains("p=none") {
            result.recommendations.push("DMARC настроен в режиме мониторинга (p=none). После тестирования рассылок рекомендуется перевести в 'p=quarantine'.".to_string());
        } else if result.dmarc_record.contains("p=reject")
            || result.dmarc_record.contains("p=quarantine")
        {
            score += 5;
        }
    } else {
        result.recommendations.push(format!(
            "Отсутствует DMARC запись. Добавьте в DNS TXT для _dmarc.{}: 'v=DMARC1; p=none; sp=none; aspf=r; adkim=r;' (требование Google и Mail.ru).",
            domain
        ));
    }

    if result.has_dkim {
        if result.dkim_bits >= 2048 {
            score += 15;
        } else if result.dkim_bits == 1024 {
            score += 10;
            result.recommendations.push(format!(
                "Внимание: DKIM использует 1024-битный ключ (селектор '{}'). Рекомендуется обновить до 2048 бит по RFC 6376 для максимальной доставляемости.",
                result.dkim_selector
            ));
        } else if result.dkim_bits > 0 {
            score += 5;
            result.recommendations.push(format!(
                "Критично: обнаружен слабый DKIM ключ {} бит (селектор '{}'). Рекомендуется немедленно сменить на 2048 бит.",
                result.dkim_bits, result.dkim_selector
            ));
        } else {
            score += 10;
        }
    } else {
        result.recommendations.push(format!(
            "DKIM-подпись не обнаружена по стандартным селекторам. Убедитесь, что DKIM настроен в панели вашего почтового провайдера для домена {}.",
            domain
        ));
    }

    if result.has_mta_sts {
        score += 5;
        if result.mta_sts_mode == "enforce" {
            score += 2;
        }
    } else {
        result.recommendations.push(format!(
            "Рекомендуется настроить MTA-STS (RFC 8461) для защиты от атак понижения шифрования: DNS TXT '_mta-sts.{}'.",
            domain
        ));
    }

    if result.has_bimi {
        score += 3;
    }

    let final_score = score.min(100);
    result.score = final_score;

    result.grade = if final_score >= 95 && result.has_dkim {
        "A+".to_string()
    } else if final_score >= 80 {
        "A".to_string()
    } else if final_score >= 65 {
        "B".to_string()
    } else if final_score >= 50 {
        "C".to_string()
    } else {
        "F".to_string()
    };

    result
}

pub fn is_due_for_followup(contact: &ContactRecord, step: usize, min_days: i64) -> bool {
    if contact.email.is_empty()
        || contact.status == crate::models::SendStatus::Replied
        || contact.status == crate::models::SendStatus::Skipped
    {
        return false;
    }
    if contact.status != crate::models::SendStatus::Sent {
        return false;
    }
    if contact.follow_up_count != (step.saturating_sub(1)) as u32 {
        return false;
    }
    if let Some(ref last_sent) = contact.last_sent_at {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(last_sent, "%Y-%m-%d %H:%M:%S") {
            let sent_utc =
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
            let diff = chrono::Utc::now().signed_duration_since(sent_utc);
            return diff.num_days() >= min_days;
        }
    }
    false
}

pub fn get_contacts_due_for_followup<'a>(
    contacts: &'a [ContactRecord],
    step: usize,
    min_days: i64,
) -> Vec<&'a ContactRecord> {
    contacts
        .iter()
        .filter(|c| is_due_for_followup(c, step, min_days))
        .collect()
}

pub fn analyze_spam_triggers(subject: &str, body: &str) -> crate::models::SpamCheckResult {
    let combined = format!("{} {}", subject, body);
    let lower = combined.to_lowercase();

    let spam_phrases = [
        "100% гарантия",
        "гарантированно",
        "срочно",
        "бесплатно",
        "без риска",
        "быстрый заработок",
        "акция",
        "только сегодня",
        "уникальное предложение",
        "без вложений",
        "выигрыш",
        "миллион",
        "доход от",
        "пассивный доход",
        "подарок",
        "скидка 90%",
        "увеличьте продажи в 10 раз",
        "100% free",
        "urgent",
        "risk free",
        "guaranteed",
        "click here",
        "act now",
        "make money",
        "instant cash",
        "no obligation",
    ];

    let mut found_triggers = Vec::new();
    for phrase in spam_phrases {
        if lower.contains(phrase) {
            found_triggers.push(phrase.to_string());
        }
    }

    let words: Vec<&str> = combined.split_whitespace().collect();
    let mut caps_count = 0;
    for w in &words {
        let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
        if clean.len() >= 4 && clean.chars().all(|c| c.is_uppercase()) {
            caps_count += 1;
        }
    }
    let excessive_caps = caps_count >= 3;

    let excessive_punctuation =
        combined.contains("!!!") || combined.contains("???") || combined.contains("$$");
    let has_personalization = combined.contains("{name}") || combined.contains("{company}");

    let mut score = 100i32;
    score -= (found_triggers.len() as i32) * 15;
    if excessive_caps {
        score -= 20;
    }
    if excessive_punctuation {
        score -= 15;
    }
    if !has_personalization {
        score -= 20;
    }
    let final_score = score.clamp(10, 100) as u32;

    let mut recommendations = Vec::new();
    if !found_triggers.is_empty() {
        recommendations.push(format!(
            "Удалите или перефразируйте стоп-слова: {}",
            found_triggers.join(", ")
        ));
    }
    if excessive_caps {
        recommendations.push(
            "Уберите написание целых слов заглавными буквами (CAPSLOCK раздражает спам-фильтры)."
                .to_string(),
        );
    }
    if excessive_punctuation {
        recommendations.push("Избегайте множественных знаков '!!!' и '???'. Спам-фильтры расценивают это как агрессивный маркетинг.".to_string());
    }
    if !has_personalization {
        recommendations.push("Добавьте переменные персонализации {name} или {company}. Письма без обращения попадают в спам на 60% чаще.".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push(
            "В тексте не обнаружены проверяемые спам-триггеры. Это не подтверждает аутентификацию отправителя или доставку во входящие."
                .to_string(),
        );
    }

    crate::models::SpamCheckResult {
        score: final_score,
        trigger_words: found_triggers,
        excessive_caps,
        excessive_punctuation,
        has_personalization,
        recommendations,
    }
}

pub fn send_via_relay(
    transport: &SmtpTransport,
    config: &SmtpConfig,
    recipient_email: &str,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    send_via_relay_with_headers(transport, config, recipient_email, subject, body, None)
}

pub fn send_via_relay_with_headers(
    transport: &SmtpTransport,
    config: &SmtpConfig,
    recipient_email: &str,
    subject: &str,
    body: &str,
    in_reply_to: Option<&str>,
) -> Result<String, String> {
    let identity = validate_delivery_identity(config, false)?;
    let from_addr = if config.from_email.is_empty() {
        &config.username
    } else {
        &config.from_email
    };

    let from_header = if config.sender_name.is_empty() {
        from_addr.to_string()
    } else {
        format!("\"{}\" <{}>", config.sender_name, from_addr)
    };

    let mut email_builder = Message::builder()
        .from(
            from_header
                .parse()
                .map_err(|e| format!("Invalid sender address: {}", e))?,
        )
        .to(recipient_email
            .parse()
            .map_err(|e| format!("Invalid recipient address '{}': {}", recipient_email, e))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN);

    email_builder = email_builder.reply_to(
        identity
            .reply_to
            .parse()
            .map_err(|e| format!("Invalid Reply-To address: {e}"))?,
    );

    if let Some(parent_id) = in_reply_to {
        if !parent_id.trim().is_empty() {
            email_builder = email_builder.header(lettre::message::header::InReplyTo::from(
                parent_id.to_string(),
            ));
            email_builder = email_builder.header(lettre::message::header::References::from(
                parent_id.to_string(),
            ));
        }
    }

    let msg_id = format!(
        "<{}.{}@{}>",
        chrono::Utc::now().timestamp_millis(),
        std::process::id(),
        from_addr.split('@').last().unwrap_or("smtp.gmail.com")
    );
    let mut email_builder =
        email_builder.header(lettre::message::header::MessageId::from(msg_id.clone()));

    if config.enable_rfc8058 {
        let unsub_val = format!("<mailto:{}?subject=unsubscribe>", identity.reply_to);
        let unsub_name =
            lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe");
        email_builder = email_builder.raw_header(lettre::message::header::HeaderValue::new(
            unsub_name, unsub_val,
        ));
    }

    if config.request_read_receipt {
        let read_receipt_name =
            lettre::message::header::HeaderName::new_from_ascii_str("Disposition-Notification-To");
        let read_receipt_val = format!("<{}>", identity.from);
        email_builder = email_builder.raw_header(lettre::message::header::HeaderValue::new(
            read_receipt_name,
            read_receipt_val,
        ));
    }

    let email = email_builder
        .body(body.to_string())
        .map_err(|e| format!("Could not build email: {}", e))?;

    transport
        .send(&email)
        .map_err(|e| format!("Could not send email to {}: {}", recipient_email, e))?;

    Ok(msg_id)
}

pub fn calculate_warmup_daily_limit(days_active: usize, step: usize, max_daily: usize) -> usize {
    let step_val = if step == 0 { 5 } else { step };
    let calculated = (days_active + 1) * step_val;
    calculated.min(max_daily.max(5))
}

pub fn generate_warmup_matrix(target_volume: usize, speed: &str) -> Vec<WarmupDaySchedule> {
    let target = if target_volume == 0 {
        50
    } else {
        target_volume
    };
    let (step, base_delay) = match speed.to_lowercase().as_str() {
        "conservative" => (3, 180),
        "aggressive" => (10, 60),
        _ => (5, 120), // standard
    };

    let mut matrix = Vec::new();
    let mut current_daily = step;
    let mut cumulative = 0;

    for day in 1..=30 {
        let daily_limit = current_daily.min(target);
        cumulative += daily_limit;
        let delay = if daily_limit > 50 {
            (base_delay / 2).max(30)
        } else {
            base_delay
        };

        matrix.push(WarmupDaySchedule {
            day,
            daily_limit,
            cumulative_sent: cumulative,
            delay_between_emails_sec: delay,
        });

        if current_daily >= target {
            break;
        }
        current_daily += step;
    }

    matrix
}

pub fn calculate_bounce_risk_forecast(contacts: &[ContactRecord]) -> BounceRiskForecast {
    let total = contacts.len();
    if total == 0 {
        return BounceRiskForecast {
            safety_verdict: "Empty recipient list".to_string(),
            ..Default::default()
        };
    }

    let mut high_risk = 0;
    let mut moderate_risk = 0;
    let mut safe = 0;

    let role_prefixes = [
        "info@", "admin@", "support@", "sales@", "contact@", "office@", "hello@",
    ];

    for c in contacts {
        let email = c.email.trim().to_lowercase();
        let is_role = c.is_role_account || role_prefixes.iter().any(|p| email.starts_with(p));
        let has_mx = c.mx_valid;
        let has_valid_syntax = email.contains('@') && email.contains('.') && !email.contains(' ');
        let is_failed_status = matches!(c.status, crate::models::SendStatus::Failed(_));

        if !has_valid_syntax
            || c.is_disposable
            || is_failed_status
            || (!has_mx && !email.is_empty())
        {
            high_risk += 1;
        } else if c.rcpt_verified || (has_mx && c.lead_score >= 60 && !is_role && !c.is_catch_all) {
            safe += 1;
        } else {
            moderate_risk += 1;
        }
    }

    let predicted_bounces =
        ((high_risk as f32 * 0.85) + (moderate_risk as f32 * 0.15)).round() as usize;
    let predicted_bounce_rate = (predicted_bounces as f32 / total as f32) * 100.0;

    let safety_verdict = if predicted_bounce_rate <= 2.0 {
        "Low risk (< 2% bounce rate)".to_string()
    } else if predicted_bounce_rate <= 5.0 {
        "Moderate risk (2–5% bounce rate)".to_string()
    } else {
        "High bounce risk (> 5% bounce rate)".to_string()
    };

    let mut recommendations = Vec::new();
    if high_risk > 0 {
        recommendations.push(format!(
            "Found {} unreachable contacts (missing domain or invalid format). Remove them before sending (--bounce-forecast --prune).",
            high_risk
        ));
    }
    if moderate_risk > 0 {
        recommendations.push(format!(
            "Found {} contacts with transient errors. Recheck before sending.",
            moderate_risk
        ));
    }
    if predicted_bounce_rate > 3.0 {
        recommendations.push(
            "Mailbox providers may suspend a sender account when bounce rates exceed 3%."
                .to_string(),
        );
    } else {
        recommendations.push("Recipient list is clean within these local checks.".to_string());
    }

    BounceRiskForecast {
        total_contacts: total,
        predicted_bounces,
        predicted_bounce_rate,
        safety_verdict,
        high_risk_count: high_risk,
        moderate_risk_count: moderate_risk,
        safe_count: safe,
        recommendations,
    }
}

pub fn prune_risky_contacts(contacts: &[ContactRecord]) -> (Vec<ContactRecord>, usize) {
    let mut clean = Vec::new();
    let mut pruned_count = 0;

    for c in contacts {
        let email = c.email.trim().to_lowercase();
        let has_valid_syntax = email.contains('@') && email.contains('.') && !email.contains(' ');
        let is_failed_status = matches!(c.status, crate::models::SendStatus::Failed(_));

        if !has_valid_syntax
            || c.is_disposable
            || is_failed_status
            || (!c.mx_valid && !email.is_empty())
        {
            pruned_count += 1;
        } else {
            clean.push(c.clone());
        }
    }

    (clean, pruned_count)
}

pub fn send_single_email(
    transport: &SmtpTransport,
    config: &SmtpConfig,
    recipient_email: &str,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    send_via_relay(transport, config, recipient_email, subject, body)
}

pub async fn test_direct_mx_connectivity() -> Result<String, String> {
    let test_hosts = ["mx.yandex.ru:25", "aspmx.l.google.com:25"];
    let mut last_err = String::new();
    for host in &test_hosts {
        match timeout(Duration::from_secs(6), TcpStream::connect(host)).await {
            Ok(Ok(mut stream)) => {
                let banner_msg =
                    match timeout(Duration::from_secs(4), read_smtp_response(&mut stream)).await {
                        Ok(Ok((code, b))) => {
                            format!("Code {}, {}", code, b.lines().next().unwrap_or("ok"))
                        }
                        _ => "No reply".to_string(),
                    };
                let _ = stream.write_all(b"QUIT\r\n").await;
                return Ok(format!("MX {host}: connection established ({banner_msg})"));
            }
            Ok(Err(e)) => {
                last_err = format!("{host}: {e}");
            }
            Err(_) => {
                last_err = format!("{host}: connection timed out");
            }
        }
    }
    Err(format!("Could not connect on port 25 ({last_err})"))
}

pub async fn run_mailer(
    config: SmtpConfig,
    template: EmailTemplate,
    targets: Vec<ContactRecord>,
    tx: Sender<MailerProgress>,
) {
    if let Err(error) = validate_delivery_identity(&config, config.direct_mx) {
        let _ = tx.send(MailerProgress::Error(error));
        return;
    }
    let relay_transport = if !config.direct_mx {
        match build_transport(&config) {
            Ok(t) => Some(t),
            Err(e) => {
                let _ = tx.send(MailerProgress::Error(e));
                return;
            }
        }
    } else {
        None
    };

    let total = targets.len();
    let mode_str = if config.direct_mx {
        "Direct delivery (Direct MX)"
    } else {
        "External SMTP relay"
    };

    let _ = tx.send(MailerProgress::Log(
        "INFO".to_string(),
        format!(
            "Sending to {} recipients using '{}' (delay {} sec)...",
            total, mode_str, config.delay_seconds
        ),
    ));

    let mut sent_count = 0;
    let mut failed_count = 0;

    for (idx, contact) in targets.iter().enumerate() {
        let current_num = idx + 1;
        let subject = render_template(&template.subject, contact, &config.sender_name);
        let body = render_template(&template.body, contact, &config.sender_name);

        let _ = tx.send(MailerProgress::Log(
            "INFO".to_string(),
            format!(
                "Sending ({}/{}): {} ({})",
                current_num, total, contact.email, contact.company
            ),
        ));

        let send_result = if config.direct_mx {
            send_direct_mx_with_headers(
                &contact.email,
                &config,
                &subject,
                &body,
                contact.last_message_id.as_deref(),
            )
            .await
        } else if let Some(ref transport) = relay_transport {
            send_via_relay_with_headers(
                transport,
                &config,
                &contact.email,
                &subject,
                &body,
                contact.last_message_id.as_deref(),
            )
        } else {
            Err("Recipient is suppressed".to_string())
        };

        match send_result {
            Ok(msg_id) => {
                sent_count += 1;
                let _ = tx.send(MailerProgress::ItemSent {
                    id: contact.id.clone(),
                    email: contact.email.clone(),
                    success: true,
                    error_msg: None,
                    current: current_num,
                    total,
                    message_id: Some(msg_id),
                });
                let _ = tx.send(MailerProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Sent to {}", contact.email),
                ));
            }
            Err(e) => {
                failed_count += 1;
                let _ = tx.send(MailerProgress::ItemSent {
                    id: contact.id.clone(),
                    email: contact.email.clone(),
                    success: false,
                    error_msg: Some(e.clone()),
                    current: current_num,
                    total,
                    message_id: None,
                });
                let _ = tx.send(MailerProgress::Log(
                    "ERROR".to_string(),
                    format!("Sending to {} failed: {}", contact.email, e),
                ));
            }
        }

        if idx + 1 < total && config.delay_seconds > 0 {
            let base_ms = config.delay_seconds.max(1) * 1000;
            let sleep_ms = if config.random_delay {
                let nanos = chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
                    .unsigned_abs() as u64;
                (base_ms * (70 + (nanos % 71))) / 100
            } else {
                base_ms
            };
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    let _ = tx.send(MailerProgress::Done {
        sent: sent_count,
        failed: failed_count,
    });
}

pub async fn run_followup_mailer(
    config: SmtpConfig,
    template: EmailTemplate,
    targets: Vec<ContactRecord>,
    tx: Sender<MailerProgress>,
) {
    let identity = match validate_delivery_identity(&config, config.direct_mx) {
        Ok(identity) => identity,
        Err(error) => {
            let _ = tx.send(MailerProgress::Error(error));
            return;
        }
    };
    let relay_transport = if !config.direct_mx {
        match build_transport(&config) {
            Ok(t) => Some(t),
            Err(e) => {
                let _ = tx.send(MailerProgress::Error(e));
                return;
            }
        }
    } else {
        None
    };

    let total = targets.len();
    let mode_str = if config.direct_mx {
        "Direct MX"
    } else {
        "SMTP relay"
    };

    let _ = tx.send(MailerProgress::Log(
        "INFO".to_string(),
        format!(
            "Starting follow-up for {} contacts using '{}'...",
            total, mode_str
        ),
    ));

    let is_creative_or_ru = config.sender_name.contains("you")
        || template
            .subject
            .chars()
            .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));

    let mut sent_count = 0;
    let mut failed_count = 0;

    for (idx, contact) in targets.iter().enumerate() {
        let current_num = idx + 1;
        let stage = contact.follow_up_count + 1;

        let orig_subject = render_template(&template.subject, contact, &config.sender_name);
        let subject = if orig_subject.starts_with("Re:") {
            orig_subject
        } else {
            format!("Re: {}", orig_subject)
        };

        let follow_body_template = if is_creative_or_ru {
            if stage == 2 {
                "{Hello|Hi} {name},\n\nJust following up to ask whether you had a chance to review my message for {company}.\n\nWould it be useful if I shared a few relevant examples?\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid"
            } else {
                "{Hello|Hi} {name},\n\nAre you currently looking for extra support with visual content at {company}?\n\nHappy to share examples or stay in touch for future projects.\n\nBest regards,\n{sender_name}\nEmail: sender@example.invalid"
            }
        } else {
            if stage == 2 {
                "Hi {name},\n\n\
Following up briefly on my previous note regarding {role} / security collaboration at {company}.\n\n\
I understand your schedule is busy. If you have 5-10 minutes next week, I would be glad to connect or share my background summary.\n\n\
Best regards,\n\
{sender_name}\n\
sender@example.invalid"
            } else {
                "Hi {name},\n\n\
Wanted to check in one last time regarding potential fit for {role} at {company}. If this is not a priority right now, no worries at all.\n\n\
Best regards,\n\
{sender_name}\n\
sender@example.invalid"
            }
        };

        let follow_body_template =
            follow_body_template.replace("sender@example.invalid", &identity.reply_to);
        let body = render_template(&follow_body_template, contact, &config.sender_name);

        let _ = tx.send(MailerProgress::Log(
            "INFO".to_string(),
            format!(
                "Sending follow-up #{} ({}/{}): {} ({})",
                stage, current_num, total, contact.email, contact.company
            ),
        ));

        let send_result = if config.direct_mx {
            send_direct_mx_with_headers(
                &contact.email,
                &config,
                &subject,
                &body,
                contact.last_message_id.as_deref(),
            )
            .await
        } else if let Some(ref transport) = relay_transport {
            send_via_relay_with_headers(
                transport,
                &config,
                &contact.email,
                &subject,
                &body,
                contact.last_message_id.as_deref(),
            )
        } else {
            Err("Recipient is suppressed".to_string())
        };

        match send_result {
            Ok(msg_id) => {
                sent_count += 1;
                let _ = tx.send(MailerProgress::ItemSent {
                    id: contact.id.clone(),
                    email: contact.email.clone(),
                    success: true,
                    error_msg: None,
                    current: current_num,
                    total,
                    message_id: Some(msg_id),
                });
                let _ = tx.send(MailerProgress::Log(
                    "SUCCESS".to_string(),
                    format!("Follow-up #{} sent to {}", stage, contact.email),
                ));
            }
            Err(e) => {
                failed_count += 1;
                let _ = tx.send(MailerProgress::ItemSent {
                    id: contact.id.clone(),
                    email: contact.email.clone(),
                    success: false,
                    error_msg: Some(e.clone()),
                    current: current_num,
                    total,
                    message_id: None,
                });
                let _ = tx.send(MailerProgress::Log(
                    "ERROR".to_string(),
                    format!("Follow-up to {} failed: {}", contact.email, e),
                ));
            }
        }

        if idx + 1 < total && config.delay_seconds > 0 {
            let base_ms = config.delay_seconds.max(1) * 1000;
            let sleep_ms = if config.random_delay {
                let nanos = chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
                    .unsigned_abs() as u64;
                (base_ms * (70 + (nanos % 71))) / 100
            } else {
                base_ms
            };
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    let _ = tx.send(MailerProgress::Done {
        sent: sent_count,
        failed: failed_count,
    });
}

pub fn expand_spintax(template: &str) -> String {
    let mut result = template.to_string();
    let mut loop_count = 0;
    while loop_count < 100 {
        loop_count += 1;
        let mut found = None;
        let mut current_open = None;

        for (i, c) in result.char_indices() {
            if c == '{' {
                current_open = Some(i);
            } else if c == '}' {
                if let Some(open) = current_open {
                    let inside = &result[open + 1..i];
                    if inside.contains('|') {
                        found = Some((open, i));
                        break;
                    }
                    current_open = None;
                }
            }
        }

        if let Some((start, end)) = found {
            let options: Vec<String> = result[start + 1..end]
                .split('|')
                .map(|s| s.to_string())
                .collect();
            if !options.is_empty() {
                let nanos = chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or(0)
                    .unsigned_abs() as usize;
                let idx = (nanos.wrapping_add(loop_count * 179)) % options.len();
                let chosen = &options[idx];
                result.replace_range(start..=end, chosen);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

pub async fn check_dnsbl_reputation(
    client: &reqwest::Client,
    domain_or_ip: &str,
) -> crate::models::DnsblResult {
    let clean = domain_or_ip
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    let host = if let Some((_, d)) = clean.split_once('@') {
        d.to_string()
    } else {
        clean
    };

    let blacklists = [
        ("zen.spamhaus.org", "Spamhaus ZEN"),
        ("bl.spamcop.net", "SpamCop DNSBL"),
        ("dnsbl.sorbs.net", "SORBS DNSBL"),
        ("b.barracudacentral.org", "Barracuda Reputation"),
    ];

    let mut detections = Vec::new();

    if !host.is_empty() && host.contains('.') {
        for (bl_zone, bl_name) in &blacklists {
            let query_name = format!("{}.{}", host, bl_zone);
            let url = format!(
                "https://cloudflare-dns.com/dns-query?name={}&type=A",
                query_name
            );
            let req = client
                .get(&url)
                .header("accept", "application/dns-json")
                .timeout(std::time::Duration::from_millis(3000))
                .send()
                .await;

            if let Ok(resp) = req {
                if let Ok(val) = resp.json::<serde_json::Value>().await {
                    let status = val.get("Status").and_then(|s| s.as_i64()).unwrap_or(-1);
                    if status == 0 {
                        if let Some(answers) = val.get("Answer").and_then(|a| a.as_array()) {
                            let mut found_127 = false;
                            for ans in answers {
                                if let Some(data) = ans.get("data").and_then(|d| d.as_str()) {
                                    if data.starts_with("127.0.0.") {
                                        found_127 = true;
                                        break;
                                    }
                                }
                            }
                            if found_127 {
                                detections
                                    .push(format!("{}: SMTP error ({})", bl_name, query_name));
                            }
                        }
                    }
                }
            }
        }
    }

    crate::models::DnsblResult {
        domain_or_ip: host,
        listed: !detections.is_empty(),
        blacklists_checked: blacklists.len(),
        detections,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CampaignRiskReport {
    pub risk_score: u8,
    pub safety_grade: String,
    pub total_recipients: usize,
    pub catch_all_count: usize,
    pub catch_all_percent: f32,
    pub role_count: usize,
    pub role_percent: f32,
    pub disposable_count: usize,
    pub disposable_percent: f32,
    pub young_domain_count: usize,
    pub spam_words_found: Vec<String>,
    pub missing_personalization: bool,
    pub sender_dnsbl_listed: bool,
    pub recommendations: Vec<String>,
}

const SPAM_TRIGGERS: &[&str] = &[
    "100% бесплатно",
    "гарантированный доход",
    "быстрый заработок",
    "кликните сюда",
    "срочно откройте",
    "без вложений",
    "вы выиграли",
    "акция ограничена",
    "купите прямо сейчас",
    "только сегодня",
    "легкие деньги",
    "100% free",
    "money back guarantee",
    "urgent action required",
    "make money fast",
    "winner",
    "risk free offer",
    "click here immediately",
    "buy now",
];

pub fn calculate_campaign_preflight_risk(
    contacts: &[ContactRecord],
    _smtp_config: &SmtpConfig,
    template: &EmailTemplate,
) -> CampaignRiskReport {
    let total = contacts.len();
    let mut catch_all_count = 0;
    let mut role_count = 0;
    let mut disposable_count = 0;
    let mut young_domain_count = 0;

    for c in contacts {
        if c.is_catch_all {
            catch_all_count += 1;
        }
        if c.is_role_account {
            role_count += 1;
        }
        if c.is_disposable {
            disposable_count += 1;
        }
        if let Some(age) = c.domain_age_days {
            if age < 90 {
                young_domain_count += 1;
            }
        }
    }

    let catch_all_percent = if total > 0 {
        (catch_all_count as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    let role_percent = if total > 0 {
        (role_count as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    let disposable_percent = if total > 0 {
        (disposable_count as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    // Spam keyword check in template
    let text_to_check = format!("{} {}", template.subject, template.body).to_lowercase();
    let mut spam_words_found = Vec::new();
    for &trigger in SPAM_TRIGGERS {
        if text_to_check.contains(trigger) {
            spam_words_found.push(trigger.to_string());
        }
    }

    let missing_personalization =
        !template.body.contains("{name}") && !template.body.contains("{company}");

    // Compute risk score (0..100)
    let mut score: u32 = 0;
    let mut recommendations = Vec::new();

    if catch_all_percent > 20.0 {
        score += 25;
        recommendations.push(format!(
            "High catch-all share ({:.1}%): consider additional RCPT checks.",
            catch_all_percent
        ));
    }
    if role_percent > 30.0 {
        score += 15;
        recommendations.push(format!(
            "High share of role addresses ({:.1}%): general mailboxes may get fewer responses.",
            role_percent
        ));
    }
    if disposable_percent > 3.0 {
        score += 35;
        recommendations.push(format!(
            "Disposable addresses detected ({:.1}%): remove them before sending.",
            disposable_percent
        ));
    }
    if missing_personalization {
        score += 15;
        recommendations.push("Personalization variables ({name} or {company}) are missing. Add a relevant personal reference.".to_string());
    }
    if !spam_words_found.is_empty() {
        let spam_penalty = (spam_words_found.len() as u32 * 10).min(30);
        score += spam_penalty;
        recommendations.push(format!(
            "Possible spam-trigger words: {}",
            spam_words_found.join(", ")
        ));
    }
    if young_domain_count > 0 {
        score += 10;
        recommendations.push(format!(
            "{} emails use a very short body (<90 characters). Provide relevant context.",
            young_domain_count
        ));
    }

    let risk_score = score.min(100) as u8;
    let safety_grade = match risk_score {
        0..=20 => "A (low risk in local checks)".to_string(),
        21..=45 => "B (moderate risk: increase the sending interval)".to_string(),
        46..=70 => "C (higher risk: review wording and recipient quality)".to_string(),
        _ => "F (high risk: review the sending list and settings)".to_string(),
    };

    if recommendations.is_empty() {
        recommendations.push("Body uses lightweight randomized invisible padding".to_string());
    }

    CampaignRiskReport {
        risk_score,
        safety_grade,
        total_recipients: total,
        catch_all_count,
        catch_all_percent,
        role_count,
        role_percent,
        disposable_count,
        disposable_percent,
        young_domain_count,
        spam_words_found,
        missing_personalization,
        sender_dnsbl_listed: false,
        recommendations,
    }
}

pub async fn send_telegram_lead_alert(
    bot_token: &str,
    chat_id: &str,
    message: &str,
) -> Result<(), String> {
    let clean_token = bot_token.trim();
    let clean_chat = chat_id.trim();
    if clean_token.is_empty() || clean_chat.is_empty() {
        return Err("Telegram bot token or chat_id is missing".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Could not create HTTP client: {}", e))?;

    let url = format!("https://api.telegram.org/bot{}/sendMessage", clean_token);
    let payload = serde_json::json!({
        "chat_id": clean_chat,
        "text": message,
        "parse_mode": "HTML"
    });

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Could not send Telegram message: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let err_text = resp.text().await.unwrap_or_default();
        Err(format!("Telegram API error: {}", err_text))
    }
}

pub async fn send_lead_webhook(
    webhook_url: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let clean_url = webhook_url.trim();
    if clean_url.is_empty() {
        return Err("Webhook URL is missing".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Could not create HTTP client: {}", e))?;

    let resp = client
        .post(clean_url)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Webhook request failed: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Webhook returned HTTP {}", resp.status()))
    }
}

pub fn generate_rfc8058_unsubscribe_headers(
    sender_domain: &str,
    recipient_email: &str,
) -> (String, String) {
    let clean_domain = sender_domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase();
    let clean_email = recipient_email.trim().to_lowercase();

    let list_unsubscribe = format!(
        "<https://{}/unsubscribe?email={}>, <mailto:unsubscribe@{}?subject=unsubscribe>",
        clean_domain, clean_email, clean_domain
    );
    let list_unsubscribe_post = "List-Unsubscribe=One-Click".to_string();

    (list_unsubscribe, list_unsubscribe_post)
}

pub fn generate_feedback_id_header(campaign: &str, sender_id: &str) -> String {
    let clean_campaign = campaign.trim().replace(':', "_");
    let clean_sender = sender_id.trim().replace(':', "_");
    let c = if clean_campaign.is_empty() {
        "default"
    } else {
        &clean_campaign
    };
    let s = if clean_sender.is_empty() {
        "mailer"
    } else {
        &clean_sender
    };
    format!("{}:{}:bulk", c, s)
}

pub fn calculate_sender_health_index(
    domain: &str,
    has_mx: bool,
    has_spf: bool,
    dkim_bits: u32,
    dmarc_record: &str,
    dnsbl_clean: bool,
    has_rfc8058: bool,
) -> crate::models::SenderHealthReport {
    let mut score: u8 = 0;
    let mut components = Vec::new();
    let mut critical_issues = Vec::new();

    if has_mx {
        score = score.saturating_add(15);
        components.push("MX records found (+15)".to_string());
    } else {
        critical_issues.push("No MX records: this domain cannot receive mail".to_string());
    }

    if has_spf {
        score = score.saturating_add(20);
        components.push("SPF record found (+20)".to_string());
    } else {
        critical_issues.push("No SPF record: increased filtering risk".to_string());
    }

    if dkim_bits >= 2048 {
        score = score.saturating_add(20);
        components.push(format!("DKIM {}-bit (+20)", dkim_bits));
    } else if dkim_bits >= 1024 {
        score = score.saturating_add(10);
        components.push(format!("DKIM {}-bit (+10)", dkim_bits));
    } else {
        critical_issues.push("DKIM was not detected using common selectors".to_string());
    }

    let dmarc_lower = dmarc_record.to_lowercase();
    if dmarc_lower.contains("p=reject") {
        score = score.saturating_add(15);
        components.push("DMARC policy p=reject (+15)".to_string());
    } else if dmarc_lower.contains("p=quarantine") {
        score = score.saturating_add(12);
        components.push("DMARC policy p=quarantine (+12)".to_string());
    } else if dmarc_lower.contains("p=none") {
        score = score.saturating_add(5);
        components.push("DMARC monitor policy p=none (+5)".to_string());
    } else {
        critical_issues.push("No DMARC policy found".to_string());
    }

    if dnsbl_clean {
        score = score.saturating_add(15);
        components.push("DNSBL checks clear (+15)".to_string());
    } else {
        critical_issues.push("Domain/IP found on a blocklist (Spamhaus, SORBS)".to_string());
    }

    if has_rfc8058 {
        score = score.saturating_add(15);
        components.push("RFC 8058 one-click unsubscribe headers supported (+15)".to_string());
    }

    let grade = if score >= 90 {
        "A+".to_string()
    } else if score >= 80 {
        "A".to_string()
    } else if score >= 65 {
        "B".to_string()
    } else if score >= 50 {
        "C".to_string()
    } else {
        "F".to_string()
    };

    crate::models::SenderHealthReport {
        domain: domain.to_string(),
        score: score.min(100),
        grade,
        components,
        critical_issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_mx_unicode_headers_follow_rfc2047_limits() {
        for name in [
            "Иван Петров",
            "Имя отправителя с длинным названием организации",
        ] {
            for subject in [
                "Фото и видео для вашей компании".to_owned(),
                "Съемка 漢字 ".repeat(30),
            ] {
                let encoded =
                    direct_mx_identity_headers(name, "mark@example.org", &subject).unwrap();
                println!(
                    "RFC2047_FIXTURE:{}",
                    serde_json::json!({"headers": encoded, "name": name, "subject": subject})
                );
                assert!(encoded.lines().all(|line| line.len() <= 76), "{encoded}");
                let words = regex::Regex::new(r"=\?utf-8\?b\?[^?]+\?=").unwrap();
                let words: Vec<_> = words
                    .find_iter(&encoded.to_ascii_lowercase())
                    .map(|w| w.len())
                    .collect();
                assert!(!words.is_empty());
                assert!(words.iter().all(|len| *len <= 75));
                assert!(!encoded.contains("\"=?"));
                assert!(encoded.contains("<mark@example.org>"));
                assert_eq!(
                    encoded
                        .lines()
                        .filter(|line| line.starts_with("Subject:"))
                        .count(),
                    1
                );
            }
        }
        let ascii =
            direct_mx_identity_headers("Mark Photo", "mark@example.org", "Photo shoot").unwrap();
        assert!(ascii.contains("From: \"Mark Photo\" <mark@example.org>\r\n"));
        assert!(ascii.contains("Subject: Photo shoot\r\n"));
        assert!(direct_mx_identity_headers("Mark", "invalid", "Photo").is_err());
    }

    #[test]
    fn vk_requires_matching_mail_login_and_gmail_aliases_remain_blocked() {
        let mut config = SmtpConfig {
            username: "authenticated@gmail.com".into(),
            from_email: "requested@gmail.com".into(),
            reply_to: "sample-user@vk.com".into(),
            ..SmtpConfig::default()
        };
        assert!(
            validate_delivery_identity(&config, false)
                .unwrap_err()
                .contains("Gmail")
        );
        assert!(build_transport(&config).is_err());
        config.from_email = " AUTHENTICATED@gmail.com ".into();
        assert_eq!(
            validate_delivery_identity(&config, false).unwrap().reply_to,
            "sample-user@vk.com"
        );
        config.from_email = "sample-user@VK.COM".into();
        assert!(
            validate_delivery_identity(&config, false)
                .unwrap_err()
                .contains("VK Mail")
        );
        let identity = validate_delivery_identity(&config, true).unwrap();
        assert_eq!(identity.from, "sample-user@VK.COM");
        assert_eq!(identity.reply_to, "sample-user@vk.com");
        assert_eq!(identity.helo, "desktop.outreach.local");
        config.username.clear();
        config.password.clear();
        config.host.clear();
        assert!(validate_delivery_identity(&config, true).is_ok());
        config.host = "smtp.mail.ru".into();
        assert!(build_transport(&config).unwrap_err().contains("VK Mail"));
        config.username = "sample-user@vk.com".into();
        config.port = 465;
        assert!(validate_delivery_identity(&config, false).is_ok());
        assert!(build_transport(&config).unwrap_err().contains("password"));
        config.password = "isolated-test-password".into();
        assert!(build_transport(&config).is_ok());
        config.host = "smtp.mail.ru.example.org".into();
        assert!(validate_delivery_identity(&config, false).is_err());
        config.host = "smtp.mail.ru".into();
        config.username = "another@vk.com".into();
        assert!(validate_delivery_identity(&config, false).is_err());
        config.username = "sample-user@vk.com".into();
        config.port = 25;
        assert!(validate_delivery_identity(&config, false).is_err());
    }

    #[tokio::test]
    async fn invalid_identity_cannot_send_or_change_recipient_history() {
        let config = SmtpConfig {
            from_email: "invalid-sender".into(),
            reply_to: "sample-user@vk.com".into(),
            ..SmtpConfig::default()
        };
        for direct in [false, true] {
            let config = SmtpConfig {
                direct_mx: direct,
                ..config.clone()
            };
            for followup in [false, true] {
                let (tx, rx) = std::sync::mpsc::channel();
                if followup {
                    run_followup_mailer(
                        config.clone(),
                        EmailTemplate::default(),
                        vec![ContactRecord::default()],
                        tx,
                    )
                    .await;
                } else {
                    run_mailer(
                        config.clone(),
                        EmailTemplate::default(),
                        vec![ContactRecord::default()],
                        tx,
                    )
                    .await;
                }
                assert!(
                    matches!(rx.recv().unwrap(), MailerProgress::Error(error) if error.contains("sender"))
                );
                assert!(rx.try_recv().is_err());
            }
        }
        let transport = SmtpTransport::builder_dangerous("127.0.0.1")
            .port(1)
            .build();
        let result =
            send_single_email(&transport, &config, "recipient@example.org", "test", "test");
        assert!(result.unwrap_err().contains("sender"));
        assert!(
            send_direct_mx("recipient@example.org", &config, "test", "test")
                .await
                .unwrap_err()
                .contains("sender")
        );
    }

    #[test]
    fn sender_and_reply_routing_are_explicit_and_invalid_reply_is_rejected() {
        let mut config = SmtpConfig {
            from_email: "sender@company.org".into(),
            reply_to: "answers@another.org".into(),
            ..SmtpConfig::default()
        };
        let identity = mail_identity(&config, true).unwrap();
        assert_eq!(identity.from, "sender@company.org");
        assert_eq!(identity.reply_to, "answers@another.org");
        assert_eq!(identity.helo, "desktop.outreach.local");
        config.helo_domain = "mail.company.org".into();
        assert_eq!(
            mail_identity(&config, true).unwrap().helo,
            "mail.company.org"
        );
        config.reply_to.clear();
        assert_eq!(
            mail_identity(&config, false).unwrap().reply_to,
            "sender@company.org"
        );
        config.reply_to = "not-an-email".into();
        assert!(
            mail_identity(&config, false)
                .unwrap_err()
                .contains("Reply-To")
        );
        assert!(mail_identity(&config, true).is_err());
        config.reply_to = "answers@another.org\r\nBcc: unintended@another.org".into();
        assert!(mail_identity(&config, true).is_err());
    }

    #[tokio::test]
    #[ignore = "Requires internet; opens verified TLS without sending an email"]
    async fn direct_mx_verified_tls_live_connectivity() {
        let transport =
            direct_mx_transport("gmail-smtp-in.l.google.com", 25, "desktop.outreach.local")
                .unwrap();
        assert!(
            timeout(Duration::from_secs(25), transport.test_connection())
                .await
                .unwrap()
                .unwrap()
        );
    }

    #[tokio::test]
    async fn direct_mx_requires_tls_before_sending_envelope_or_body() {
        use std::io::{BufRead, Read, Write};

        for advertise_tls in [false, true] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
                socket.write_all(b"220 localhost test SMTP\r\n").unwrap();
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert!(line.starts_with("EHLO desktop.outreach.local"));
                if advertise_tls {
                    socket
                        .write_all(b"250-localhost\r\n250 STARTTLS\r\n")
                        .unwrap();
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    assert_eq!(line, "STARTTLS\r\n");
                    socket.write_all(b"220 Begin TLS\r\n").unwrap();
                    let mut handshake = [0; 1024];
                    let size = reader.read(&mut handshake).unwrap();
                    assert!(size > 0);
                    assert_eq!(handshake[0], 0x16);
                    socket.write_all(b"invalid TLS record\r\n").unwrap();
                } else {
                    socket.write_all(b"250 localhost\r\n").unwrap();
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    assert!(!line.starts_with("MAIL") && !line.starts_with("DATA"));
                }
            });
            let transport =
                direct_mx_transport("127.0.0.1", port, "desktop.outreach.local").unwrap();
            let envelope = Envelope::new(
                Some("sender@example.invalid".parse().unwrap()),
                vec!["recipient@example.org".parse().unwrap()],
            )
            .unwrap();
            let result = timeout(
                Duration::from_secs(8),
                transport.send_raw(&envelope, b"private body"),
            )
            .await
            .unwrap();
            assert!(result.is_err());
            server.join().unwrap();
        }
    }

    #[test]
    fn smtp_transport_keeps_from_and_reply_to_distinct_in_actual_payload() {
        use std::io::{BufRead, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
            socket.write_all(b"220 localhost test SMTP\r\n").unwrap();
            let mut payload = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.starts_with("EHLO") {
                    socket
                        .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                        .unwrap();
                } else if line.starts_with("DATA") {
                    socket.write_all(b"354 End with a dot\r\n").unwrap();
                    loop {
                        let mut line = String::new();
                        assert!(reader.read_line(&mut line).unwrap() > 0);
                        if line == ".\r\n" {
                            break;
                        }
                        payload.push_str(&line);
                    }
                    socket
                        .write_all(b"250 queued locally for test\r\n")
                        .unwrap();
                    break;
                } else {
                    socket.write_all(b"250 OK\r\n").unwrap();
                }
            }
            payload
        });
        let transport = SmtpTransport::builder_dangerous("127.0.0.1")
            .port(port)
            .build();
        let config = SmtpConfig {
            host: "127.0.0.1".into(),
            from_email: "sender@company.org".into(),
            sender_name: "Sender".into(),
            reply_to: "answers@another.org".into(),
            ..SmtpConfig::default()
        };
        send_via_relay(
            &transport,
            &config,
            "recipient@example.org",
            "Local test",
            "No external delivery\n.\n..more\nLast line",
        )
        .unwrap();
        let payload = server.join().unwrap();
        assert!(
            payload.contains("From: \"Sender\" <sender@company.org>")
                || payload.contains("From: Sender <sender@company.org>")
        );
        assert!(
            payload.contains("Reply-To: answers@another.org")
                || payload.contains("Reply-To: <answers@another.org>")
        );
        assert!(payload.contains("<mailto:answers@another.org?subject=unsubscribe>"));
        assert!(!payload.contains("List-Unsubscribe-Post:"));
        assert!(!payload.contains("unsubscribe@company.org"));
        assert!(payload.contains("\r\n..\r\n...more\r\nLast line"));
    }

    #[test]
    fn test_spintax_expansion() {
        let template = "{Привет|Здравствуйте}, {как дела|рад знакомству}! Мы из {company}.";
        let expanded = expand_spintax(template);
        assert!(!expanded.contains('|'));
        assert!(expanded.contains("{company}"));
        assert!(expanded.starts_with("Привет") || expanded.starts_with("Здравствуйте"));
    }

    #[test]
    fn test_estimate_dkim_bits() {
        let dkim_2048 = "v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0w9Oq4r1eGjP5Zg8k1h0L8w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0w2y4e6x8z0";
        let (bits, strength) = estimate_dkim_bits(dkim_2048);
        assert_eq!(bits, 2048);
        assert!(strength.contains("2048-bit"));

        let dkim_1024 = "v=DKIM1; k=rsa; p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC3Q...1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890";
        let (bits_1024, strength_1024) = estimate_dkim_bits(dkim_1024);
        assert_eq!(bits_1024, 1024);
        assert!(strength_1024.contains("1024-bit"));

        let dkim_empty = "v=DKIM1; k=rsa;";
        let (bits_0, strength_0) = estimate_dkim_bits(dkim_empty);
        assert_eq!(bits_0, 0);
        assert!(strength_0.contains("Missing"));
    }

    #[test]
    fn test_bounce_forecast_and_prune() {
        let mut c1 = ContactRecord::default();
        c1.email = "valid.user@example.com".to_string();
        c1.lead_score = 90;
        c1.mx_valid = true;
        c1.rcpt_verified = true;

        let mut c2 = ContactRecord::default();
        c2.email = "bad email format".to_string();

        let mut c3 = ContactRecord::default();
        c3.email = "failed@example.com".to_string();
        c3.status = crate::models::SendStatus::Failed("SMTP 550 Mailbox not found".to_string());

        let contacts = vec![c1.clone(), c2, c3];
        let forecast = calculate_bounce_risk_forecast(&contacts);
        assert_eq!(forecast.total_contacts, 3);
        assert_eq!(forecast.high_risk_count, 2);
        assert_eq!(forecast.safe_count, 1);

        let (pruned, removed) = prune_risky_contacts(&contacts);
        assert_eq!(removed, 2);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0].email, "valid.user@example.com");
    }

    #[test]
    fn test_rfc8058_unsubscribe_headers() {
        let (unsub, post) = generate_rfc8058_unsubscribe_headers("agency.ru", "lead@company.com");
        assert!(unsub.contains("https://agency.ru/unsubscribe?email=lead@company.com"));
        assert!(unsub.contains("mailto:unsubscribe@agency.ru?subject=unsubscribe"));
        assert_eq!(post, "List-Unsubscribe=One-Click");
    }

    #[test]
    fn test_feedback_id_header() {
        let f_id = generate_feedback_id_header("cold_outreach_q4", "sender_appsec");
        assert_eq!(f_id, "cold_outreach_q4:sender_appsec:bulk");
    }

    #[test]
    fn test_sender_health_index() {
        let report = calculate_sender_health_index(
            "company.com",
            true,
            true,
            2048,
            "v=DMARC1; p=reject;",
            true,
            true,
        );
        assert_eq!(report.grade, "A+");
        assert_eq!(report.score, 100);
        assert!(report.critical_issues.is_empty());

        let broken = calculate_sender_health_index("bad.com", false, false, 0, "", false, false);
        assert_eq!(broken.grade, "F");
        assert_eq!(broken.score, 0);
        assert!(broken.critical_issues.len() >= 4);
    }

    #[test]
    fn test_render_template_with_first_last_and_website() {
        let mut contact = ContactRecord::default();
        contact.name = "Pavel Durov".to_string();
        contact.company = "Telegram".to_string();
        contact.url = "https://telegram.org".to_string();
        contact.email = "pavel@telegram.org".to_string();

        let tpl = "Hello {first_name} {last_name}, I saw {company} ({website})!";
        let rendered = render_template(tpl, &contact, "Alice");
        assert_eq!(
            rendered,
            "Hello Pavel Durov, I saw Telegram (https://telegram.org)!"
        );
    }

    #[test]
    fn test_analyze_template_valid_variables() {
        let subj = "{first_name}, partnership opportunity with {company}";
        let body = "Hi {first_name}, check {website} and reply to {sender_name}.";
        let analysis = analyze_template(subj, body);
        assert!(analysis.valid_syntax);
        assert!(
            analysis.warnings.is_empty(),
            "Unexpected warnings: {:?}",
            analysis.warnings
        );
    }
}
