use crate::{mailer::to_base64, models::SmtpConfig};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn validate_config(config: &SmtpConfig) -> Result<(), String> {
    let selector = config.dkim_selector.trim();
    let key = config.dkim_key_path.trim();
    if selector.is_empty() && key.is_empty() {
        return Ok(());
    }
    if selector.is_empty()
        || selector.len() > 63
        || !selector
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(
            "DKIM selector must contain 1-63 ASCII letters, digits, underscores or hyphens.".into(),
        );
    }
    if key.is_empty() || !std::path::Path::new(key).is_file() {
        return Err("DKIM private key file is missing. Sending stopped before connecting.".into());
    }
    Ok(())
}

fn openssl(args: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
    let bundled = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .map(|p| p.join("Git/usr/bin/openssl.exe"));
    let executable = bundled
        .filter(|p| p.is_file())
        .unwrap_or_else(|| "openssl".into());
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start installed OpenSSL for DKIM: {e}"))?;
    let written = child.stdin.take().unwrap().write_all(input);
    if let Err(e) = written {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("Could not pass message to local DKIM signer: {e}"));
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("DKIM signer failed: {e}"))?;
    if !output.status.success() {
        return Err("Local DKIM signing failed. Check the unencrypted RSA private key; no unsigned fallback was used.".into());
    }
    Ok(output.stdout)
}

fn compress_wsp(value: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in value.chars() {
        if c == ' ' || c == '\t' {
            space = true;
        } else {
            if space {
                out.push(' ');
            }
            space = false;
            out.push(c);
        }
    }
    out
}

fn relaxed_body(body: &str) -> String {
    let lines: Vec<_> = body.split("\r\n").map(compress_wsp).collect();
    let mut out = lines.join("\r\n");
    while out.ends_with("\r\n") {
        out.truncate(out.len() - 2);
    }
    if !out.is_empty() {
        out.push_str("\r\n");
    }
    out
}

fn relaxed_header(name: &str, value: &str) -> String {
    format!(
        "{}:{}",
        name.trim().to_ascii_lowercase(),
        compress_wsp(value).trim_start_matches(' ')
    )
}

pub fn sign_message(payload: &str, config: &SmtpConfig, domain: &str) -> Result<String, String> {
    validate_config(config)?;
    if config.dkim_selector.trim().is_empty() {
        return Ok(payload.to_string());
    }
    if domain.is_empty()
        || !domain
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
    {
        return Err("Invalid DKIM signing domain.".into());
    }
    let (head, body) = payload
        .split_once("\r\n\r\n")
        .ok_or("Missing message header/body separator")?;
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in head.split("\r\n") {
        if line.starts_with([' ', '\t']) {
            headers
                .last_mut()
                .ok_or("Invalid folded header")?
                .1
                .push_str(line);
        } else {
            let (name, value) = line.split_once(':').ok_or("Invalid message header")?;
            let name = name.to_ascii_lowercase();
            if headers.iter().any(|(n, _)| n == &name) {
                return Err("Duplicate message headers cannot be signed by this sender.".into());
            }
            headers.push((name, value.to_string()));
        }
    }
    if !headers.iter().any(|(name, _)| name == "from") {
        return Err("DKIM requires a From header.".into());
    }
    let hash = openssl(
        &["dgst", "-sha256", "-binary"],
        relaxed_body(body).as_bytes(),
    )?;
    if hash.len() != 32 {
        return Err("Invalid SHA-256 output from DKIM signer.".into());
    }
    let names = headers
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(":");
    let value = format!(
        "v=1; a=rsa-sha256; c=relaxed/relaxed; d={domain}; s={}; h={names}; bh={}; b=",
        config.dkim_selector.trim(),
        to_base64(&hash)
    );
    let mut signed = String::new();
    for (name, value) in &headers {
        signed.push_str(&relaxed_header(name, value));
        signed.push_str("\r\n");
    }
    signed.push_str(&relaxed_header("DKIM-Signature", &value));
    let signature = openssl(
        &[
            "dgst",
            "-sha256",
            "-sign",
            config.dkim_key_path.trim(),
            "-passin",
            "pass:",
        ],
        signed.as_bytes(),
    )?;
    if signature.len() < 256 {
        return Err("DKIM requires an RSA key of at least 2048 bits.".into());
    }
    let encoded = to_base64(&signature);
    let folded = encoded
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n\t");
    Ok(format!("DKIM-Signature: {value}{folded}\r\n{payload}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaxed_canonicalization_matches_rfc6376() {
        assert_eq!(relaxed_body(" C \r\nD \t E\r\n\r\n\r\n"), " C\r\nD E\r\n");
        assert_eq!(relaxed_body(" \t\r\n\r\n"), "");
        assert_eq!(relaxed_header("B \t", " Y\t Z  "), "b:Y Z");
        assert_eq!(relaxed_body("Привет\t мир  \r\n"), "Привет мир\r\n");
    }

    #[test]
    fn missing_keys_and_injected_selectors_fail_closed() {
        let mut config = SmtpConfig::default();
        assert_eq!(
            sign_message("unchanged", &config, "example.org").unwrap(),
            "unchanged"
        );
        config.dkim_selector = "pc20261019".into();
        assert!(sign_message("message", &config, "example.org").is_err());
        config.dkim_selector = "selector\r\nBcc: another@example.org".into();
        assert!(validate_config(&config).unwrap_err().contains("selector"));
    }
}
