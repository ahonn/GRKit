//! The GRKit service (`grkit_serve`): `SERVE.BIN` on the card.
//!
//! `SCAN.SH` starts the service from the card, because the camera's `/tmp`
//! does not run programs on every camera (GR IV mounts it without exec). The
//! service then runs a copy of itself from RAM (`run_from_ram`), so that no
//! file on the card stays open while it runs.

use grkit_serve::{
    api::{Reply, Service},
    head_end, parse, read_head,
    transport::{admit, drain, ConnectionSlot, DeadlineReader, IDLE_TIMEOUT, TRANSFER_TIMEOUT},
    update::{Card, Device, MAX_PACKAGE},
    Connection, Env, Response, System, PORT,
};
#[cfg(target_os = "linux")]
use std::{
    fs::File,
    os::{
        fd::{FromRawFd, IntoRawFd},
        unix::process::CommandExt,
    },
    process::Command,
};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    process::ExitCode,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    thread,
    time::Duration,
};

/// Maximum idle time for a peer and for the camera properties API.
const PEER_TIMEOUT: Duration = IDLE_TIMEOUT;
/// The pause after a failed `accept`, so that a lasting failure (for example,
/// no free file descriptor) does not keep the CPU busy.
const ACCEPT_PAUSE: Duration = Duration::from_millis(500);
/// The argument of the copy that runs from RAM.
const FROM_RAM: &str = grkit_core::runtime::SERVE_FROM_RAM;
/// How long a request waits while another one reads or writes the card.
const CARD_WAIT: Duration = Duration::from_secs(3);
/// The service marks itself as running in its mailbox as it starts, so that
/// the resident loader of the camera stops starting it
/// (`grkit_core::runtime::SERVE_POLLS`). GRKit can still be starting: the
/// service tries again at this interval, as long as the loader would start
/// it.
const ANNOUNCE_PAUSE: Duration = Duration::from_secs(2);
const ANNOUNCE_TRIES: u32 = grkit_core::runtime::SERVE_POLLS;

/// Waits for the card for at most `CARD_WAIT`. Concurrent requests of one
/// client (for example a page that reads several resources) then queue
/// instead of failing; a long write answers a retryable 503.
fn lock_card(card_access: &Mutex<()>) -> Result<std::sync::MutexGuard<'_, ()>, Reply> {
    let deadline = std::time::Instant::now() + CARD_WAIT;
    loop {
        match card_access.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => return Ok(poisoned.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err(Reply::problem(grkit_serve::protocol::Problem::new(
                    503,
                    "card_busy",
                    "Another request is using the card. Try again.",
                )))
            }
        }
    }
}

fn main() -> ExitCode {
    // A stop of SCAN.SH gives this argument when no service runs: the copy
    // only clears the mark of the service, with no socket.
    if std::env::args().nth(1).as_deref() == Some(grkit_core::runtime::SERVE_MARK_STOPPED) {
        return match grkit_serve::control::discover(std::path::Path::new(
            grkit_core::card::LINUX_CARD,
        ))
        .and_then(|mut client| client.mark_service(false))
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    // Elsewhere, such as on a development computer, the service runs in place.
    if cfg!(target_os = "linux") && std::env::args().nth(1).as_deref() != Some(FROM_RAM) {
        // `run_from_ram` returns only if it fails. The service then stops,
        // because a service that runs from the card keeps its file open.
        let _ = run_from_ram();
        return ExitCode::FAILURE;
    }
    let Ok(listener) = TcpListener::bind(("0.0.0.0", PORT)) else {
        return ExitCode::FAILURE;
    };
    let Ok(boot_id) = grkit_serve::identity::boot_id() else {
        return ExitCode::FAILURE;
    };
    let mut nonce = [0u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut nonce))
        .is_err()
    {
        return ExitCode::FAILURE;
    }
    let backend = grkit_serve::runtime_backend::LazyLinuxBackend::new(
        std::path::PathBuf::from(grkit_core::card::LINUX_CARD),
        nonce,
    );
    let service = Arc::new(Service::with_runtime_backend(boot_id, Box::new(backend)));
    let card_access = Arc::new(Mutex::new(()));
    // No background work on the card: the card cannot change while the
    // service runs (removing it powers the camera off), so each request reads
    // what it needs, and an idle service does not wake the card. The change
    // stream reads one word of memory (`changes`).
    let connections = Arc::new(AtomicUsize::new(0));
    let upload = Arc::new(Mutex::new(()));
    let stop = stop_requests();
    {
        let service = Arc::clone(&service);
        let _ = thread::Builder::new()
            .name("grkit-announce".into())
            .spawn(move || {
                for _ in 0..ANNOUNCE_TRIES {
                    if service.announce(true) {
                        break;
                    }
                    thread::sleep(ANNOUNCE_PAUSE);
                }
            });
    }
    loop {
        if stop_requested(&listener, stop) {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let Some(slot) = ConnectionSlot::acquire(&connections) else {
                    // Do not let an overloaded peer block the accept loop.
                    continue;
                };
                let service = Arc::clone(&service);
                let card_access = Arc::clone(&card_access);
                let upload = Arc::clone(&upload);
                let _ = thread::Builder::new()
                    .name("grkit-http".into())
                    .spawn(move || {
                        let _slot = slot;
                        let _ = serve(&mut stream, &card_access, &upload, &service);
                    });
            }
            Err(_) => thread::sleep(ACCEPT_PAUSE),
        }
    }
    // A stop takes no new connection, answers each request that it took,
    // card writes included, and then lets the loader start the service again.
    drop(listener);
    drain(
        &connections,
        std::time::Instant::now() + TRANSFER_TIMEOUT + PEER_TIMEOUT,
    );
    service.announce(false);
    // SCAN.SH starts the service again unless it ends with success.
    ExitCode::SUCCESS
}

