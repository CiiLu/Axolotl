//! HTTP/2 multiplexed file downloads over shared per-authority connections.
//!
//! Every download to the same authority reuses one long-lived HTTP/2
//! connection: small files are single streams, files at or above
//! [`SEGMENTED_DOWNLOAD_THRESHOLD`] split into concurrent range streams on
//! that same connection. Any protocol or server failure falls back to the
//! legacy HTTP/1.1 single-stream path, so downloads remain correct even when
//! a server or network does not support HTTP/2 or range requests.

use super::h2_pool::SharedH2Connection;
use crate::util::fetch;
use crate::util::fetch::{DownloadRequest, DownloadResult, Integrity};
use bytes::Bytes;
use http::header::{ACCEPT_ENCODING, RANGE, USER_AGENT};
use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const SEGMENTED_DOWNLOAD_THRESHOLD: u64 = 4 * 1024 * 1024;
const MIN_SEGMENT_SIZE: u64 = 256 * 1024;
const MAX_SEGMENT_CONCURRENCY: usize = 8;
const INITIAL_SEGMENT_CONCURRENCY: usize = 4;
const RANGE_IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_RECV_TIMEOUT: Duration = Duration::from_secs(30);

/// Outcome of attempting a multiplexed download.
pub(crate) enum H2DownloadOutcome {
    /// The download completed through the multiplexed path.
    Completed(DownloadResult),
    /// The multiplexed path cannot be used; the caller should fall back to
    /// the legacy path.
    Fallback { reason: &'static str },
}

/// Attempts to download `request` to `destination` over a shared HTTP/2
/// connection, multiplexing range streams for large files.
pub(crate) async fn try_download_via_h2(
    request: &DownloadRequest,
    destination: &Path,
    part_path: &Path,
) -> H2DownloadOutcome {
    let Some(connection) = connect_authority(&request.url).await else {
        return H2DownloadOutcome::Fallback {
            reason: "no shared HTTP/2 connection",
        };
    };
    let Ok(uri) = request.url.parse::<Uri>() else {
        return H2DownloadOutcome::Fallback {
            reason: "unparsable URL",
        };
    };

    let integrity = request.integrity.clone();
    let expected_size = integrity.size;

    fetch::record_install_download_started(
        request,
        &fetch::DownloadRoute {
            url: request.url.clone(),
            source: fetch::DownloadRouteSource::Official,
            is_mirror: false,
            allow_sensitive_headers: false,
            supports_range: true,
            proxy: fetch::ProxyPolicy::System,
        },
        0,
        1,
    )
    .await;

    // When the size is known (Modrinth metadata provides it) skip the probe
    // entirely: small files fetch the body directly, large files split into
    // range streams right away. The probe is only used when the size must be
    // discovered from the server.
    let total_size = if let Some(size) = expected_size {
        size
    } else {
        let mut probe_headers = request_headers(request);
        probe_headers.insert(RANGE, HeaderValue::from_static("bytes=0-0"));
        probe_headers
            .insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

        let (response, mut probe_body) =
            match open_stream(&connection, &uri, probe_headers).await {
                Ok(pair) => pair,
                Err(error) => {
                    tracing::debug!(
                        url = %fetch::sanitize_url_for_log(&request.url),
                        error = %error,
                        "HTTP/2 probe failed; falling back to legacy download"
                    );
                    return H2DownloadOutcome::Fallback {
                        reason: "probe failed",
                    };
                }
            };

        let status = response.status();
        let headers = response.headers();
        let total_size = parse_content_range_total(headers)
            .or_else(|| parse_content_length(headers));
        // Drain the probe body so the stream slot is released.
        drain_body(&mut probe_body).await;
        drop(probe_body);

        let Some(total_size) = total_size else {
            return H2DownloadOutcome::Fallback {
                reason: "unknown content size",
            };
        };
        if total_size == 0 {
            return H2DownloadOutcome::Fallback {
                reason: "empty content",
            };
        }
        if status != StatusCode::PARTIAL_CONTENT {
            return H2DownloadOutcome::Fallback {
                reason: "range requests unsupported",
            };
        }
        total_size
    };

    let segmented = total_size >= SEGMENTED_DOWNLOAD_THRESHOLD;
    let result = if segmented {
        multiplexed_ranges(
            Arc::clone(&connection),
            &uri,
            request,
            destination,
            part_path,
            &integrity,
            total_size,
        )
        .await
    } else {
        single_stream(
            &connection,
            &uri,
            request,
            destination,
            part_path,
            &integrity,
            total_size,
        )
        .await
    };
    match result {
        Ok(result) => H2DownloadOutcome::Completed(result),
        Err(error) => {
            tracing::debug!(
                url = %fetch::sanitize_url_for_log(&request.url),
                error = %error,
                "Multiplexed download failed; falling back to legacy download"
            );
            H2DownloadOutcome::Fallback {
                reason: "multiplexed download failed",
            }
        }
    }
}

async fn connect_authority(url: &str) -> Option<Arc<SharedH2Connection>> {
    let authority = fetch::url_authority(url)?;
    match super::h2_pool::shared_connection(&authority).await {
        Ok(connection) => Some(connection),
        Err(error) => {
            tracing::debug!(
                authority,
                error = %error,
                "Failed to establish shared HTTP/2 connection"
            );
            None
        }
    }
}

fn request_headers(request: &DownloadRequest) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&crate::launcher_user_agent())
            .unwrap_or_else(|_| HeaderValue::from_static("Axolotl Launcher")),
    );
    if let Some((name, value)) = &request.header {
        if let Ok(name) = http::header::HeaderName::from_str(name) {
            if let Ok(value) = HeaderValue::from_str(value) {
                headers.insert(name, value);
            }
        }
    }
    if let Some(download_meta) = &request.download_meta {
        if let Ok(value) =
            HeaderValue::from_str(&download_meta.to_header_value())
        {
            headers.insert("modrinth-download-meta", value);
        }
    }
    headers
}

