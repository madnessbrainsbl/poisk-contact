use super::extractor::unescape_html_entities;
use regex::Regex;
use reqwest::Client;
use std::{collections::HashMap, sync::LazyLock, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use url::Url;

#[derive(Default)]
struct SearchState {
    next_request: HashMap<String, Instant>,
    blocked: HashMap<String, (Instant, String)>,
    cache: HashMap<String, (Instant, Vec<SearchHit>)>,
}

static SEARCH_STATE: LazyLock<Mutex<SearchState>> =
    LazyLock::new(|| Mutex::new(SearchState::default()));
const REQUEST_INTERVAL: Duration = Duration::from_secs(2);
const CACHE_TTL: Duration = Duration::from_secs(300);

static ANCHOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<a\b([^>]*)>.*?</a\s*>").unwrap());
static CLASS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(?:^|\s)class\s*=\s*["']([^"']+)["']"#).unwrap());
static HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(?:^|\s)href\s*=\s*["']([^"']+)["']"#).unwrap());
static YAHOO_RESULT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(?:^|\s)data-matarget\s*=\s*["']algo["']"#).unwrap());

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub url: String,
}

pub async fn search_web(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    if query.trim().is_empty() {
        return Err("Enter a search query before searching the web.".into());
    }
    let mut api_error = None;
    if let Some(base) = std::env::var("SEARXNG_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    {
        match search_searxng(client, base.trim(), query, limit).await {
            Ok(hits) => return Ok(hits),
            Err(error) => api_error = Some(format!("SearXNG: {error}")),
        }
    }
    if let Some(key) = std::env::var("BRAVE_SEARCH_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
    {
        match search_brave_api(client, key.trim(), query, limit).await {
            Ok(hits) => return Ok(hits),
            Err(error) => api_error = Some(format!("Brave API: {error}")),
        }
    }
    search_providers(
        client,
        query,
        limit,
        &[
            ("Brave", "https://search.brave.com/search", "q"),
            ("DuckDuckGo", "https://html.duckduckgo.com/html/", "q"),
            ("Yahoo", "https://search.yahoo.com/search", "p"),
            ("Mwmbl", "https://api.mwmbl.org/search/", "s"),
        ],
    )
    .await
    .map_err(|error| match api_error {
        Some(api) => format!("{api}; {error}"),
        None => error,
    })
}

// Self-hosted SearXNG metasearch; `json` must be listed under `search.formats` in settings.yml.
async fn search_searxng(
    client: &Client,
    base: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let mut hits = Vec::new();
    for page in 1..=3 {
        let mut url = Url::parse(base)
            .and_then(|u| u.join("search"))
            .map_err(|e| format!("invalid SEARXNG_URL: {e}"))?;
        url.query_pairs_mut()
            .append_pair("q", query)
            .append_pair("format", "json")
            .append_pair("pageno", &page.to_string());
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(
                "JSON output is disabled; add `json` to search.formats in settings.yml".into(),
            );
        }
        let body = response
            .error_for_status()
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let before = hits.len();
        // SearXNG's {"results":[{"url"}]} is Brave's `web` object, so the same reader applies.
        parse_brave_api_json(&format!(r#"{{"web":{body}}}"#), &mut hits)?;
        if hits.len() >= limit || hits.len() == before {
            break;
        }
    }
    if hits.is_empty() {
        return Err("no results for this query".into());
    }
    hits.truncate(limit);
    Ok(hits)
}

// Official Brave Search API (https://api-dashboard.search.brave.com). The free plan allows
// 1 request per second, 20 results per page and offsets 0..=9.
async fn search_brave_api(
    client: &Client,
    key: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let mut hits = Vec::new();
    for offset in 0..10 {
        if offset > 0 {
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }
        let mut url = Url::parse("https://api.search.brave.com/res/v1/web/search").unwrap();
        url.query_pairs_mut()
            .append_pair("q", query)
            .append_pair("count", "20")
            .append_pair("offset", &offset.to_string());
        let response = client
            .get(url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", key)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = response.status();
        if !status.is_success() {
            if !hits.is_empty() {
                break;
            }
            return Err(match status.as_u16() {
                401 | 403 | 422 => "the API key was rejected; check BRAVE_SEARCH_API_KEY".into(),
                429 => "the API plan limit was reached; wait or raise the plan".into(),
                code => format!("HTTP {code}"),
            });
        }
        let body = response
            .text()
            .await
            .map_err(|e| format!("unreadable response: {e}"))?;
        let more = parse_brave_api_json(&body, &mut hits)?;
        if hits.len() >= limit || !more {
            break;
        }
    }
    if hits.is_empty() {
        return Err("no results for this query".into());
    }
    hits.truncate(limit);
    Ok(hits)
}

/// Appends new result URLs and reports whether another page is available.
fn parse_brave_api_json(body: &str, hits: &mut Vec<SearchHit>) -> Result<bool, String> {
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("unreadable response: {e}"))?;
    let results = json["web"]["results"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for url in results.iter().filter_map(|r| r["url"].as_str()) {
        if url.starts_with("http") && !hits.iter().any(|h| h.url == url) {
            hits.push(SearchHit {
                url: url.to_string(),
            });
        }
    }
    Ok(results.len() >= 20 && json["query"]["more_results_available"].as_bool() != Some(false))
}

async fn search_providers(
    client: &Client,
    query: &str,
    limit: usize,
    providers: &[(&str, &str, &str)],
) -> Result<Vec<SearchHit>, String> {
    let mut errors = Vec::new();
    for (provider, endpoint, parameter) in providers {
        let mut url = Url::parse(endpoint).unwrap();
        let query = if *parameter == "s" {
            query
                .strip_suffix(" contact")
                .or_else(|| query.strip_suffix(" контакты"))
                .or_else(|| query.strip_suffix(" email"))
                .unwrap_or(query)
        } else {
            query
        };
        url.query_pairs_mut().append_pair(parameter, query);
        let result = search_provider(client, url, limit).await;
        match result {
            Ok(hits) => return Ok(hits),
            Err(error) => errors.push(format!("{provider}: {error}")),
        }
    }
    Err(errors.join("; "))
}

async fn search_provider(
    client: &Client,
    url: Url,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let mut state = SEARCH_STATE.lock().await;
    let key = url.to_string();
    let origin = url.origin().ascii_serialization();
    state.cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
    if let Some((_, hits)) = state.cache.get(&key) {
        return Ok(hits.iter().take(limit).cloned().collect());
    }
    for block_key in [&origin, &key] {
        if let Some((until, reason)) = state.blocked.get(block_key) {
            if *until > Instant::now() {
                return Err(format!(
                    "{reason}; provider paused for another {} seconds",
                    until.saturating_duration_since(Instant::now()).as_secs() + 1
                ));
            }
        }
    }
    for attempt in 0..2 {
        if let Some(next) = state.next_request.get(&origin) {
            tokio::time::sleep_until(*next).await;
        }
        state
            .next_request
            .insert(origin.clone(), Instant::now() + REQUEST_INTERVAL);
        let response = client.get(url.clone())
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send().await;
        let response = match response {
            Ok(response) => response,
            Err(_) if attempt == 0 => continue,
            Err(error) => return Err(format!("Search request failed after retry: {error}")),
        };
        let path = response.url().path();
        if path.contains("captcha") || path.starts_with("/_bv/") {
            let reason = "Search requires browser verification".to_string();
            state.blocked.insert(
                if path.starts_with("/_bv/") {
                    key.clone()
                } else {
                    origin.clone()
                },
                (Instant::now() + Duration::from_secs(300), reason.clone()),
            );
            return Err(reason);
        }
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
            || (response.status().is_server_error()
                && response.headers().contains_key("Retry-After"))
        {
            let delay = retry_after(
                response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok()),
            );
            let reason = format!(
                "Search unavailable (HTTP {}); retry after {} seconds",
                response.status().as_u16(),
                delay.as_secs()
            );
            let until = Instant::now()
                .checked_add(delay)
                .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400 * 365));
            state
                .blocked
                .insert(origin.clone(), (until, reason.clone()));
            return Err(reason);
        }
        if response.status().is_server_error() && attempt == 0 {
            continue;
        }
        let response = response
            .error_for_status()
            .map_err(|e| format!("Search request failed: {e}"))?;
        let html = response
            .text()
            .await
            .map_err(|e| format!("Cannot read Search response: {e}"))?;
        let hits = if let Some((_, query)) = url.query_pairs().find(|(key, _)| key == "s") {
            parse_mwmbl_json(&html, &query, usize::MAX)?
        } else {
            parse_search_html(&html, usize::MAX)?
        };
        if state.cache.len() >= 32 {
            if let Some(oldest) = state
                .cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(key, _)| key.clone())
            {
                state.cache.remove(&oldest);
            }
        }
        state.cache.insert(key, (Instant::now(), hits.clone()));
        return Ok(hits.into_iter().take(limit).collect());
    }
    unreachable!("Both request attempts return a result")
}