/// The stop request of SCAN.SH (`grkit_core::runtime::SCAN_STOP_ARGUMENT`)
/// is SIGTERM. Its handler writes a byte to a pipe, which the accept loop
/// waits on together with the listener, so the service wakes only for a
/// connection or a stop. Returns the read end, or None where there is none.
#[cfg(target_os = "linux")]
fn stop_requests() -> Option<i32> {
    static STOP_WRITE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
    extern "C" fn on_stop(_: libc::c_int) {
        let fd = STOP_WRITE.load(std::sync::atomic::Ordering::Relaxed);
        // SAFETY: write is async-signal-safe; the byte lives on the stack.
        unsafe {
            libc::write(fd, [1u8].as_ptr().cast(), 1);
        }
    }
    let mut fds = [0; 2];
    // SAFETY: pipe2 fills the two descriptors of the array.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
        return None;
    }
    STOP_WRITE.store(fds[1], std::sync::atomic::Ordering::Relaxed);
    let handler: extern "C" fn(libc::c_int) = on_stop;
    // SAFETY: the handler only calls write, and the descriptor stays open.
    if unsafe { libc::signal(libc::SIGTERM, handler as libc::sighandler_t) } == libc::SIG_ERR {
        return None;
    }
    Some(fds[0])
}
#[cfg(not(target_os = "linux"))]
fn stop_requests() -> Option<i32> {
    None
}

/// Wait for a connection or a stop. True for a stop.
#[cfg(target_os = "linux")]
fn stop_requested(listener: &TcpListener, stop: Option<i32>) -> bool {
    use std::os::fd::AsRawFd;
    let Some(stop) = stop else {
        return false;
    };
    let mut fds = [
        libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: stop,
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    // SAFETY: both descriptors stay open for the call. A signal interrupts
    // the wait with EINTR, after its byte is in the pipe.
    while unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) } < 0 {}
    fds[1].revents & libc::POLLIN != 0
}
#[cfg(not(target_os = "linux"))]
fn stop_requested(_listener: &TcpListener, _stop: Option<i32>) -> bool {
    false
}

