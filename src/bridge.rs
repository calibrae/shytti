use std::io::{Read, Write};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::error::Error;
use crate::shell::ShellManager;

pub struct HermyttBridge {
    base_url: String,
    auth_key: String,
}

impl HermyttBridge {
    pub fn new(hermytt_url: &str, auth_key: &str) -> Self {
        Self {
            base_url: hermytt_url.trim_end_matches('/').to_string(),
            auth_key: auth_key.to_string(),
        }
    }

    /// Returns true if the bridge has a real (non-localhost) hermytt URL configured.
    pub fn is_configured(&self) -> bool {
        !self.base_url.contains("localhost") && !self.auth_key.is_empty()
    }

    /// Register a managed session with Hermytt, open WS pipe, bridge PTY I/O.
    pub async fn attach(
        &self,
        shell_id: &str,
        manager: &ShellManager,
    ) -> Result<String, Error> {
        let session_id = self.register_session(Some(shell_id)).await?;

        let pty_reader = manager.get_reader(shell_id).await?;
        let pty_writer = manager.get_writer(shell_id).await?;

        let ws_url = self.ws_url(&session_id);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| Error::Bridge(format!("ws connect: {e}")))?;

        let (mut ws_sink, mut ws_source) = ws_stream.split();

        // Auth handshake
        ws_sink.send(Message::Text(self.auth_key.clone().into())).await
            .map_err(|e| Error::Bridge(format!("ws auth send: {e}")))?;

        match ws_source.next().await {
            Some(Ok(msg)) => {
                let text = msg.into_text().unwrap_or_default();
                if text != "auth:ok" {
                    return Err(Error::Bridge(format!("auth rejected: {text}")));
                }
            }
            other => return Err(Error::Bridge(format!("auth failed: {other:?}"))),
        }

