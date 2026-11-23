//! The change stream of the camera service, on a thread of its own: a small
//! WebSocket client on `std::net::TcpStream` (`domain::changes` holds the
//! frames and the events). The thread answers each Ping, forwards events to
//! the app, and ends when the app drops the stream, when the socket fails,
//! or when the stream falls silent.

use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::domain::changes::{
    self, ChangeEvent, MAX_HEAD, Opcode, SILENCE_LIMIT, check_handshake, close_code, decode_frame,
    encode_frame, head_length, parse_event,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// How often the thread looks at the stop flag while it waits for a frame.
const READ_POLL: Duration = Duration::from_millis(500);
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
/// The status code of a Close frame that ends the stream normally.
const NORMAL_CLOSURE: u16 = 1000;

/// What the stream tells the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    Change(ChangeEvent),
    /// Another frame: a Ping, a Pong, or an event the app does not know.
    Heartbeat,
}

/// An open change stream. Dropping it ends its thread.
pub struct ChangeStream {
    events: async_channel::Receiver<StreamEvent>,
    stop: Arc<AtomicBool>,
}

impl ChangeStream {
    /// The next event; `None` once the stream ended.
    pub async fn next(&self) -> Option<StreamEvent> {
        self.events.recv().await.ok()
    }
}

impl Drop for ChangeStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Whether this build opens the change stream. The desktop build has no
/// camera link.
pub fn supported() -> bool {
    cfg!(any(target_os = "ios", target_os = "android"))
}

/// Open the change stream of the HTTP service at `base`. The connection and
/// the handshake run on the thread, so this returns at once; a stream that
/// cannot connect ends without an event.
pub fn open(base: &str) -> ChangeStream {
    let (tx, rx) = async_channel::unbounded();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let base = base.to_owned();
    let spawned = std::thread::Builder::new()
        .name("grkit-changes".into())
        .spawn(move || {
            if let Err(error) = run(&base, &tx, &flag) {
                log::info!("Change stream ended: {error}");
            }
        });
    // A thread that did not start drops its sender, which ends the stream.
    if let Err(error) = spawned {
        log::warn!("Could not start the change stream: {error}");
    }
    ChangeStream { events: rx, stop }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn run(base: &str, tx: &async_channel::Sender<StreamEvent>, stop: &AtomicBool) -> io::Result<()> {
    let authority = changes::authority(base)
        .ok_or_else(|| invalid(format!("The camera address is not an HTTP URL: {base}")))?;
    let address = authority
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| invalid(format!("No address for {authority}")))?;
    let mut socket = connect(address)?;
    socket.set_nodelay(true)?;
    socket.set_write_timeout(Some(WRITE_TIMEOUT))?;
    let mut buffer = handshake(&mut socket, address)?;
    socket.set_read_timeout(Some(READ_POLL))?;
    let mut heard = Instant::now();
    let mut chunk = [0; 1024];
    loop {
        while let Some((frame, used)) =
            decode_frame(&buffer).map_err(|error| invalid(format!("Frame refused: {error:?}")))?
        {
            buffer.drain(..used);
            heard = Instant::now();
            let event = match frame.opcode {
                Opcode::Text => std::str::from_utf8(&frame.payload)
                    .ok()
                    .and_then(parse_event)
                    .map_or(StreamEvent::Heartbeat, StreamEvent::Change),
                Opcode::Ping => {
                    send(&mut socket, Opcode::Pong, &frame.payload)?;
                    StreamEvent::Heartbeat
                }
                Opcode::Close => {
                    // Answer with the same code, as RFC 6455 asks.
                    let code = close_code(&frame.payload).map(u16::to_be_bytes);
                    let _ = send(
                        &mut socket,
                        Opcode::Close,
                        code.as_ref().map_or(&[][..], |c| &c[..]),
                    );
                    return Ok(());
                }
                Opcode::Binary | Opcode::Pong => StreamEvent::Heartbeat,
            };
            if tx.try_send(event).is_err() {
                return close(&mut socket);
            }
        }
        if stop.load(Ordering::Relaxed) || tx.is_closed() {
            return close(&mut socket);
        }
        if heard.elapsed() > SILENCE_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "The change stream fell silent",
            ));
        }
        match socket.read(&mut chunk) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The camera closed the change stream",
                ));
            }
            Ok(count) => buffer.extend_from_slice(&chunk[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
}

/// Send the upgrade request and read the answer head. Returns the bytes
/// after the head: the first frames can come with it.
fn handshake(socket: &mut TcpStream, address: SocketAddr) -> io::Result<Vec<u8>> {
    let key = changes::base64(&random_bytes::<16>());
    // The service takes only the address and port that the phone connected to.
    socket.write_all(changes::handshake_request(&address.to_string(), &key).as_bytes())?;
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut answer = Vec::new();
    let mut chunk = [0; 512];
    loop {
        if let Some(length) = head_length(&answer) {
            check_handshake(&answer[..length]).map_err(invalid)?;
            return Ok(answer.split_off(length));
        }
        if answer.len() > MAX_HEAD {
            return Err(invalid("The handshake answer is too long"));
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "The handshake timed out"))?;
        socket.set_read_timeout(Some(remaining))?;
        match socket.read(&mut chunk)? {
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The camera closed the connection during the handshake",
                ));
            }
            count => answer.extend_from_slice(&chunk[..count]),
        }
    }
}

