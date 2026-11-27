//! What iOS and the desktop build do the same way, where Android has its
//! own bridge: HTTP, sockets, the route, and the settings page.

use std::io;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::domain::connect_policy::NetworkSnapshot;

/// No host settings: the process environment alone (`config_value`).
pub fn config(_name: &str) -> Option<String> {
    None
}

/// Only Android reports a lost camera network; elsewhere the probes notice.
pub fn watch_network_loss(tx: async_channel::Sender<()>) {
    drop(tx);
}

pub async fn network_snapshot() -> NetworkSnapshot {
    super::link_support::network_snapshot()
}

pub async fn open_app_settings() {
    let _ = nsh_core::prelude::open_url("app-settings:").await;
}

pub fn connect_socket(address: SocketAddr, timeout: Duration) -> io::Result<TcpStream> {
    TcpStream::connect_timeout(&address, timeout)
}

pub async fn camera_request(
    method: &str,
    url: String,
    body: Vec<u8>,
    budget: Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, String> {
    let mut request = nsh_core::platform::HttpRequest::new(method, url)
        .header("Cache-Control", "no-cache, no-store");
    for (name, value) in headers {
        request = request.header(*name, value.clone());
    }
    let mut request = request.body(body);
    request.follow_redirects = false;
    super::with_timeout(nsh_core::prelude::fetch(request), budget)
        .await
        .ok_or_else(|| "ConnectionLost".to_owned())?
}