fn parse_content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

fn parse_content_range_total(headers: &HeaderMap) -> Option<u64> {
    let value = headers.get(http::header::CONTENT_RANGE)?.to_str().ok()?;
    let (_, total) = value.split_once('/')?;
    if total == "*" {
        return None;
    }
    total.parse().ok()
}

type StreamPair = (http::Response<()>, h2::RecvStream);

async fn open_stream(
    connection: &SharedH2Connection,
    uri: &Uri,
    headers: HeaderMap,
) -> crate::Result<StreamPair> {
    let mut request = http::Request::builder()
        .method(Method::GET)
        .uri(uri.clone())
        .version(http::Version::HTTP_2)
        .body(())
        .unwrap();
    *request.headers_mut() = headers;
    let response = connection.open(request).await.map_err(|error| {
        crate::ErrorKind::NetworkError(format!("HTTP/2 stream error: {error}"))
    })?;
    let (parts, body) = response.into_parts();
    let response = http::Response::from_parts(parts, ());
    Ok((response, body))
}

async fn drain_body(stream: &mut h2::RecvStream) {
    while let Ok(Some(Ok(_))) =
        tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data()).await
    {}
}

/// Downloads a single-stream body to `part_path`, hashing as it streams,
/// then verifies and finalises.
async fn single_stream(
    connection: &SharedH2Connection,
    uri: &Uri,
    request: &DownloadRequest,
    destination: &Path,
    part_path: &Path,
    integrity: &Integrity,
    total_size: u64,
) -> crate::Result<DownloadResult> {
    let mut headers = request_headers(request);
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

    let (response, mut stream) = open_stream(connection, uri, headers).await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP/2 GET failed with status {}",
            response.status()
        ))
        .into());
    }

    let mut hashers = fetch::IntegrityHashers::new_integrity_hashers(integrity);
    let mut file = tokio::fs::File::create(part_path).await?;
    let mut downloaded = 0_u64;
    loop {
        let chunk = tokio::time::timeout(STREAM_RECV_TIMEOUT, stream.data())
            .await
            .map_err(|_| {
                crate::ErrorKind::NetworkError(
                    "HTTP/2 stream receive timed out".into(),
                )
            })?
            .transpose()
            .map_err(|error| {
                crate::ErrorKind::NetworkError(format!(
                    "HTTP/2 stream error: {error}"
                ))
            })?;
        let Some(chunk) = chunk else {
            break;
        };
        file.write_all(&chunk).await?;
        hashers.update(&chunk);
        downloaded += chunk.len() as u64;
        record_install_progress(request, downloaded, total_size).await;
    }
    file.flush().await?;
    drop(file);
    let computed = hashers.finish(downloaded);
    record_install_stage(request).await;

    verify_and_finalize(
        part_path,
        destination,
        integrity,
        computed,
        downloaded,
        total_size,
    )
    .await?;

    Ok(DownloadResult {
        path: destination.to_path_buf(),
        url: uri.to_string(),
        source: fetch::DownloadRouteSource::Official,
        size: downloaded,
        attempts: 0,
        fallback_count: 0,
    })
}

