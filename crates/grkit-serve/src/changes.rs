//! The change stream: one WebSocket on which the service tells the phone
//! that something changed. An event names a kind of change and carries no
//! data: the phone then reads that resource again over HTTP. The phone also
//! reads each resource after it connects, so the stream keeps no history.
//!
//! A change on the camera itself (MENU, ADJ, a recipe, the mode dial) reaches
//! the stream through the count of setting changes that the App task copies
//! into the control mailbox. The stream reads that word from memory: it sends
//! no command to the camera and does not read the card.
use crate::protocol::Problem;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::json;
use std::{
    io::{self, Cursor, Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame, WebSocketConfig},
    Error, Message,
};

pub const MAX_SUBSCRIBERS: usize = 2;
/// The stream looks for a change and for a frame of the phone this often.
const READ_POLL: Duration = Duration::from_millis(100);
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
const PING_INTERVAL: Duration = Duration::from_secs(15);
const PONG_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FRAME: usize = 1024;
const MAX_OUTPUT: usize = 4 * 1024;

/// A kind of change, as the `changed` field of an event names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changed {
    /// The Looks of the card or their choices, by a request of the phone.
    Looks = 0,
    /// The update that waits on the card.
    Software = 1,
    /// A GRKit setting, changed on the camera.
    Settings,
}
impl Changed {
    fn name(self) -> &'static str {
        match self {
            Self::Looks => "looks",
            Self::Software => "software",
            Self::Settings => "settings",
        }
    }
}

/// The counts of the changes of each kind. A subscriber sends an event when
/// a count differs from the one it last saw.
#[derive(Clone, Default)]
pub struct Changes {
    counts: Arc<[AtomicU32; 2]>,
    active: Arc<AtomicUsize>,
}

impl Changes {
    /// A request of the phone changed the Looks or the software state.
    pub fn note(&self, kind: Changed) {
        if let Some(count) = self.counts.get(kind as usize) {
            count.fetch_add(1, Ordering::AcqRel);
        }
    }
    pub(crate) fn counts(&self) -> [u32; 2] {
        std::array::from_fn(|kind| self.counts[kind].load(Ordering::Acquire))
    }

    /// Reserve before starting a dedicated WebSocket worker. Release the HTTP
    /// worker slot once that worker owns the connection. A failed spawn drops
    /// this value and releases its capacity reservation.
    pub fn reserve(&self, boot_id: &str) -> Result<Subscription, Problem> {
        let mut count = self.active.load(Ordering::Acquire);
        loop {
            if count >= MAX_SUBSCRIBERS {
                return Err(Problem::new(
                    503,
                    "subscriber_capacity",
                    "The change subscription limit has been reached.",
                ));
            }
            match self.active.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => count = current,
            }
        }
        Ok(Subscription {
            changes: self.clone(),
            boot_id: boot_id.to_owned(),
        })
    }
}

pub struct Subscription {
    changes: Changes,
    boot_id: String,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.changes.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Subscription {
    /// `prefix` contains the complete, admitted HTTP head and any bytes read
    /// with it. Replaying these bytes preserves an early first WebSocket frame.
    /// `settings` gives the count of setting changes of the camera, or None
    /// while it is not known.
    pub fn serve(
        self,
        stream: TcpStream,
        prefix: Vec<u8>,
        settings: impl Fn() -> Option<u32>,
    ) -> Result<()> {
        if prefix.len() > crate::MAX_HEAD || crate::head_end(&prefix).is_none() {
            bail!("The WebSocket request head is incomplete or oversized");
        }
        let request = crate::parse(&prefix).context("Invalid WebSocket request")?;
        if request.method != "GET" || request.path != "/v1/changes" || request.length != 0 {
            bail!("Invalid change subscription endpoint");
        }
        crate::transport::admit(&request, stream.local_addr()?)
            .map_err(|response| anyhow!("WebSocket admission failed: {}", response.body))?;
        let config = WebSocketConfig::default()
            .read_buffer_size(2048)
            .write_buffer_size(0)
            .max_write_buffer_size(MAX_OUTPUT)
            .max_message_size(Some(MAX_FRAME))
            .max_frame_size(Some(MAX_FRAME));
        let io = SubscriptionIo::new(stream, prefix);
        let mut socket = tungstenite::accept_with_config(io, Some(config))
            .map_err(|error| anyhow!("WebSocket handshake failed: {error}"))?;
        // The counts before the hello: the phone reads each resource after
        // the hello, so a change from here on gets an event.
        let mut seen = self.changes.counts();
        let mut seen_settings = settings();
        socket.get_mut().begin_write();
        socket.send(Message::text(
            json!({"type":"hello","bootId":self.boot_id}).to_string(),
        ))?;
        let mut next_ping = Instant::now() + PING_INTERVAL;
        let mut awaiting_pong = None;
        loop {
            // The first known count is no change: the phone reads the
            // settings after the hello.
            let mut changed = Vec::new();
            if let Some(count) = settings() {
                if seen_settings
                    .replace(count)
                    .is_some_and(|last| last != count)
                {
                    changed.push(Changed::Settings);
                }
            }
            let counts = self.changes.counts();
            for kind in [Changed::Looks, Changed::Software] {
                if counts[kind as usize] != seen[kind as usize] {
                    changed.push(kind);
                }
            }
            seen = counts;
            for kind in changed {
                socket.get_mut().begin_write();
                socket.send(Message::text(
                    json!({"type":"changed","changed":kind.name()}).to_string(),
                ))?;
            }
            let now = Instant::now();
            if awaiting_pong.is_some_and(|sent| now.duration_since(sent) >= PONG_TIMEOUT) {
                bail!("WebSocket heartbeat timed out");
            }
            if awaiting_pong.is_none() && now >= next_ping {
                socket.get_mut().begin_write();
                socket.send(Message::Ping(b"grkit".to_vec().into()))?;
                awaiting_pong = Some(now);
            }
            socket.get_mut().begin_read();
            match socket.read() {
                Ok(Message::Ping(_)) => {
                    socket.get_mut().begin_write();
                    socket.flush()?; // tungstenite queued the matching Pong.
                }
                Ok(Message::Pong(payload)) => {
                    if payload.as_ref() == b"grkit" && awaiting_pong.is_some() {
                        awaiting_pong = None;
                        next_ping = Instant::now() + PING_INTERVAL;
                    }
                }
                Ok(Message::Close(_)) => {
                    socket.get_mut().begin_write();
                    let _ = socket.flush();
                    return Ok(());
                }
                Ok(_) => {
                    socket.get_mut().begin_write();
                    socket.close(Some(CloseFrame {
                        code: CloseCode::Policy,
                        reason: "Use HTTP for application requests".into(),
                    }))?;
                    return Ok(());
                }
                Err(Error::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) => {}
                Err(Error::ConnectionClosed | Error::AlreadyClosed) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

/// Per-call deadlines also bound streams of tiny fragments or partial writes.
/// Tungstenite retains partial frames across a read timeout.
struct SubscriptionIo {
    stream: TcpStream,
    prefix: Cursor<Vec<u8>>,
    read_deadline: Instant,
    write_deadline: Instant,
}
impl SubscriptionIo {
    fn new(stream: TcpStream, prefix: Vec<u8>) -> Self {
        let now = Instant::now();
        Self {
            stream,
            prefix: Cursor::new(prefix),
            read_deadline: now + WRITE_TIMEOUT,
            write_deadline: now + WRITE_TIMEOUT,
        }
    }
    fn begin_read(&mut self) {
        self.read_deadline = Instant::now() + READ_POLL;
    }
    fn begin_write(&mut self) {
        self.write_deadline = Instant::now() + WRITE_TIMEOUT;
    }
    fn remaining(deadline: Instant) -> io::Result<Duration> {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::TimedOut, "WebSocket I/O deadline exceeded")
            })
    }
}
impl Read for SubscriptionIo {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let remaining = Self::remaining(self.read_deadline)?;
        if self.prefix.position() < self.prefix.get_ref().len() as u64 {
            return self.prefix.read(bytes);
        }
        self.stream.set_read_timeout(Some(remaining))?;
        self.stream.read(bytes)
    }
}
impl Write for SubscriptionIo {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream
            .set_write_timeout(Some(Self::remaining(self.write_deadline)?))?;
        self.stream.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::{net::TcpListener, sync::Mutex, thread};
    use tungstenite::WebSocket;

