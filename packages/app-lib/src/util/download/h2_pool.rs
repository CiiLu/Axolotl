//! Shared per-authority HTTP/2 connections for file downloads.
//!
//! `reqwest`'s connection pool opens a fresh TCP+TLS connection for every
//! request that arrives while no idle connection is available, so a batch of
//! concurrent downloads to one CDN costs one handshake per file. This module
//! instead maintains a single long-lived HTTP/2 connection per authority and
//! multiplexes every download as a separate stream over it (`SendRequest` is
//! cheap to clone and each clone opens an independent stream). Handshakes
//! happen once per authority, and large files can also split into range
//! streams over the same connection.

use bytes::Bytes;
use h2::client::SendRequest;
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// A live shared HTTP/2 connection to one authority.
pub struct SharedH2Connection {
    sender: Mutex<SendRequest<Bytes>>,
    /// Set to true by the driver task when the connection terminates.
    dead: Arc<std::sync::atomic::AtomicBool>,
}

impl SharedH2Connection {
    fn new(sender: SendRequest<Bytes>) -> Self {
        Self {
            sender: Mutex::new(sender),
            dead: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Sends a request on the shared connection and awaits the response
    /// headers, yielding the response and its receive stream. Each call
    /// opens an independent multiplexed stream.
    pub async fn open(
        &self,
        request: http::Request<()>,
    ) -> Result<http::Response<h2::RecvStream>, h2::Error> {
        let sender = self.sender.lock().unwrap().clone();
        let mut sender = sender.ready().await?;
        let (response, send_stream) = sender.send_request(request, false)?;
        drop(send_stream);
        response.await
    }
}

/// Registry of live shared connections, keyed by authority.
static CONNECTIONS: std::sync::LazyLock<
    Mutex<HashMap<String, Weak<SharedH2Connection>>>,
> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn tls_config() -> Arc<ClientConfig> {
    static CONFIG: std::sync::LazyLock<Mutex<Option<Arc<ClientConfig>>>> =
        std::sync::LazyLock::new(|| Mutex::new(None));
    let mut guard = CONFIG.lock().unwrap();
    if let Some(config) = guard.as_ref() {
        return Arc::clone(config);
    }
    let mut config = ClientConfig::builder()
        .with_root_certificates(platform_root_certs())
        .with_no_client_auth();
    config.enable_early_data = true;
    config.alpn_protocols = vec![b"h2".to_vec()];
    let config = Arc::new(config);
    *guard = Some(Arc::clone(&config));
    config
}

fn platform_root_certs() -> rustls::RootCertStore {
    let mut store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs();
    for cert in certs.certs {
        let _ = store.add(cert);
    }
    if certs.errors.is_empty() {
        return store;
    }
    store.extend(
        webpki_roots::TLS_SERVER_ROOTS
            .iter()
            .cloned(),
    );
    store
}

async fn connect_tcp(host: &str, port: u16) -> std::io::Result<TcpStream> {
    // Prefer the ordered address list from the shared download resolver
    // (IPv4/IPv6 preference and per-IP reliability), falling back to the
    // system resolver when no list is cached yet.
    let addresses = crate::util::fetch::DOWNLOAD_DNS_RESOLVER
        .resolved_addresses(host);
    let mut last_error = None;
    if !addresses.is_empty() {
        for address in addresses {
            match tokio::time::timeout(
                CONNECT_TIMEOUT,
                TcpStream::connect((address, port)),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    stream.set_nodelay(true).ok();
                    return Ok(stream);
                }
                Ok(Err(error)) => last_error = Some(error),
                Err(_) => {
                    last_error = Some(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("connection to {host}:{port} timed out"),
                    ));
                }
            }
        }
    }
    let stream = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio::net::TcpStream::connect((host, port)),
    )
    .await
    .map_err(|_| {
        last_error.take().unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("connection to {host}:{port} timed out"),
            )
        })
    })?
    .map_err(|error| {
        last_error.take().unwrap_or_else(|| {
            std::io::Error::new(
                error.kind(),
                format!("connection to {host}:{port} failed: {error}"),
            )
        })
    })?;
    stream.set_nodelay(true).ok();
    Ok(stream)
}

/// Connects a new shared HTTP/2 connection to `authority` (host[:port]).
async fn establish(authority: &str) -> crate::Result<Arc<SharedH2Connection>> {
    let (host, port) = authority
        .rsplit_once(':')
        .map(|(host, port)| (host, port.parse::<u16>().unwrap_or(443)))
        .unwrap_or((authority, 443));

    // Pre-resolve so `connect_tcp` gets the ordered, reliability-ranked
    // address list shared with the legacy reqwest path.
    crate::util::fetch::DOWNLOAD_DNS_RESOLVER.pre_resolve(host).await;

    let tcp = connect_tcp(host, port).await.map_err(|error| {
        crate::ErrorKind::NetworkError(format!(
            "failed to establish shared HTTP/2 connection to {authority}: {error}"
        ))
    })?;

    let server_name =
        ServerName::try_from(host.to_string()).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "invalid server name for {host}: {error}"
            ))
        })?;
    let connector = TlsConnector::from(tls_config());
    let tls = tokio::time::timeout(
        TLS_HANDSHAKE_TIMEOUT,
        connector.connect(server_name, tcp),
    )
    .await
    .map_err(|_| {
        crate::ErrorKind::NetworkError(format!(
            "TLS handshake with {authority} timed out"
        ))
    })?
    .map_err(|error| {
        crate::ErrorKind::NetworkError(format!(
            "TLS handshake with {authority} failed: {error}"
        ))
    })?;

    let (sender, connection) =
        h2::client::handshake(Box::pin(tls)).await.map_err(|error| {
            crate::ErrorKind::NetworkError(format!(
                "HTTP/2 handshake with {authority} failed: {error}"
            ))
        })?;

    let shared = Arc::new(SharedH2Connection::new(sender));
    {
        let mut registry = CONNECTIONS.lock().unwrap();
        registry.insert(authority.to_string(), Arc::downgrade(&shared));
    }

    let dead = Arc::clone(&shared.dead);
    let authority = authority.to_string();
    tokio::spawn(async move {
        let _ = connection.await;
        dead.store(true, std::sync::atomic::Ordering::Release);
        tracing::debug!(authority, "Shared HTTP/2 connection closed");
    });

    Ok(shared)
}

/// Returns the live shared connection for `authority`, establishing one on
/// first use or after a previous connection died.
pub async fn shared_connection(
    authority: &str,
) -> crate::Result<Arc<SharedH2Connection>> {
    let cached = CONNECTIONS
        .lock()
        .unwrap()
        .get(authority)
        .and_then(Weak::upgrade)
        .filter(|connection| !connection.is_dead());
    if let Some(connection) = cached {
        return Ok(connection);
    }
    CONNECTIONS.lock().unwrap().remove(authority);
    establish(authority).await
}

/// Drops all cached connections (used by tests).
#[cfg(test)]
pub fn reset_for_tests() {
    CONNECTIONS.lock().unwrap().clear();
}
