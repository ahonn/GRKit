//! Bounded HTTP transport admission. This layer never writes camera state.
use crate::{api::Reply, Request};
use std::{
    io::{self, Read},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub const MAX_CONNECTIONS: usize = 4;
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
// A separate total bound prevents a peer from keeping a worker forever by
// sending one byte before each idle timeout. Measure this on physical Wi-Fi.
pub const TRANSFER_TIMEOUT: Duration = Duration::from_secs(120);

pub fn admit(request: &Request, local: SocketAddr) -> Result<(), Reply> {
    let host = request
        .headers
        .get("host")
        .ok_or_else(|| Reply::error(400, "Host is required"))?;
    let address = host.parse::<SocketAddr>().ok().or_else(|| {
        host.parse::<IpAddr>()
            .ok()
            .map(|ip| SocketAddr::new(ip, 80))
    });
    if address != Some(local) {
        return Err(Reply::error(
            403,
            "Host must match the local IP address and port",
        ));
    }
    if request.headers.contains_key("origin") {
        return Err(Reply::error(403, "Browser requests are not allowed"));
    }
    // The body that the route takes with the method (`route::Route::body`).
    match crate::route::Route::parse(&request.path).and_then(|route| route.body(&request.method)) {
        Some(body) => body
            .check(
                request.length,
                request.headers.get("content-type").map(String::as_str),
            )
            .map_err(Reply::problem)?,
        None if request.length > 0 => {
            return Err(Reply::error(
                400,
                "This endpoint does not accept a request body",
            ))
        }
        None => {}
    }
    // The service answers `100-continue` (or a final status) before it
    // reads a body; it supports no other expectation.
    if request
        .headers
        .get("expect")
        .is_some_and(|value| !value.eq_ignore_ascii_case("100-continue"))
    {
        return Err(Reply::error(417, "Expect is not supported"));
    }
    Ok(())
}

/// A slot is released even if a connection exits early or a worker panics.
pub struct ConnectionSlot(Arc<AtomicUsize>);
impl ConnectionSlot {
    pub fn acquire(count: &Arc<AtomicUsize>) -> Option<Self> {
        let mut current = count.load(Ordering::Acquire);
        loop {
            if current >= MAX_CONNECTIONS {
                return None;
            }
            match count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self(Arc::clone(count))),
                Err(actual) => current = actual,
            }
        }
    }
}
impl ConnectionSlot {
    /// Waits for a free slot, or `None` once `stopped` says so. The service
    /// takes a connection only with a slot, so one beyond MAX_CONNECTIONS
    /// waits in the listen queue instead of closing without an answer.
    pub fn wait(count: &Arc<AtomicUsize>, stopped: impl Fn() -> bool) -> Option<Self> {
        loop {
            if let Some(slot) = Self::acquire(count) {
                return Some(slot);
            }
            if stopped() {
                return None;
            }
            std::thread::sleep(SLOT_PAUSE);
        }
    }
}
/// How often a full service looks for a free slot.
const SLOT_PAUSE: Duration = Duration::from_millis(20);
impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Wait until no connection holds a slot, or until `deadline`: a service that
/// stops answers each request that it took, card writes included. True when
/// no connection is left.
pub fn drain(count: &AtomicUsize, deadline: Instant) -> bool {
    while count.load(Ordering::Acquire) > 0 {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

/// Apply the total deadline to each read, including the request head.
pub struct DeadlineReader<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
}
impl<'a> DeadlineReader<'a> {
    pub fn new(stream: &'a mut TcpStream) -> Self {
        Self {
            stream,
            deadline: Instant::now() + TRANSFER_TIMEOUT,
        }
    }
    /// The connection, to answer before the body is read.
    pub fn get_mut(&mut self) -> &mut TcpStream {
        self.stream
    }
}
impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|time| !time.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "transfer deadline exceeded"))?;
        self.stream
            .set_read_timeout(Some(remaining.min(IDLE_TIMEOUT)))?;
        self.stream.read(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    fn accepted(headers: &str, method: &str) -> bool {
        let request =
            parse(format!("{method} /v1/ping HTTP/1.1\r\n{headers}\r\n").as_bytes()).unwrap();
        admit(&request, "192.168.0.1:8080".parse().unwrap()).is_ok()
    }
    #[test]
    fn rejects_browser_and_rebinding_requests() {
        assert!(accepted("Host: 192.168.0.1:8080\r\n", "GET"));
        assert!(!accepted("Host: camera.example:8080\r\n", "GET"));
        assert!(!accepted("Host: 192.168.0.2:8080\r\n", "GET"));
        assert!(!accepted(
            "Host: 192.168.0.1:8080\r\nOrigin: null\r\n",
            "GET"
        ));
        assert!(!accepted("", "GET"));
    }
    #[test]
    fn the_settings_and_layout_routes_take_a_json_body() {
        for (method, path) in [
            ("PATCH", "/v1/params/settings"),
            ("PUT", "/v1/params/modules/layout"),
        ] {
            let request = parse(
                format!(
                    "{method} {path} HTTP/1.1\r\nHost: 192.168.0.1:8080\r\n\
                     Content-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}"
                )
                .as_bytes(),
            )
            .unwrap();
            assert!(admit(&request, "192.168.0.1:8080".parse().unwrap()).is_ok());
        }
    }
    #[test]
    fn only_the_continue_expectation_is_accepted() {
        assert!(accepted(
            "Host: 192.168.0.1:8080\r\nExpect: 100-continue\r\n",
            "GET"
        ));
        assert!(!accepted(
            "Host: 192.168.0.1:8080\r\nExpect: something-else\r\n",
            "GET"
        ));
    }
    #[test]
    fn connection_slots_are_bounded_and_released() {
        let count = Arc::new(AtomicUsize::new(0));
        let slots: Vec<_> = (0..MAX_CONNECTIONS)
            .map(|_| ConnectionSlot::acquire(&count).unwrap())
            .collect();
        assert!(ConnectionSlot::acquire(&count).is_none());
        drop(slots);
        assert!(ConnectionSlot::acquire(&count).is_some());
    }
    #[test]
    fn a_full_service_waits_for_a_slot() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut slots: Vec<_> = (0..MAX_CONNECTIONS)
            .map(|_| ConnectionSlot::acquire(&count).unwrap())
            .collect();
        let held = slots.pop().unwrap();
        let freed = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drop(held);
        });
        let _taken = ConnectionSlot::wait(&count, || false).unwrap();
        freed.join().unwrap();
        assert!(ConnectionSlot::wait(&count, || true).is_none());
    }
    #[test]
    fn a_stop_waits_for_the_request_in_flight() {
        let count = Arc::new(AtomicUsize::new(0));
        let slot = ConnectionSlot::acquire(&count).unwrap();
        let busy = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drop(slot);
        });
        let started = Instant::now();
        assert!(drain(&count, started + Duration::from_secs(5)));
        assert!(started.elapsed() >= Duration::from_millis(100));
        busy.join().unwrap();
        let _held = ConnectionSlot::acquire(&count).unwrap();
        assert!(!drain(&count, Instant::now()));
    }
    #[test]
    fn total_deadline_expires_even_when_idle_timeout_has_not() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer, _) = listener.accept().unwrap();
        let mut reader = DeadlineReader {
            stream: &mut stream,
            deadline: Instant::now(),
        };
        assert_eq!(
            reader.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
}
