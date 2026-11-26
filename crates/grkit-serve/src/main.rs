//! The GRKit service (`grkit_serve`): `SERVE.BIN` on the card.
//!
//! `SCAN.SH` starts the service from the card, because the camera's `/tmp`
//! does not run programs on every camera (GR IV mounts it without exec). The
//! service then runs a copy of itself from RAM (`run_from_ram`), so that no
//! file on the card stays open while it runs.

use grkit_serve::{
    api::Service,
    connection,
    transport::{drain, ConnectionSlot, IDLE_TIMEOUT, TRANSFER_TIMEOUT},
    update::Card,
    PORT,
};
#[cfg(target_os = "linux")]
use std::{
    fs::File,
    io::Write,
    os::{
        fd::{FromRawFd, IntoRawFd},
        unix::process::CommandExt,
    },
    process::Command,
};
use std::{
    io::{self, Read},
    net::TcpListener,
    process::ExitCode,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    thread,
    time::Duration,
};

/// The pause after a failed `accept`, so that a lasting failure (for example,
/// no free file descriptor) does not keep the CPU busy.
const ACCEPT_PAUSE: Duration = Duration::from_millis(500);
/// The argument of the copy that runs from RAM.
const FROM_RAM: &str = grkit_core::runtime::SERVE_FROM_RAM;
/// The service marks itself as running in its mailbox as it starts, so that
/// the resident loader of the camera stops starting it
/// (`grkit_core::runtime::SERVE_POLLS`). GRKit can still be starting: the
/// service tries again at this interval, as long as the loader would start
/// it.
const ANNOUNCE_PAUSE: Duration = Duration::from_secs(2);
const ANNOUNCE_TRIES: u32 = grkit_core::runtime::SERVE_POLLS;

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
    // One card for the service: it keeps the summary of its files between
    // requests (`Card::forget_summary`).
    let card = Arc::new(Card::new(grkit_core::card::LINUX_CARD));
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
        // A connection beyond the slots waits in the listen queue until a
        // request ends; one closed without an answer leaves its client
        // unsure whether a write ran.
        let Some(slot) = ConnectionSlot::wait(&connections, || stop_pending(stop)) else {
            break;
        };
        if stop_requested(&listener, stop) {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let service = Arc::clone(&service);
                let card = Arc::clone(&card);
                let card_access = Arc::clone(&card_access);
                let upload = Arc::clone(&upload);
                let _ = thread::Builder::new()
                    .name("grkit-http".into())
                    .spawn(move || {
                        let _slot = slot;
                        let _ =
                            connection::serve(&mut stream, &card, &card_access, &upload, &service);
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
        std::time::Instant::now() + TRANSFER_TIMEOUT + IDLE_TIMEOUT,
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

/// Whether a stop waits in its pipe, without waiting for one.
#[cfg(target_os = "linux")]
fn stop_pending(stop: Option<i32>) -> bool {
    let Some(stop) = stop else {
        return false;
    };
    let mut fd = libc::pollfd {
        fd: stop,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the descriptor stays open for the call; a zero timeout
    // returns at once.
    unsafe { libc::poll(&mut fd, 1, 0) > 0 && fd.revents & libc::POLLIN != 0 }
}
#[cfg(not(target_os = "linux"))]
fn stop_pending(_stop: Option<i32>) -> bool {
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