        // PTY stdout → WS
        tokio::spawn(async move {
            let mut reader = pty_reader;
            loop {
                let mut r = reader;
                let (r_back, result): (Box<dyn Read + Send>, std::io::Result<Vec<u8>>) =
                    tokio::task::spawn_blocking(move || {
                        let mut buf = [0u8; 4096];
                        let n = r.read(&mut buf);
                        (r, n.map(|n| buf[..n].to_vec()))
                    }).await.unwrap_or_else(|_| panic!("blocking read panicked"));

                reader = r_back;
                match result {
                    Ok(ref data) if data.is_empty() => break,
                    Ok(data) => {
                        if ws_sink.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = ws_sink.close().await;
        });

        // WS recv → PTY stdin (filtering resize control frames)
        let mgr = manager.clone();
        let shell_id_owned = shell_id.to_string();
        tokio::spawn(async move {
            let mut writer = pty_writer;
            while let Some(Ok(msg)) = ws_source.next().await {
                let raw = match msg {
                    Message::Binary(data) => data.to_vec(),
                    Message::Text(text) => text.as_bytes().to_vec(),
                    Message::Close(_) => break,
                    _ => continue,
                };

                // Check for resize control frame
                if let Ok(text) = std::str::from_utf8(&raw) {
                    if let Some((cols, rows)) = parse_resize(text) {
                        let _ = mgr.resize(&shell_id_owned, rows, cols).await;
                        continue;
                    }
                }

                let mut w = writer;
                let (w_back, result): (Box<dyn std::io::Write + Send>, std::io::Result<()>) =
                    tokio::task::spawn_blocking(move || {
                        let r = w.write_all(&raw);
                        (w, r)
                    }).await.unwrap_or_else(|_| panic!("blocking write panicked"));

                writer = w_back;
                if result.is_err() { break; }
            }
        });

        tracing::info!(shell_id, session_id = %session_id, "bridged to hermytt");
        Ok(session_id)
    }

    pub async fn detach(&self, session_id: &str) -> Result<(), Error> {
        self.unregister_session(session_id).await
    }

    async fn register_session(&self, id: Option<&str>) -> Result<String, Error> {
        let url = format!("{}/internal/session", self.base_url);
        let body = match id {
            Some(id) => serde_json::json!({"id": id}).to_string(),
            None => "{}".to_string(),
        };

        let resp = http_post(&url, &self.auth_key, &body).await?;
        let v: serde_json::Value = serde_json::from_str(&resp)
            .map_err(|e| Error::Bridge(format!("register parse: {e}")))?;
        v["id"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Bridge("no id in register response".into()))
    }

    async fn unregister_session(&self, session_id: &str) -> Result<(), Error> {
        let url = format!("{}/internal/session/{session_id}", self.base_url);
        http_delete(&url, &self.auth_key).await?;
        tracing::info!(session_id, "unregistered from hermytt");
        Ok(())
    }

    fn ws_url(&self, session_id: &str) -> String {
        let ws_base = self.base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        format!("{ws_base}/internal/session/{session_id}/pipe")
    }
}

pub(crate) fn parse_resize(text: &str) -> Option<(u16, u16)> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let arr = v.get("resize")?.as_array()?;
    let cols = arr.first()?.as_u64()? as u16;
    let rows = arr.get(1)?.as_u64()? as u16;
    Some((cols, rows))
}

async fn http_post(url: &str, auth_key: &str, body: &str) -> Result<String, Error> {
    let (host, path) = parse_url(url)?;
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nX-Hermytt-Key: {auth_key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    raw_http(&host, &req).await
}

async fn http_delete(url: &str, auth_key: &str) -> Result<String, Error> {
    let (host, path) = parse_url(url)?;
    let req = format!(
        "DELETE {path} HTTP/1.1\r\nHost: {host}\r\nX-Hermytt-Key: {auth_key}\r\nConnection: close\r\n\r\n"
    );
    raw_http(&host, &req).await
}

pub(crate) fn parse_url(url: &str) -> Result<(String, String), Error> {
    let stripped = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| Error::Bridge("bad url".into()))?;
    let (host, path) = stripped.split_once('/').unwrap_or((stripped, ""));
    Ok((host.to_string(), format!("/{path}")))
}

pub fn gethostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

async fn raw_http(host: &str, req: &str) -> Result<String, Error> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let mut stream = TcpStream::connect(host).await
        .map_err(|e| Error::Bridge(format!("connect {host}: {e}")))?;
    stream.write_all(req.as_bytes()).await
        .map_err(|e| Error::Bridge(format!("write: {e}")))?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp).await
        .map_err(|e| Error::Bridge(format!("read: {e}")))?;

    resp.split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .ok_or_else(|| Error::Bridge("malformed response".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resize_valid_80x24() {
        assert_eq!(parse_resize(r#"{"resize":[80,24]}"#), Some((80, 24)));
    }

    #[test]
    fn parse_resize_valid_120x40() {
        assert_eq!(parse_resize(r#"{"resize":[120,40]}"#), Some((120, 40)));
    }

    #[test]
    fn parse_resize_other_key() {
        assert_eq!(parse_resize(r#"{"other":"data"}"#), None);
    }

    #[test]
    fn parse_resize_not_json() {
        assert_eq!(parse_resize("not json"), None);
    }

    #[test]
    fn parse_resize_not_array() {
        assert_eq!(parse_resize(r#"{"resize":"notarray"}"#), None);
    }

    #[test]
    fn parse_resize_empty_object() {
        assert_eq!(parse_resize("{}"), None);
    }

    #[test]
    fn parse_url_http_with_path() {
        let (host, path) = parse_url("http://localhost:7777/foo/bar").unwrap();
        assert_eq!(host, "localhost:7777");
        assert_eq!(path, "/foo/bar");
    }

    #[test]
    fn parse_url_https_with_path() {
        let (host, path) = parse_url("https://host:443/path").unwrap();
        assert_eq!(host, "host:443");
        assert_eq!(path, "/path");
    }

    #[test]
    fn parse_url_bad_scheme() {
        assert!(parse_url("ftp://bad").is_err());
    }

    #[test]
    fn parse_url_no_path() {
        let (host, path) = parse_url("http://host").unwrap();
        assert_eq!(host, "host");
        assert_eq!(path, "/");
    }

    #[test]
    fn bridge_new_trims_trailing_slash() {
        let b = HermyttBridge::new("http://localhost:7777/", "key");
        assert_eq!(b.base_url, "http://localhost:7777");
    }

    #[test]
    fn bridge_ws_url_http_to_ws() {
        let b = HermyttBridge::new("http://localhost:7777", "key");
        let url = b.ws_url("sess-1");
        assert_eq!(url, "ws://localhost:7777/internal/session/sess-1/pipe");
    }

    #[test]
    fn bridge_ws_url_https_to_wss() {
        let b = HermyttBridge::new("https://secure.host:443", "key");
        let url = b.ws_url("sess-2");
        assert_eq!(url, "wss://secure.host:443/internal/session/sess-2/pipe");
    }
}