fn retry_after(value: Option<&str>) -> Duration {
    let seconds = value
        .and_then(|value| {
            value.parse::<u64>().ok().or_else(|| {
                chrono::DateTime::parse_from_rfc2822(value)
                    .ok()
                    .map(|at| (at.timestamp() - chrono::Utc::now().timestamp()).max(0) as u64)
            })
        })
        .unwrap_or(300);
    Duration::from_secs(seconds.max(2))
}

fn parse_mwmbl_json(body: &str, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let data: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("Invalid search JSON: {error}"))?;
    let results = data.as_array().ok_or("Invalid search result list")?;
    let terms = super::location::normalized(query);
    let mut hits = Vec::new();
    for result in results {
        let Some(raw_url) = result["url"].as_str() else {
            continue;
        };
        let Ok(mut url) = Url::parse(raw_url) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let mut text = raw_url.to_string();
        for field in ["title", "extract"] {
            if let Some(chunks) = result[field].as_array() {
                text.push(' ');
                text.extend(chunks.iter().filter_map(|chunk| chunk["value"].as_str()));
            }
        }
        let text = format!(" {} ", super::location::normalized(&text));
        // The small independent index may return partial matches; require every requested term.
        if terms.is_empty()
            || !terms
                .split_whitespace()
                .all(|term| text.contains(&format!(" {term} ")))
        {
            continue;
        }
        url.set_fragment(None);
        let url = url.to_string();
        if !hits.iter().any(|hit: &SearchHit| hit.url == url) {
            hits.push(SearchHit { url });
        }
        if hits.len() >= limit {
            break;
        }
    }
    if hits.is_empty() {
        return Err("Independent search returned no results matching all requested terms.".into());
    }
    Ok(hits)
}

