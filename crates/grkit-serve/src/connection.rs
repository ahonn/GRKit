//! One connection of the service: it reads the request, admits it
//! (`transport::admit`), keeps the change stream open (`changes`), receives
//! a body under its limits, and answers with the card held
//! (`api::Service`). `main` accepts the connections.

use crate::{
    api::{Reply, Service},
    head_end, parse,
    protocol::Problem,
    read_head,
    route::Route,
    transport::{admit, DeadlineReader, IDLE_TIMEOUT},
    update::{Card, MAX_PACKAGE},
    Connection, Env, System,
};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

/// Maximum idle time for a peer.
const PEER_TIMEOUT: Duration = IDLE_TIMEOUT;
/// How long a request waits while another one reads or writes the card.
const CARD_WAIT: Duration = Duration::from_secs(3);

/// Waits for the card for at most `CARD_WAIT`. Concurrent requests of one
/// client (for example a page that reads several resources) then queue
/// instead of failing; a long write answers a retryable 503.
fn lock_card(card_access: &Mutex<()>) -> Result<MutexGuard<'_, ()>, Reply> {
    let deadline = Instant::now() + CARD_WAIT;
    loop {
        if let Some(guard) = crate::sync::try_lock(card_access) {
            return Ok(guard);
        }
        if Instant::now() >= deadline {
            return Err(Reply::problem(Problem::new(
                503,
                "card_busy",
                "Another request is using the card. Try again.",
            )));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Answers the request of `stream` and closes it.
pub fn serve(
    stream: &mut TcpStream,
    card: &Card,
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
        None => Reply::error(400, "bad request"),
        Some(request) if request.length > MAX_PACKAGE => {
            Reply::error(413, "the package is too large")
        }
        Some(request) => {
            if let Err(response) = admit(&request, local) {
                stream.write_all(&response.to_bytes())?;
                return stream.shutdown(Shutdown::Both);
            }
            let route = Route::parse(&request.path);
            if route == Some(Route::Changes) && request.method == "GET" {
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
                match crate::sync::try_lock(upload) {
                    Some(guard) => Some(guard),
                    None => {
                        stream.write_all(
                            &Reply::error(409, "An upload is already active").to_bytes(),
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
            // One allocation of the declared length: growing the buffer
            // while it reads would take up to twice the package.
            body.reserve_exact(rest);
            let env = Env {
                files: &System,
                connection: &connection,
                card,
                device: &crate::camera_api::device,
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
            if route == Some(Route::Ping) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

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
                &Card::new(grkit_core::card::LINUX_CARD),
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
        let card_access = Arc::new(Mutex::new(()));
        let upload = Arc::new(Mutex::new(()));
        let server = thread::spawn(move || {
            let mut workers = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let card_access = Arc::clone(&card_access);
                let upload = Arc::clone(&upload);
                workers.push(thread::spawn(move || {
                    serve(
                        &mut stream,
                        &Card::new(grkit_core::card::LINUX_CARD),
                        &card_access,
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
