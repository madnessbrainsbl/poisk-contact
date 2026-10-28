use std::{path::PathBuf, process::Stdio, sync::LazyLock, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Semaphore,
};
use url::Url;

static BROWSERS: Semaphore = Semaphore::const_new(2);
const BRIDGE: &str = include_str!("../../tools/scrapling_fetch.py");

pub fn python_path() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("POISK_SCRAPLING_PYTHON") {
        return (!value.is_empty()).then(|| PathBuf::from(value));
    }
    let relative = if cfg!(windows) {
        ".scrapling/Scripts/python.exe"
    } else {
        ".scrapling/bin/python"
    };
    std::env::current_dir()
        .ok()
        .into_iter()
        .chain(
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_owned())),
        )
        .map(|p| p.join(relative))
        .find(|p| p.is_file())
}

pub(super) fn needs_browser(html: &str) -> bool {
    static SCRIPTS: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<(?:script|style)\b[^>]*>.*?</(?:script|style)\s*>").unwrap()
    });
    static TAGS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"<[^>]*>").unwrap());
    let content = SCRIPTS.replace_all(html, " ");
    let text = TAGS.replace_all(&content, " ");
    if text.contains('@') || text.chars().filter(char::is_ascii_digit).count() >= 7 {
        return false;
    }
    let lower = html.to_lowercase();
    lower.contains("enable javascript")
        || lower.contains("включите javascript")
        || (text.trim().chars().count() < 400
            && lower.contains("<script")
            && [
                "id=\"root\"",
                "id=\"app\"",
                "id=\"__next\"",
                "__next_data__",
            ]
            .iter()
            .any(|m| lower.contains(m)))
}

pub(super) struct Page {
    pub url: Url,
    pub html: String,
}

fn parse_output(raw: &[u8]) -> Result<Page, String> {
    #[derive(serde::Deserialize)]
    struct Response {
        url: String,
        status: u16,
        html: String,
    }
    let response: Response =
        serde_json::from_slice(raw).map_err(|e| format!("Invalid Scrapling response: {e}"))?;
    let url = Url::parse(&response.url).map_err(|e| format!("Invalid Scrapling URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || !(200..300).contains(&response.status)
        || response.html.trim().is_empty()
        || response.html.len() > 4 * 1024 * 1024
    {
        return Err("Scrapling returned an unsuccessful or invalid page".into());
    }
    Ok(Page {
        url,
        html: response.html,
    })
}

pub(super) async fn render(url: &str) -> Result<Page, String> {
    let python =
        python_path().ok_or("Scrapling is not installed; run tools/setup_scrapling.ps1")?;
    let _permit = tokio::time::timeout(Duration::from_secs(2), BROWSERS.acquire())
        .await
        .map_err(|_| "Scrapling browser workers are busy".to_string())?
        .map_err(|e| e.to_string())?;
    let mut command = Command::new(python);
    command
        .args(["-X", "utf8", "-c", BRIDGE])
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start Scrapling: {e}"))?;
    let mut input = child.stdin.take().ok_or("Scrapling stdin unavailable")?;
    let mut stdout = child.stdout.take().ok_or("Scrapling stdout unavailable")?;
    let mut stderr = child.stderr.take().ok_or("Scrapling stderr unavailable")?;
    let request = serde_json::to_vec(&serde_json::json!({"url":url})).map_err(|e| e.to_string())?;
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        input.write_all(&request).await.map_err(|e| e.to_string())?;
        drop(input);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut limited_stdout = (&mut stdout).take(8 * 1024 * 1024);
        let mut limited_stderr = (&mut stderr).take(64 * 1024);
        let (a, b, status) = tokio::join!(
            limited_stdout.read_to_end(&mut out),
            limited_stderr.read_to_end(&mut err),
            child.wait()
        );
        a.map_err(|e| e.to_string())?;
        b.map_err(|e| e.to_string())?;
        if !status.map_err(|e| e.to_string())?.success() {
            let message: String = String::from_utf8_lossy(&err)
                .chars()
                .rev()
                .take(600)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            return Err(format!("Scrapling failed: {message}"));
        }
        parse_output(&out)
    })
    .await
    .map_err(|_| "Scrapling exceeded its 20-second page budget".to_string())?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_javascript_shells_but_not_normal_or_contact_pages() {
        assert!(needs_browser(
            "<html><body><div id=\"root\"></div><script src=\"/app.js\"></script></body></html>"
        ));
        assert!(needs_browser(
            "<html><body>Please enable JavaScript to continue</body></html>"
        ));
        assert!(!needs_browser(
            "<html><body><h1>Studio</h1><p>Call us at +7 391 2345678</p></body></html>"
        ));
        assert!(!needs_browser(
            "<html><body><div id=\"app\">Contact: team@studio.org</div><script src=\"/app.js\"></script></body></html>"
        ));
        assert!(!needs_browser(&format!(
            "<div id=\"app\">{}</div><script src=\"/app.js\"></script>",
            "Useful company text ".repeat(50)
        )));
    }

    #[test]
    fn validates_bridge_output_before_giving_it_to_the_crawler() {
        let good =
            r#"{"url":"https://studio.org/contact","status":200,"html":"<html>Contact</html>"}"#;
        assert_eq!(
            parse_output(good.as_bytes()).unwrap().url.as_str(),
            "https://studio.org/contact"
        );
        for raw in [
            "not JSON",
            r#"{"url":"file:///secret","status":200,"html":"hello"}"#,
            r#"{"url":"https://studio.org","status":403,"html":"blocked"}"#,
            r#"{"url":"https://studio.org","status":200,"html":""}"#,
        ] {
            assert!(parse_output(raw.as_bytes()).is_err());
        }
    }
}