fn send(socket: &mut TcpStream, opcode: Opcode, payload: &[u8]) -> io::Result<()> {
    socket.write_all(&encode_frame(opcode, payload, random_bytes()))
}

/// End the stream from the app side. The camera may already be gone.
fn close(socket: &mut TcpStream) -> io::Result<()> {
    let _ = send(socket, Opcode::Close, &NORMAL_CLOSURE.to_be_bytes());
    let _ = socket.shutdown(std::net::Shutdown::Both);
    Ok(())
}

/// Bytes that the camera cannot predict, for the handshake key and the
/// masks. Each `RandomState` has its own keys, which the system seeds.
fn random_bytes<const N: usize>() -> [u8; N] {
    use std::hash::{BuildHasher, Hasher, RandomState};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    let mut bytes = [0; N];
    for chunk in bytes.chunks_mut(8) {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes()[..chunk.len()]);
    }
    bytes
}

#[cfg(not(target_os = "android"))]
fn connect(address: SocketAddr) -> io::Result<TcpStream> {
    TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
}

/// On Android the camera Wi-Fi, which has no internet, is not the default
/// network: the socket goes over the network that the app requested for the
/// camera, as the camera HTTP requests do.
#[cfg(target_os = "android")]
fn connect(address: SocketAddr) -> io::Result<TcpStream> {
    use std::os::fd::{FromRawFd, OwnedFd};
    // The camera service has an IPv4 address; a development URL may not.
    let SocketAddr::V4(target) = address else {
        return TcpStream::connect_timeout(&address, CONNECT_TIMEOUT);
    };
    // SAFETY: `socket` returns a new descriptor or -1.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a new descriptor that nothing else owns. Owning it at
    // once closes it on each early return.
    let socket = unsafe { OwnedFd::from_raw_fd(fd) };
    super::android::bind_socket(fd).map_err(io::Error::other)?;
    // Linux bounds a blocking connect by the send timeout.
    let timeout = libc::timeval {
        tv_sec: CONNECT_TIMEOUT.as_secs() as libc::time_t,
        tv_usec: 0,
    };
    // SAFETY: the option value is a `timeval` of the given size.
    let set = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            (&raw const timeout).cast(),
            size_of::<libc::timeval>() as libc::socklen_t,
        )
    };
    if set != 0 {
        return Err(io::Error::last_os_error());
    }
    let target = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: target.port().to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from(*target.ip()).to_be(),
        },
        sin_zero: [0; 8],
    };
    // SAFETY: the address is a `sockaddr_in` of the given size.
    let connected = unsafe {
        libc::connect(
            fd,
            (&raw const target).cast(),
            size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    if connected != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(TcpStream::from(socket))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Read one masked frame of the app, as the service does.
    fn read_frame(peer: &mut TcpStream) -> (u8, Vec<u8>) {
        let mut head = [0; 2];
        peer.read_exact(&mut head).unwrap();
        assert_eq!(head[1] & 0x80, 0x80, "a frame of the app is masked");
        let length = usize::from(head[1] & 0x7F);
        assert!(length <= 125);
        let mut mask = [0; 4];
        peer.read_exact(&mut mask).unwrap();
        let mut payload = vec![0; length];
        peer.read_exact(&mut payload).unwrap();
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        (head[0], payload)
    }

    fn text_frame(text: &str) -> Vec<u8> {
        let mut frame = vec![0x81, text.len() as u8];
        frame.extend_from_slice(text.as_bytes());
        frame
    }

    /// Accept the upgrade like the service: check the request, then answer
    /// 101 with the hello in the same write.
    fn accept(listener: &TcpListener) -> TcpStream {
        let (mut peer, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        let mut byte = [0; 1];
        while head_length(&request).is_none() {
            peer.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let request = String::from_utf8(request).unwrap();
        let local = peer.local_addr().unwrap();
        assert!(request.starts_with("GET /v1/changes HTTP/1.1\r\n"));
        assert!(request.contains(&format!("\r\nHost: {local}\r\n")));
        assert!(!request.to_ascii_lowercase().contains("\r\norigin:"));
        let key = request
            .lines()
            .find_map(|line| line.strip_prefix("Sec-WebSocket-Key: "))
            .unwrap();
        assert_eq!(key.len(), 24, "a base64 key of 16 bytes");
        let mut answer = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
            Connection: Upgrade\r\nSec-WebSocket-Accept: x\r\n\r\n"
            .to_vec();
        answer.extend(text_frame(r#"{"type":"hello","bootId":"boot-test"}"#));
        peer.write_all(&answer).unwrap();
        peer
    }

    fn next(stream: &ChangeStream) -> Option<StreamEvent> {
        stream.events.recv_blocking().ok()
    }

    #[test]
    fn follows_the_stream_answers_pings_and_closes_when_dropped() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stream = open(&format!("http://{address}"));
        let mut peer = accept(&listener);
        assert_eq!(
            next(&stream),
            Some(StreamEvent::Change(ChangeEvent::Hello {
                boot_id: "boot-test".into()
            }))
        );
        peer.write_all(&text_frame(r#"{"type":"changed","changed":"looks"}"#))
            .unwrap();
        assert_eq!(
            next(&stream),
            Some(StreamEvent::Change(ChangeEvent::Changed(
                changes::ChangeKind::Looks
            )))
        );
        peer.write_all(&[0x89, 5]).unwrap();
        peer.write_all(b"grkit").unwrap();
        assert_eq!(read_frame(&mut peer), (0x8A, b"grkit".to_vec()));
        assert_eq!(next(&stream), Some(StreamEvent::Heartbeat));
        drop(stream);
        let (first, payload) = read_frame(&mut peer);
        assert_eq!(first, 0x88);
        assert_eq!(close_code(&payload), Some(NORMAL_CLOSURE));
    }

    #[test]
    fn ends_on_a_fragmented_frame_and_on_a_refused_upgrade() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stream = open(&format!("http://{address}"));
        let mut peer = accept(&listener);
        assert!(matches!(next(&stream), Some(StreamEvent::Change(_))));
        // FIN=0: the service sends each message in one frame.
        peer.write_all(&[0x01, 1, b'x']).unwrap();
        assert_eq!(next(&stream), None);

        let stream = open(&format!("http://{address}"));
        let (mut peer, _) = listener.accept().unwrap();
        peer.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
        assert_eq!(next(&stream), None);
    }
}