/// Copies this program into a memory file and runs the copy in place of this
/// process, with the same process ID. Returns only if a step fails.
#[cfg(target_os = "linux")]
fn run_from_ram() -> io::Result<()> {
    let program = std::fs::read("/proc/self/exe")?;
    // SAFETY: the name is a NUL-terminated string, and the flags are valid.
    // The exec opens the file by its path before it closes the descriptor.
    let fd = unsafe { libc::memfd_create(c"grkit-serve".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a new descriptor that nothing else owns.
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(&program)?;
    // The copy runs from the descriptor, which stays open until the exec.
    let fd = file.into_raw_fd();
    Err(Command::new(format!("/proc/self/fd/{fd}"))
        .arg0("grkit-serve")
        .arg(FROM_RAM)
        .exec())
}

#[cfg(not(target_os = "linux"))]
fn run_from_ram() -> io::Result<()> {
    unreachable!("the service runs in place")
}

fn serve(
    stream: &mut TcpStream,
    card_access: &Mutex<()>,
    upload: &Mutex<()>,
    service: &Arc<Service>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(PEER_TIMEOUT))?;
    stream.set_write_timeout(Some(PEER_TIMEOUT))?;
    let connection = Connection {
        local: stream.local_addr()?.to_string(),
        peer: stream.peer_addr()?.to_string(),
    };
    let local = stream.local_addr()?;
    let mut reader = DeadlineReader::new(stream);
    let mut bytes = read_head(&mut reader)?;
    let response: Reply = match parse(&bytes) {
        None => Response::error(400, "bad request").into(),
        Some(request) if request.length > MAX_PACKAGE => {
            Response::error(413, "the package is too large").into()
        }
        Some(request) => {
            if let Err(response) = admit(&request, local) {
                stream.write_all(&response.to_bytes())?;
                return stream.shutdown(Shutdown::Both);
            }
            if request.path == "/v1/changes" && request.method == "GET" {
                let subscription = match service.changes.reserve(&service.boot_id) {
                    Ok(subscription) => subscription,
                    Err(problem) => {
                        stream.write_all(&Reply::problem(problem).to_bytes())?;
                        return stream.shutdown(Shutdown::Both);
                    }
                };
                let socket = stream.try_clone()?;
                let service = Arc::clone(service);
                thread::Builder::new()
                    .name("grkit-changes".into())
                    .spawn(move || {
                        let _ = subscription.serve(socket, bytes, || service.setting_changes());
                    })?;
                return Ok(());
            }
            // Only one body can occupy the package budget. Reads do not
            // acquire this lock and remain available during a slow upload.
            let _upload = if request.method != "GET" || request.length > 0 {
                match upload.try_lock() {
                    Ok(guard) => Some(guard),
                    // An upload that panicked ended with its request.
                    Err(std::sync::TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
                    Err(std::sync::TryLockError::WouldBlock) => {
                        stream.write_all(
                            &Response::error(409, "An upload is already active").to_bytes(),
                        )?;
                        return stream.shutdown(Shutdown::Both);
                    }
                }
            } else {
                None
            };
            // The head may have brought the first bytes of the body.
            let mut body = bytes.split_off(head_end(&bytes).unwrap_or(bytes.len()));
            body.truncate(request.length);
            let rest = request.length - body.len();
            let card = Card::new(grkit_core::card::LINUX_CARD);
            let env = Env {
                files: &System,
                connection: &connection,
                card: &card,
                device: &device,
            };
            // Answer a software request that cannot succeed before its package
            // arrives: the camera then neither receives nor hashes it.
            let continues = request
                .headers
                .get("expect")
                .is_some_and(|value| value.eq_ignore_ascii_case("100-continue"));
            if rest > 0 {
                let early = match lock_card(card_access) {
                    Ok(_guard) => service.precheck(&request, &env),
                    Err(busy) => Some(busy),
                };
                if let Some(reply) = early {
                    reader.get_mut().write_all(&reply.to_bytes())?;
                    if !continues {
                        // A client that sends without waiting reads the answer
                        // after its upload; drop the bytes instead of resetting.
                        io::copy(
                            &mut Read::by_ref(&mut reader).take(rest as u64),
                            &mut io::sink(),
                        )?;
                    }
                    return reader.get_mut().shutdown(Shutdown::Both);
                }
                if continues {
                    reader
                        .get_mut()
                        .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
                }
            }
            Read::by_ref(&mut reader)
                .take(rest as u64)
                .read_to_end(&mut body)?;
            if body.len() < request.length {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            if request.path == "/v1/ping" {
                service.handle(&request, &body, &env)
            } else {
                // The existing update journal cannot be read midway through
                // staging, nor can cancellation race its writer.
                match lock_card(card_access) {
                    Ok(_guard) => service.handle(&request, &body, &env),
                    Err(busy) => busy,
                }
            }
        }
    };
    stream.write_all(&response.to_bytes())?;
    stream.shutdown(Shutdown::Both)
}

/// The camera, as its own HTTP API on port 80 describes it, and the free
/// space of the card.
fn device() -> anyhow::Result<Device> {
    use anyhow::Context;
    let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], 80).into(), PEER_TIMEOUT)
        .context("The camera's Wi-Fi API is not ready")?;
    stream.set_read_timeout(Some(PEER_TIMEOUT))?;
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
            answer.len() <= grkit_serve::MAX_FILE,
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
    Some(u64::from(stats.f_bavail) * u64::from(stats.f_frsize))
}

#[cfg(not(target_os = "linux"))]
fn free_bytes(_path: &str) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiving_upload_excludes_bodyless_cancel() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let upload = Arc::new(Mutex::new(()));
        let receiving = upload.lock().unwrap();
        let worker_upload = Arc::clone(&upload);
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            serve(
                &mut stream,
                &Mutex::new(()),
                &worker_upload,
                &Arc::new(Service::new("test".into())),
            )
            .unwrap();
        });
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        write!(
            client,
            "POST /v1/update/cancel HTTP/1.1\r\nHost: {address}\r\n\r\n"
        )
        .unwrap();
        let mut answer = String::new();
        client.read_to_string(&mut answer).unwrap();
        assert!(answer.starts_with("HTTP/1.1 409"), "{answer}");
        worker.join().unwrap();
        drop(receiving);
    }

    #[test]
    fn incomplete_upload_does_not_block_ping() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let card = Arc::new(Mutex::new(()));
        let upload = Arc::new(Mutex::new(()));
        let server = thread::spawn(move || {
            let mut workers = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let card = Arc::clone(&card);
                let upload = Arc::clone(&upload);
                workers.push(thread::spawn(move || {
                    serve(
                        &mut stream,
                        &card,
                        &upload,
                        &Arc::new(Service::new("test".into())),
                    )
                }));
            }
            for worker in workers {
                let _ = worker.join().unwrap();
            }
        });
        let mut slow = TcpStream::connect(address).unwrap();
        write!(slow, "PUT /v1/update HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/zip\r\nContent-Length: 100\r\n\r\nx").unwrap();
        let mut ping = TcpStream::connect(address).unwrap();
        ping.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        write!(ping, "GET /v1/ping HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
        let mut answer = String::new();
        ping.read_to_string(&mut answer).unwrap();
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        slow.shutdown(Shutdown::Both).unwrap();
        server.join().unwrap();
    }
}
