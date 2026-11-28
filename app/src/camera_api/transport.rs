//! Where the camera service is, and the turns its requests take.

use crate::domain::CAMERA_HTTP_BASE;

/// Base URL for camera HTTP. In a development build, `GRKIT_CAMERA_URL`
/// overrides the on-camera default when set.
pub fn camera_http_base() -> String {
    dev_camera_url()
        .map(|s| s.trim().trim_end_matches('/').to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CAMERA_HTTP_BASE.to_owned())
}

/// True when a development build has `GRKIT_CAMERA_URL` set: it connects
/// over HTTP alone (simulator, local server).
pub fn has_dev_camera_url() -> bool {
    dev_camera_url().is_some_and(|s| !s.trim().is_empty())
}

/// `GRKIT_CAMERA_URL`, which only a development build (`cfg(demo)`) reads.
fn dev_camera_url() -> Option<String> {
    if cfg!(demo) {
        crate::platform::config_value("GRKIT_CAMERA_URL")
    } else {
        None
    }
}

/// The camera service answers at most four connections at once
/// (grkit-serve `MAX_CONNECTIONS`); a connection beyond them waits until a
/// request ends. The change feed holds one and the link check
/// (`platform::camera_request_now`) one,
/// so the other requests to the service take turns in the rest.
const SERVICE_SLOTS: usize = 2;
static SERVICE_IN_FLIGHT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A turn at the camera service, given back when dropped.
struct ServiceTurn;

impl ServiceTurn {
    async fn take() -> Self {
        use std::sync::atomic::Ordering;
        loop {
            let current = SERVICE_IN_FLIGHT.load(Ordering::Acquire);
            if current < SERVICE_SLOTS
                && SERVICE_IN_FLIGHT
                    .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                return Self;
            }
            nsh_core::prelude::timer(std::time::Duration::from_millis(10)).await;
        }
    }
}

impl Drop for ServiceTurn {
    fn drop(&mut self) {
        SERVICE_IN_FLIGHT.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}

/// A request to the camera. One to the GRKit service waits for its turn
/// first; `budget` counts from the send.
pub(crate) async fn camera_request(
    method: &str,
    url: String,
    body: Vec<u8>,
    budget: std::time::Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, String> {
    let _turn = if url.starts_with(&camera_http_base()) {
        Some(ServiceTurn::take().await)
    } else {
        None
    };
    crate::platform::camera_request_now(method, url, body, budget, headers).await
}
