//! Platform pieces the connect flow shares on iOS and Android: foreground
//! time and the Wi-Fi interface address.
// The desktop stub has no camera link; only the clock is used there.
#![cfg_attr(not(any(target_os = "ios", target_os = "android")), allow(dead_code))]

use std::cell::Cell;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::{lifecycle, timer};

use crate::domain::connect_policy::NetworkSnapshot;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_has_a_readable_interface_list() {
        // Desktop hosts have no en0/wlan in CI sometimes; the call must not fail.
        let _ = network_snapshot();
    }
}
