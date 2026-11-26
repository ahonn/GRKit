//! The camera's own HTTP API on port 80, which tells the model and the
//! firmware of the camera.

use crate::{head_end, transport::DeadlineReader, update::Device};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

/// Maximum time for the camera properties API.
const TIMEOUT: Duration = crate::transport::IDLE_TIMEOUT;

/// The camera, as its own HTTP API on port 80 describes it, and the free
/// space of the card.
pub fn device() -> anyhow::Result<Device> {
    use anyhow::Context;
    let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], 80).into(), TIMEOUT)
        .context("The camera's Wi-Fi API is not ready")?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.write_all(b"GET /v1/props HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")?;
    let mut reader = DeadlineReader::new(&mut stream);
    let mut answer = Vec::new();
    // The API may keep the connection open after its answer: stop at the end
    // of the JSON body.
    let mut buffer = [0; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => answer.extend_from_slice(&buffer[..count]),
            Err(error) if answer.is_empty() => return Err(error.into()),
            Err(_) => break,
        }
        anyhow::ensure!(
            answer.len() <= crate::MAX_FILE,
            "The camera properties response is too large"
        );
        if let Some(at) = head_end(&answer) {
            if serde_json::from_slice::<serde_json::Value>(&answer[at..]).is_ok() {
                break;
            }
        }
    }
    let at = head_end(&answer).context("The camera's Wi-Fi API gave no answer")?;
    let props: serde_json::Value =
        serde_json::from_slice(&answer[at..]).context("Read the camera's properties")?;
    let text = |key: &str| {
        props[key]
            .as_str()
            .map(str::to_owned)
            .with_context(|| format!("The camera's properties have no {key}"))
    };
    Ok(Device {
        model: text("model")?,
        firmware: text("firmwareVersion")?,
        free_bytes: free_bytes(grkit_core::card::LINUX_CARD),
        serial_number: props["serialNo"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    })
}

/// The free space of the file system at `path`.
#[cfg(target_os = "linux")]
fn free_bytes(path: &str) -> Option<u64> {
    let path = std::ffi::CString::new(path).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a NUL-terminated string and `stats` has room for the
    // result, which is read only on success.
    let stats = unsafe {
        if libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) != 0 {
            return None;
        }
        stats.assume_init()
    };
    #[allow(
        clippy::useless_conversion,
        reason = "f_frsize is 32 bits on the camera's armv7 Linux and 64 bits on x86_64"
    )]
    let free = u64::from(stats.f_bavail) * u64::from(stats.f_frsize);
    Some(free)
}

#[cfg(not(target_os = "linux"))]
fn free_bytes(_path: &str) -> Option<u64> {
    None
}
