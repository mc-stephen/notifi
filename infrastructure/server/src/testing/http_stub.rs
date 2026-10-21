//! One-shot local HTTP stub for provider-controller unit tests: capture
//! the raw request bytes a controller sends and answer with a canned
//! status + body. Zero new dependencies — a `TcpListener` is the whole
//! server.
//!
//! Spawns on an ephemeral port, accepts ONE connection (the provider's
//! request), records it, replies, closes. Providers under test point
//! their `api_base`/endpoint at `http://127.0.0.1:{port}` (see the
//! `with_base` test constructors in each controller).

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Handle to one stubbed exchange.
pub struct Stub {
    pub port: u16,
    token: Arc<Mutex<Option<String>>>,
    handle: tokio::task::JoinHandle<()>,
}

impl Stub {
    /// The captured request (status line + headers + body) once answered.
    pub async fn request(self) -> Option<String> {
        let token = self.token;
        let _ = self.handle.await;
        token.lock().ok()?.take()
    }
}

/// Spawn a stub answering the first request with `status` then `body`.
pub async fn start(status: u16, reason: &str, body: &str) -> Stub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral stub");
    let port = listener.local_addr().expect("local addr").port();
    let token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let holder = token.clone();
    let reason = reason.to_string();
    let body = body.to_string();
    let handle = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut raw: Vec<u8> = Vec::new();
        let mut chunk = vec![0u8; 8192];
        loop {
            match socket.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    raw.extend_from_slice(&chunk[..n]);
                    match complete_after_headers(&raw) {
                        Some(total) if raw.len() >= total => break,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
        *holder.lock().expect("stub lock") = Some(String::from_utf8_lossy(&raw).to_string());
        let reply = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(reply.as_bytes()).await;
        let _ = socket.flush().await;
    });
    Stub {
        port,
        token,
        handle,
    }
}

/// Header end + declared body size once present: the total bytes a
/// complete request needs, `None` while headers are still arriving.
fn complete_after_headers(raw: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(raw);
    let idx = text.find("\r\n\r\n")?;
    let headers = &text[..idx];
    let body_start = idx + 4;
    let len: usize = headers
        .lines()
        .find_map(|l| {
            let mut parts = l.split(':');
            let key = parts.next()?.trim().to_ascii_lowercase();
            let value = parts.next()?.trim().to_string();
            if key == "content-length" {
                value.parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    Some(body_start + len)
}
