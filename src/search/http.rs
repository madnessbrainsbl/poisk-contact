use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub(super) async fn get_json(
    client: &Client,
    url: Url,
    timeout: Duration,
) -> Result<Value, reqwest::Error> {
    for attempt in 0..2 {
        let (result, retry_after) = match client
            .get(url.clone())
            .header("User-Agent", "PoiskContact/0.1 (desktop contact search)")
            .timeout(timeout)
            .send()
            .await
        {
            Ok(response) => {
                let retry_after = response.headers().contains_key("Retry-After");
                let result = match response.error_for_status() {
                    Ok(response) => response.json::<Value>().await,
                    Err(error) => Err(error),
                };
                (result, retry_after)
            }
            Err(error) => (Err(error), false),
        };
        match result {
            Err(error)
                if attempt == 0
                    && !retry_after
                    && (error.is_timeout()
                        || error.is_connect()
                        || error
                            .status()
                            .is_some_and(|status| status.is_server_error())) =>
            {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            result => return result,
        }
    }
    unreachable!("Second attempt returns its result")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn retries_a_temporary_json_service_failure() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move {
            for status in ["503 Service Unavailable", "200 OK"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                let body = r#"{"features":[{"name":"Berlin"}]}"#;
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let result = tokio::time::timeout(
            Duration::from_secs(4),
            get_json(
                &Client::builder().no_proxy().build().unwrap(),
                url,
                Duration::from_secs(1),
            ),
        )
        .await;
        server.abort();
        assert_eq!(result.unwrap().unwrap()["features"][0]["name"], "Berlin");
    }

    #[tokio::test]
    async fn does_not_retry_an_explicit_rate_limit_or_invalid_json() {
        for (status, header, body) in [
            ("429 Too Many Requests", "Retry-After: 120\r\n", "{}"),
            ("503 Service Unavailable", "Retry-After: 120\r\n", "{}"),
            ("200 OK", "", "invalid JSON"),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).await.unwrap();
                socket.write_all(format!("HTTP/1.1 {status}\r\n{header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                assert!(
                    tokio::time::timeout(Duration::from_millis(700), listener.accept())
                        .await
                        .is_err(),
                    "Unexpected second request"
                );
            });
            let result = get_json(
                &Client::builder().no_proxy().build().unwrap(),
                url,
                Duration::from_secs(1),
            )
            .await;
            assert!(result.is_err());
            server.await.unwrap();
        }
    }
}