/// Downloads a file by multiplexing concurrent range streams over the shared
/// connection, writing each segment to a sibling file, then merging.
async fn multiplexed_ranges(
    connection: Arc<SharedH2Connection>,
    uri: &Uri,
    request: &DownloadRequest,
    destination: &Path,
    part_path: &Path,
    integrity: &Integrity,
    total_size: u64,
) -> crate::Result<DownloadResult> {
    let range_count = initial_segment_count(total_size);
    let mut segments = Vec::with_capacity(range_count);
    for index in 0..range_count {
        let start = (total_size * index as u64) / range_count as u64;
        let end = if index + 1 == range_count {
            total_size.saturating_sub(1)
        } else {
            (total_size * (index + 1) as u64) / range_count as u64 - 1
        };
        if start > end {
            continue;
        }
        segments.push((start, end));
    }

    let mut handles = Vec::new();
    for (index, (start, end)) in segments.into_iter().enumerate() {
        let connection = connection.clone();
        let uri = uri.clone();
        let headers = request_headers(request);
        let segment_path = segment_path(part_path, index);
        handles.push(tokio::spawn(async move {
            let result = download_segment(
                &connection,
                &uri,
                headers,
                start,
                end,
                &segment_path,
            )
            .await;
            if result.is_err() {
                let _ = tokio::fs::remove_file(&segment_path).await;
            }
            result
        }));
    }

    let mut hashers = fetch::IntegrityHashers::new_integrity_hashers(integrity);
    let mut file = tokio::fs::File::create(part_path).await?;
    let mut downloaded = 0_u64;
    for (index, handle) in handles.into_iter().enumerate() {
        let bytes = handle
            .await
            .map_err(|error| {
                crate::ErrorKind::OtherError(format!(
                    "segment {index} task failed: {error}"
                ))
            })?
            .map_err(|error| {
                crate::ErrorKind::OtherError(format!(
                    "segment {index} failed: {error}"
                ))
            })?;
        hashers.update(&bytes);
        file.write_all(&bytes).await?;
        downloaded += bytes.len() as u64;
        let _ = tokio::fs::remove_file(segment_path(part_path, index)).await;
        record_install_progress(request, downloaded, total_size).await;
    }
    file.flush().await?;
    drop(file);
    let computed = hashers.finish(downloaded);
    record_install_stage(request).await;

    verify_and_finalize(
        part_path,
        destination,
        integrity,
        computed,
        downloaded,
        total_size,
    )
    .await?;

    Ok(DownloadResult {
        path: destination.to_path_buf(),
        url: uri.to_string(),
        source: fetch::DownloadRouteSource::Official,
        size: downloaded,
        attempts: 0,
        fallback_count: 0,
    })
}

fn initial_segment_count(size: u64) -> usize {
    let size_limited =
        usize::try_from(size / MIN_SEGMENT_SIZE).unwrap_or(usize::MAX);
    INITIAL_SEGMENT_CONCURRENCY
        .min(MAX_SEGMENT_CONCURRENCY)
        .min(size_limited.max(1))
}

async fn download_segment(
    connection: &SharedH2Connection,
    uri: &Uri,
    mut headers: HeaderMap,
    start: u64,
    end: u64,
    segment_path: &Path,
) -> crate::Result<Bytes> {
    headers.insert(
        RANGE,
        HeaderValue::from_str(&format!("bytes={start}-{end}")).unwrap(),
    );
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));

    let (response, mut stream) = open_stream(connection, uri, headers).await?;
    if response.status() != StatusCode::PARTIAL_CONTENT
        && response.status() != StatusCode::OK
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP/2 range GET failed with status {}",
            response.status()
        ))
        .into());
    }

    let mut file = tokio::fs::File::create(segment_path).await?;
    let mut bytes = Vec::with_capacity((end - start + 1) as usize);
    loop {
        let chunk = tokio::time::timeout(RANGE_IDLE_TIMEOUT, stream.data())
            .await
            .map_err(|_| {
                crate::ErrorKind::NetworkError(
                    "range stream receive timed out".into(),
                )
            })?
            .transpose()
            .map_err(|error| {
                crate::ErrorKind::NetworkError(format!(
                    "range stream error: {error}"
                ))
            })?;
        let Some(chunk) = chunk else {
            break;
        };
        file.write_all(&chunk).await?;
        bytes.extend_from_slice(&chunk);
    }
    file.flush().await?;
    drop(file);
    Ok(Bytes::from(bytes))
}

fn segment_path(part_path: &Path, index: usize) -> std::path::PathBuf {
    let mut name = part_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(format!(".segment-{index}"));
    part_path.with_file_name(name)
}

async fn record_install_stage(request: &DownloadRequest) {
    if let Some(tracking) = &request.install_tracking {
        let reporter = tracking.reporter.clone();
        let item_id = tracking.item_id.clone();
        let _ = reporter
            .record_download_stage(
                item_id,
                crate::install::DownloadItemStatus::Verifying,
            )
            .await;
    }
}

async fn record_install_progress(
    request: &DownloadRequest,
    downloaded: u64,
    total_size: u64,
) {
    if let Some(tracking) = &request.install_tracking {
        let reporter = tracking.reporter.clone();
        let item_id = tracking.item_id.clone();
        let _ = reporter
            .record_download_progress(item_id, downloaded, total_size)
            .await;
    }
}

async fn verify_and_finalize(
    part_path: &Path,
    destination: &Path,
    integrity: &Integrity,
    hashers: fetch::ComputedIntegrity,
    downloaded: u64,
    _expected_size: u64,
) -> crate::Result<()> {
    // The size check lives inside `verify_computed_integrity`: the hash is
    // authoritative whenever one is available, mirroring the legacy path.
    if let Err(error) = fetch::verify_computed_integrity(integrity, &hashers) {
        return Err(error);
    }
    if let Err(error) =
        fetch::validate_file_content(part_path, integrity.content).await
    {
        return Err(error);
    }
    if downloaded == 0 {
        return Err(crate::ErrorKind::OtherError(
            "downloaded file is empty".to_string(),
        )
        .into());
    }
    fetch::finalize_download(part_path, destination).await?;
    Ok(())
}