fn parse_search_html(html: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    // shortcut: public search markup can change; use the supported API if this parser stops working.
    for anchor in ANCHOR_RE.captures_iter(html) {
        let attrs = &anchor[1];
        let result_class = CLASS_RE.captures(attrs).is_some_and(|class| {
            class[1]
                .split_whitespace()
                .any(|c| c == "l1" || c == "result__a")
        });
        if !result_class && !YAHOO_RESULT_RE.is_match(attrs) {
            continue;
        }
        let Some(href) = HREF_RE.captures(attrs) else {
            continue;
        };
        let mut href = unescape_html_entities(&href[1]);
        if href.starts_with("//") {
            href = format!("https:{href}");
        }
        if let Ok(redirect) = Url::parse(&href) {
            if redirect.host_str() == Some("duckduckgo.com") && redirect.path() == "/l/" {
                if let Some((_, target)) = redirect.query_pairs().find(|(k, _)| k == "uddg") {
                    href = target.into_owned();
                }
            }
        }
        if let Ok(redirect) = Url::parse(&href) {
            if redirect.host_str() == Some("r.search.yahoo.com") {
                let Some(encoded) = redirect
                    .path()
                    .split("/RU=")
                    .nth(1)
                    .and_then(|v| v.split("/RK=").next())
                else {
                    continue;
                };
                href = url::form_urlencoded::parse(format!("url={encoded}").as_bytes())
                    .next()
                    .unwrap()
                    .1
                    .into_owned();
            }
        }
        let Ok(mut url) = Url::parse(&href) else {
            continue;
        };
        if !matches!(url.scheme(), "https" | "http")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let Some(host) = url.host_str() else {
            continue;
        };
        if host == "brave.com"
            || host.ends_with(".brave.com")
            || host == "yahoo.com"
            || host.ends_with(".yahoo.com")
            || host == "duckduckgo.com"
            || host.ends_with(".duckduckgo.com")
        {
            continue;
        }
        url.set_fragment(None);
        let url = url.to_string();
        if !hits.iter().any(|h: &SearchHit| h.url == url) {
            hits.push(SearchHit { url });
        }
        if hits.len() >= limit {
            break;
        }
    }
    if hits.is_empty() {
        return Err("Search returned no readable results. The query may have no matches, or the provider may be unavailable or require browser verification.".into());
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn reads_brave_api_pages_without_duplicates() {
        let page = |urls: &[&str], more: bool| {
            serde_json::json!({
                "query": {"more_results_available": more},
                "web": {"results": urls.iter().map(|u| serde_json::json!({"url": u})).collect::<Vec<_>>()}
            })
            .to_string()
        };
        let mut hits = Vec::new();
        let full: Vec<String> = (0..20).map(|i| format!("https://s{i}.ru/")).collect();
        let full: Vec<&str> = full.iter().map(String::as_str).collect();
        assert!(parse_brave_api_json(&page(&full, true), &mut hits).unwrap());
        assert!(
            !parse_brave_api_json(
                &page(&["https://s0.ru/", "https://new.ru/"], false),
                &mut hits
            )
            .unwrap()
        );
        assert_eq!(hits.len(), 21);
        assert!(parse_brave_api_json("<html>", &mut hits).is_err());
    }

    #[test]
    fn reads_duckduckgo_redirect_results() {
        let html = r#"<a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fstudio.ru%2Fcontacts%2F&amp;rut=abc">Studio</a>
        <a class="result__a" href="https://duckduckgo.com/y.js?ad_domain=ad.ru">Ad</a>
        <a class="result__url" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fstudio.ru%2Fcontacts%2F">studio.ru</a>"#;
        let hits = parse_search_html(html, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://studio.ru/contacts/");
    }

    #[test]
    fn ignores_naver_results_after_the_provider_failed_relevance_checks() {
        let html = r#"<a class="fds-anchor-layout" nocr="1" data-heatmap-target=".link" href="https://www.servcorp.fr/en/coworking/locations/paris/kleber/#contact">Coworking Paris</a>
        <a class="fds-anchor-layout" nocr="1" data-heatmap-target=".link" href="https://www.servcorp.fr/en/coworking/locations/paris/kleber/">Duplicate</a>
        <a class="fds-anchor-layout" nocr="1" data-heatmap-target=".link" href="https://www.deskeo.com/en/?city=Paris&amp;lang=en">Deskeo</a>
        <a class="fds-anchor-layout" nocr="1" data-heatmap-target=".link" href="https://search.naver.com/">Search</a>
        <a nocr="1" data-heatmap-target=".tit" href="https://news.org/story">News</a>
        <a class="fds-anchor-layout" nocr="1" data-heatmap-target=".ad" href="https://ad.org/">Ad</a>
        <a href="https://navigation.org/">Navigation</a>"#;
        assert!(parse_search_html(html, 10).is_err());
    }

    #[test]
    fn validates_json_results_and_rejects_partial_matches() {
        let body = r#"[{"url":"https://studio.org/contact?city=Paris#mail","title":[{"value":"Cowork"},{"value":"ing Paris"}],"extract":[]},
        {"url":"https://studio.org/contact?city=Paris","title":[{"value":"Coworking Paris"}]},
        {"url":"https://jewelry.org/","title":[{"value":"Paris contact"}]},
        {"url":"javascript:alert(1)","title":[{"value":"Coworking Paris"}]},
        {"url":"https://user:password@studio.org/","title":[{"value":"Coworking Paris"}]}]"#;
        let hits = parse_mwmbl_json(body, "coworking Paris", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://studio.org/contact?city=Paris");
        assert!(parse_mwmbl_json(body, "dentist Berlin", 10).is_err());
        assert!(parse_mwmbl_json("<title>Captcha</title>", "coworking Paris", 10).is_err());
        assert!(parse_mwmbl_json("{}", "coworking Paris", 10).is_err());
        assert!(parse_mwmbl_json("[]", "coworking Paris", 10).is_err());
        assert!(
            parse_mwmbl_json(body, "coworking Paris", 0)
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn uses_json_results_when_the_primary_is_rate_limited() {
        let primary = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fallback = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let primary_url = format!("http://{}/search", primary.local_addr().unwrap());
        let fallback_url = format!("http://{}/search/", fallback.local_addr().unwrap());
        let primary_server = tokio::spawn(async move {
            let (mut socket, _) = primary.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 300\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        });
        let fallback_server = tokio::spawn(async move {
            let (mut socket, _) = fallback.accept().await.unwrap();
            let mut request = [0; 2048];
            let n = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            assert!(request.contains("GET /search/?s=coworking+Paris HTTP/1.1"));
            let body = r#"[{"url":"https://www.deskeo.com/en/","title":[{"value":"Coworking Paris"}],"extract":[]},{"url":"https://jewelry.org/contact","title":[{"value":"Jewelry Paris contact"}],"extract":[]}]"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let client = Client::builder().no_proxy().build().unwrap();
        let result = search_providers(
            &client,
            "coworking Paris contact",
            10,
            &[
                ("Primary", &primary_url, "q"),
                ("Mwmbl", &fallback_url, "s"),
            ],
        )
        .await;
        primary_server.await.unwrap();
        fallback_server.await.unwrap();
        assert!(
            result.is_ok(),
            "429 should recover through JSON search: {result:?}"
        );
        assert_eq!(result.unwrap()[0].url, "https://www.deskeo.com/en/");
    }

    #[test]
    fn reads_retry_after_seconds_dates_and_missing_header() {
        assert_eq!(retry_after(Some("120")), Duration::from_secs(120));
        assert_eq!(retry_after(Some("0")), Duration::from_secs(2));
        assert_eq!(retry_after(None), Duration::from_secs(300));
        assert_eq!(retry_after(Some("invalid")), Duration::from_secs(300));
        let date = (chrono::Utc::now() + chrono::Duration::seconds(120))
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        assert!((119..=120).contains(&retry_after(Some(&date)).as_secs()));
    }

    #[tokio::test]
    async fn pauses_rate_limited_provider_and_uses_fallback_without_repeating_requests() {
        let primary = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fallback = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let primary_url = format!("http://{}/search", primary.local_addr().unwrap());
        let fallback_url = format!("http://{}/search", fallback.local_addr().unwrap());
        let primary_server = tokio::spawn(async move {
            let (mut socket, _) = primary.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            primary
        });
        let fallback_server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = fallback.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body =
                    r#"<a data-matarget="algo" href="https://clinic.org/contact">Clinic</a>"#;
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        let providers = [
            ("Primary", primary_url.as_str(), "q"),
            ("Fallback", fallback_url.as_str(), "p"),
        ];
        for query in ["Berlin dentist", "Berlin dentist email"] {
            assert_eq!(
                search_providers(&client, query, 10, &providers)
                    .await
                    .unwrap()[0]
                    .url,
                "https://clinic.org/contact"
            );
        }
        let primary = primary_server.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(50), primary.accept())
                .await
                .is_err(),
            "Rate limited provider was requested again during its cooldown"
        );
        fallback_server.await.unwrap();
    }

    #[tokio::test]
    async fn identifies_yahoo_verification_instead_of_retrying_its_500() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/search", listener.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move {
            for response in [
                "HTTP/1.1 302 Found\r\nLocation: /_bv/v.gif\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 500 INKApi Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let result = search_provider(
            &Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
            url,
            10,
        )
        .await;
        server.await.unwrap();
        assert!(result.unwrap_err().contains("browser verification"));
    }

    #[tokio::test]
    async fn verification_for_one_query_does_not_block_another_query() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!(
            "http://{}/search?q=first",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            let responses = [
                "HTTP/1.1 302 Found\r\nLocation: /_bv/v.gif\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 500 INKApi Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ];
            for response in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            socket.read(&mut request).await.unwrap();
            let body = r#"<a class="l1" href="https://clinic.org/contact">Clinic</a>"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let client = Client::builder().no_proxy().build().unwrap();
        assert!(search_provider(&client, url.clone(), 10).await.is_err());
        let mut alternative = url;
        alternative.set_query(Some("q=second"));
        let result = search_provider(&client, alternative, 10).await;
        server.abort();
        assert!(
            result.is_ok(),
            "A query-specific verification blocked an alternative query: {result:?}"
        );
    }

    #[tokio::test]
    async fn recovers_from_transient_server_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!(
            "http://{}/search?q=Berlin",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            for status in ["500 Internal Server Error", "200 OK"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body = r#"<a class="l1" href="https://clinic.org/contact">Clinic Berlin</a><a class="l1" href="https://studio.org/contact">Studio</a>"#;
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let client = Client::builder().no_proxy().build().unwrap();
        let result = search_provider(&client, url.clone(), 1).await;
        server.abort();
        assert!(
            result.is_ok(),
            "A temporary 500 must be retried: {result:?}"
        );
        assert_eq!(
            search_provider(&client, url, 10).await.unwrap().len(),
            2,
            "Cache must preserve results beyond the first caller's limit"
        );
    }

    #[test]
    fn parses_only_organic_links_preserving_query_and_unicode() {
        let html = r#"<a href="https://brave.com/">Help</a>
            <a class="thumbnail" href="https://images.example.org/photo.png">Image</a>
            <a href='https://clinic.org/contact?city=São%20Paulo&amp;lang=en#mail' class='hash l1'><div>Clinic</div></a>
            <a class="l1 hash" href="https://clinic.org/contact?city=São%20Paulo&amp;lang=en">Duplicate</a>
            <a class="l1" href="javascript:alert(1)">Invalid</a>
            <a class="l1" href="https://search.brave.com/">Provider</a>
            <a class="l1" href="https://user:pass@clinic.org/">Credentials</a>
            <a class="l1" href="https://studio.org/contacts">Studio</a>"#;
        let hits = parse_search_html(html, 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].url,
            "https://clinic.org/contact?city=S%C3%A3o%20Paulo&lang=en"
        );
        assert_eq!(hits[1].url, "https://studio.org/contacts");
        assert_eq!(parse_search_html(html, 1).unwrap().len(), 1);
        assert!(parse_search_html(html, 0).unwrap().is_empty());
    }

    #[test]
    fn refuses_unrecognized_or_challenge_pages_without_false_success() {
        assert!(parse_search_html("<title>Captcha</title>", 10).is_err());
        assert!(parse_search_html(r#"<a href="https://clinic.org/">Navigation</a>"#, 10).is_err());
        assert!(
            parse_search_html(
                r#"<script>captcha</script><a class="l1" href="https://clinic.org/">Result</a>"#,
                10
            )
            .is_ok()
        );
    }

    #[test]
    fn decodes_yahoo_organic_redirects_without_tracking_links() {
        let html = r#"<a href="https://r.search.yahoo.com/foo/RU=https%3a%2f%2fclinic.org%2fcontact%3fcity%3dBerlin%26lang%3dde/RK=2/RS=token" data-matarget="algo">Clinic</a>
        <a data-matarget="algo" href="https://studio.org/contacts">Studio</a>
        <a href="https://r.search.yahoo.com/foo/RU=https%3a%2f%2fads.org%2f/RK=2/">Advertisement</a>"#;
        let hits = parse_search_html(html, 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].url,
            "https://clinic.org/contact?city=Berlin&lang=de"
        );
        assert_eq!(hits[1].url, "https://studio.org/contacts");
    }
}

pub async fn dork_ats_vacancies(
    client: &Client,
    persona: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let queries = if persona == "creative" {
        vec![
            "site:boards.greenhouse.io \"Product Designer\"",
            "site:boards.greenhouse.io \"Art Director\"",
            "site:jobs.ashbyhq.com \"Product Designer\"",
            "site:jobs.ashbyhq.com \"Brand Designer\"",
            "site:jobs.lever.co \"Product Designer\"",
            "site:apply.workable.com \"Designer\"",
        ]
    } else {
        vec![
            "site:boards.greenhouse.io \"Security Engineer\"",
            "site:boards.greenhouse.io \"Security\"",
            "site:jobs.ashbyhq.com \"Security\"",
            "site:jobs.lever.co \"Security\"",
            "site:jobs.lever.co \"DevSecOps\"",
            "site:apply.workable.com \"Security\"",
        ]
    };

    let mut discovered_urls = Vec::new();
    let per_query_limit = (limit.max(10)).min(30);
    for q in queries {
        match search_web(client, q, per_query_limit).await {
            Ok(hits) => {
                for h in hits {
                    if (h.url.contains("greenhouse.io")
                        || h.url.contains("ashbyhq.com")
                        || h.url.contains("lever.co")
                        || h.url.contains("smartrecruiters.com")
                        || h.url.contains("workable.com"))
                        && !discovered_urls.contains(&h.url)
                    {
                        discovered_urls.push(h.url);
                    }
                }
            }
            Err(error) if discovered_urls.is_empty() => return Err(error),
            Err(_) => break,
        }
        if discovered_urls.len() >= limit {
            break;
        }
    }
    discovered_urls.truncate(limit);
    Ok(discovered_urls)
}
