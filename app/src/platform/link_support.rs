//! Platform pieces the connect flow shares on iOS and Android: foreground
//! time, the Wi-Fi interface address, and one probe of the GRKit service.
// The desktop stub has no camera link; only the clock is used there.
#![cfg_attr(not(any(target_os = "ios", target_os = "android")), allow(dead_code))]

use std::cell::Cell;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::{lifecycle, timer};

use crate::domain::connect_policy::NetworkSnapshot;
use crate::domain::{LinkError, Probe, parse_props};

#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
const FOREGROUND_POLL: Duration = Duration::from_millis(250);

thread_local! {
    /// Foreground time so far, and when it was last sampled.
    static CLOCK: Cell<(Duration, Option<Instant>)> = const { Cell::new((Duration::ZERO, None)) };
}

fn active() -> bool {
    lifecycle() == Lifecycle::Active
}

/// Monotonic time that advances only while the app is in the foreground.
/// Each call adds the time since the previous one when the app is active;
/// the connect loops call it at least every probe interval.
pub fn foreground_clock() -> Duration {
    CLOCK.with(|clock| {
        let (total, last) = clock.get();
        let now = Instant::now();
        let total = match last {
            Some(last) if active() => total + now.duration_since(last),
            _ => total,
        };
        clock.set((total, Some(now)));
        total
    })
}

/// Return once the app is in the foreground.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub async fn wait_foreground() {
    while !active() {
        timer(FOREGROUND_POLL).await;
    }
}

/// Wi-Fi interface names: `en0` on iOS, `wlan*` on Android.
fn wifi_interface(name: &str) -> bool {
    name == "en0" || name.starts_with("wlan")
}

/// The IPv4 address of the Wi-Fi interface, read with `getifaddrs`.
pub fn network_snapshot() -> NetworkSnapshot {
    let mut snapshot = NetworkSnapshot::default();
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: `getifaddrs` fills `list` with a linked list that is released
    // below with `freeifaddrs`; each node is read only while the list lives.
    unsafe {
        if libc::getifaddrs(&mut list) != 0 {
            return snapshot;
        }
        let mut node = list;
        while !node.is_null() {
            let entry = &*node;
            node = entry.ifa_next;
            if entry.ifa_name.is_null() || entry.ifa_addr.is_null() {
                continue;
            }
            let name = std::ffi::CStr::from_ptr(entry.ifa_name).to_string_lossy();
            if !wifi_interface(&name) {
                continue;
            }
            snapshot.wifi = true;
            if i32::from((*entry.ifa_addr).sa_family) == libc::AF_INET {
                let address = &*(entry.ifa_addr as *const libc::sockaddr_in);
                snapshot.address = Some(Ipv4Addr::from(u32::from_be(address.sin_addr.s_addr)));
            }
        }
        libc::freeifaddrs(list);
    }
    snapshot
}

/// Whether an HTTP answer is something other than camera JSON: an HTML
/// router page or another non-JSON body under 500.
fn non_camera_answer(status: u16, body: &str) -> bool {
    status < 500
        && !body.trim().is_empty()
        && !serde_json::from_str::<serde_json::Value>(body).is_ok_and(|v| v.is_object())
}

/// `GET /v1/props` within `timeout`, parsed; `None` when the service did not
/// answer with its properties.
pub async fn read_props(timeout: Duration) -> Option<crate::domain::PropsResponse> {
    let url = format!("{}/v1/props", super::camera_http_base());
    let response = super::camera_request("GET", url, Vec::new(), timeout, &[])
        .await
        .ok()?;
    if !response.ok() {
        return None;
    }
    parse_props(&response.text()).ok()
}

/// One `GET /v1/props` within `timeout`.
pub async fn probe_props(timeout: Duration) -> Probe {
    let url = format!("{}/v1/props", super::camera_http_base());
    let response = match super::camera_request("GET", url, Vec::new(), timeout, &[]).await {
        Ok(response) => response,
        Err(error) => {
            app_info!("probe /v1/props failed: {error}");
            return Probe::Failed;
        }
    };
    let status = response.status();
    if status != 200 {
        app_info!("probe /v1/props HTTP {status}");
    }
    let body = response.text();
    if crate::domain::should_retry_status(status) {
        return Probe::CardNotReady;
    }
    if non_camera_answer(status, &body) {
        return Probe::NotCamera;
    }
    if !(200..300).contains(&status) {
        return Probe::Failed;
    }
    match parse_props(&body) {
        Ok(_) => Probe::Reachable,
        Err(error) => Probe::Fatal(LinkError::Http(format!(
            "props body ({} bytes): {error}",
            body.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_pages_are_not_camera_answers() {
        assert!(non_camera_answer(404, "<html>Not Found</html>"));
        assert!(non_camera_answer(200, "<html>login</html>"));
        assert!(!non_camera_answer(404, r#"{"errCode":404}"#));
        assert!(!non_camera_answer(502, "<html>bad gateway</html>"));
        assert!(!non_camera_answer(404, ""));
    }

    #[test]
    fn the_host_has_a_readable_interface_list() {
        // Desktop hosts have no en0/wlan in CI sometimes; the call must not fail.
        let _ = network_snapshot();
    }
}