    type Settings = Arc<Mutex<Option<u32>>>;

    fn connect(
        subscription: Subscription,
        settings: Settings,
    ) -> (WebSocket<TcpStream>, thread::JoinHandle<Result<()>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let prefix = crate::read_head(&mut stream).unwrap();
            subscription.serve(stream, prefix, || *settings.lock().unwrap())
        });
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let (client, _) =
            tungstenite::client(format!("ws://{address}/v1/changes"), stream).unwrap();
        (client, worker)
    }

    fn text(client: &mut WebSocket<TcpStream>) -> Value {
        serde_json::from_str(client.read().unwrap().to_text().unwrap()).unwrap()
    }

    #[test]
    fn sends_hello_changes_and_control_pong() {
        let changes = Changes::default();
        let settings = Settings::default();
        let (mut client, worker) =
            connect(changes.reserve("boot-test").unwrap(), Arc::clone(&settings));
        assert_eq!(
            text(&mut client),
            json!({"type":"hello","bootId":"boot-test"})
        );
        changes.note(Changed::Looks);
        assert_eq!(
            text(&mut client),
            json!({"type":"changed","changed":"looks"})
        );
        changes.note(Changed::Software);
        assert_eq!(
            text(&mut client),
            json!({"type":"changed","changed":"software"})
        );
        // The first count that the camera gives is no change. A later one is.
        *settings.lock().unwrap() = Some(7);
        thread::sleep(READ_POLL * 3);
        *settings.lock().unwrap() = Some(8);
        assert_eq!(
            text(&mut client),
            json!({"type":"changed","changed":"settings"})
        );
        client.send(Message::Ping(b"test".to_vec().into())).unwrap();
        assert_eq!(
            client.read().unwrap(),
            Message::Pong(b"test".to_vec().into())
        );
        client.close(None).unwrap();
        let _ = client.read();
        worker.join().unwrap().unwrap();
        assert_eq!(changes.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn rejects_application_commands_and_releases_subscription() {
        let changes = Changes::default();
        let (mut client, worker) =
            connect(changes.reserve("boot-test").unwrap(), Settings::default());
        text(&mut client);
        client
            .send(Message::text("{\"action\":\"delete\"}"))
            .unwrap();
        let Message::Close(Some(close)) = client.read().unwrap() else {
            panic!("expected policy close");
        };
        assert_eq!(close.code, CloseCode::Policy);
        worker.join().unwrap().unwrap();
        assert_eq!(changes.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn admits_a_bounded_number_of_subscribers() {
        let changes = Changes::default();
        let held: Vec<_> = (0..MAX_SUBSCRIBERS)
            .map(|_| changes.reserve("boot-test").unwrap())
            .collect();
        assert_eq!(
            changes.reserve("boot-test").err().unwrap().code,
            "subscriber_capacity"
        );
        drop(held);
        assert!(changes.reserve("boot-test").is_ok());
    }
}
