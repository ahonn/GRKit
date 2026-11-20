//! The change stream of the camera service (`GET /v1/changes`): the
//! WebSocket frames, the handshake, and the events.
//!
//! An event names a kind of change and carries no data: the app reads that
//! resource again over HTTP. The stream keeps no history, so after a new
//! connection the app reads what it shows again. Shapes match `grkit-serve`
//! (`changes.rs`). Kept free of NSH so unit tests run without an engine.

use std::time::Duration;

use serde::Deserialize;

/// The path of the change stream on the camera service.
pub const PATH: &str = "/v1/changes";
/// A frame within this time shows that the camera service still answers,
/// so the periodic HTTP probe can wait. The service pings every 15 s.
pub const FRESH: Duration = Duration::from_secs(20);
/// A stream that gets no frame for this long is gone: the service pings
/// every 15 s and closes after 30 s without a Pong.
pub const SILENCE_LIMIT: Duration = Duration::from_secs(40);
/// The largest frame the app accepts. The service sends frames below 1 KiB.
pub const MAX_PAYLOAD: usize = 4096;
/// The largest handshake answer the app reads.
pub const MAX_HEAD: usize = 4096;

/// A kind of change, as the `changed` field of an event names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// A phone request changed the Looks.
    Looks,
    /// An update request was staged or cancelled.
    Software,
    /// A GRKit setting changed on the camera (menu, ADJ, recipe, mode dial).
    Settings,
}

/// One text frame of the stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    /// The first frame after the handshake.
    Hello {
        boot_id: String,
    },
    Changed(ChangeKind),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum WireEvent {
    Hello {
        #[serde(rename = "bootId")]
        boot_id: String,
    },
    Changed {
        changed: String,
    },
}

/// Parse a text frame. An event of another type or kind is `None`: a later
/// service may send more.
pub fn parse_event(text: &str) -> Option<ChangeEvent> {
    match serde_json::from_str(text).ok()? {
        WireEvent::Hello { boot_id } => Some(ChangeEvent::Hello { boot_id }),
        WireEvent::Changed { changed } => Some(ChangeEvent::Changed(match changed.as_str() {
            "looks" => ChangeKind::Looks,
            "software" => ChangeKind::Software,
            "settings" => ChangeKind::Settings,
            _ => return None,
        })),
    }
}

/// How long to wait before the next connection after `failures` streams in a
/// row ended without a hello: 2 s, 5 s, then 10 s.
pub fn retry_delay(failures: u32) -> Duration {
    Duration::from_secs(match failures {
        0 => 2,
        1 => 5,
        _ => 10,
    })
}

/// The `host:port` of the HTTP service of `base` (`http://host[:port]`), on
/// which the change stream is. None for another scheme.
pub fn authority(base: &str) -> Option<String> {
    let authority = base.strip_prefix("http://")?.split('/').next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let has_port = match authority.rsplit_once(']') {
        // An IPv6 literal: `[::1]` or `[::1]:8080`.
        Some((_, rest)) => rest.starts_with(':'),
        None => authority.contains(':'),
    };
    Some(if has_port {
        authority.to_owned()
    } else {
        format!("{authority}:80")
    })
}

/// The upgrade request. `host` is the address and port that the phone
/// connected to: the service refuses another `Host`, and any `Origin`.
pub fn handshake_request(host: &str, key: &str) -> String {
    format!(
        "GET {PATH} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    )
}

/// The length of the answer head when `bytes` holds all of it.
pub fn head_length(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// Check that the answer head switches to the WebSocket protocol.
pub fn check_handshake(head: &[u8]) -> Result<(), String> {
    let head = std::str::from_utf8(head).map_err(|_| "The handshake answer is not text")?;
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap_or_default();
    if status.split(' ').nth(1) != Some("101") {
        return Err(format!("The camera refused the change stream: {status}"));
    }
    let upgraded = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("upgrade")
                && value.trim().eq_ignore_ascii_case("websocket")
        })
    });
    if upgraded {
        Ok(())
    } else {
        Err("The handshake answer does not upgrade to WebSocket".into())
    }
}

/// Standard base64 with padding, for `Sec-WebSocket-Key`.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0u32, |value, (i, byte)| {
            value | u32::from(*byte) << (16 - 8 * i)
        });
        for i in 0..4 {
            if i <= chunk.len() {
                text.push(ALPHABET[(value >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// The kinds of frame the app handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

impl Opcode {
    fn code(self) -> u8 {
        match self {
            Opcode::Text => 0x1,
            Opcode::Binary => 0x2,
            Opcode::Close => 0x8,
            Opcode::Ping => 0x9,
            Opcode::Pong => 0xA,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub opcode: Opcode,
    pub payload: Vec<u8>,
}

/// Why a frame of the service ends the connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// The service sends each message in one frame.
    Fragmented,
    /// A frame of the service is never masked (RFC 6455, 5.1).
    Masked,
    /// A reserved bit is set: no extension was agreed.
    Reserved,
    UnknownOpcode(u8),
    TooLarge,
}

/// Decode the first frame of `bytes`. `Ok(None)` while the frame is not
/// complete; otherwise the frame and the count of bytes it took.
pub fn decode_frame(bytes: &[u8]) -> Result<Option<(Frame, usize)>, FrameError> {
    let [first, second, ..] = *bytes else {
        return Ok(None);
    };
    if first & 0x70 != 0 {
        return Err(FrameError::Reserved);
    }
    if second & 0x80 != 0 {
        return Err(FrameError::Masked);
    }
    let opcode = match first & 0x0F {
        0x1 => Opcode::Text,
        0x2 => Opcode::Binary,
        0x8 => Opcode::Close,
        0x9 => Opcode::Ping,
        0xA => Opcode::Pong,
        0x0 => return Err(FrameError::Fragmented),
        other => return Err(FrameError::UnknownOpcode(other)),
    };
    if first & 0x80 == 0 {
        return Err(FrameError::Fragmented);
    }
    let (length, start) = match second & 0x7F {
        126 => match bytes.get(2..4) {
            Some(field) => (u64::from(u16::from_be_bytes([field[0], field[1]])), 4),
            None => return Ok(None),
        },
        127 => match bytes.get(2..10) {
            Some(field) => (u64::from_be_bytes(field.try_into().expect("8 bytes")), 10),
            None => return Ok(None),
        },
        length => (u64::from(length), 2),
    };
    let control = matches!(opcode, Opcode::Close | Opcode::Ping | Opcode::Pong);
    if length > MAX_PAYLOAD as u64 || (control && length > 125) {
        return Err(FrameError::TooLarge);
    }
    let end = start + length as usize;
    let Some(payload) = bytes.get(start..end) else {
        return Ok(None);
    };
    Ok(Some((
        Frame {
            opcode,
            payload: payload.to_vec(),
        },
        end,
    )))
}

/// Encode one frame of the app, masked with `mask` as RFC 6455 requires of a
/// client.
pub fn encode_frame(opcode: Opcode, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.push(0x80 | opcode.code());
    match payload.len() {
        length @ 0..=125 => frame.push(0x80 | length as u8),
        length @ 126..=0xFFFF => {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(length as u16).to_be_bytes());
        }
        length => {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(length as u64).to_be_bytes());
        }
    }
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ mask[i % 4]),
    );
    frame
}

/// The status code of a Close frame, which the app sends back.
pub fn close_code(payload: &[u8]) -> Option<u16> {
    payload
        .get(..2)
        .map(|code| u16::from_be_bytes([code[0], code[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unmask a frame of the app, as the service does.
    fn unmask(frame: &[u8]) -> (u8, Vec<u8>) {
        assert_eq!(frame[1] & 0x80, 0x80, "a frame of the app is masked");
        let (length, start) = match frame[1] & 0x7F {
            126 => (u16::from_be_bytes([frame[2], frame[3]]) as usize, 4),
            length => (length as usize, 2),
        };
        let mask = &frame[start..start + 4];
        let payload = frame[start + 4..]
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ mask[i % 4])
            .collect::<Vec<_>>();
        assert_eq!(payload.len(), length);
        (frame[0], payload)
    }

    #[test]
    fn parses_the_events_of_the_service() {
        assert_eq!(
            parse_event(r#"{"type":"hello","bootId":"boot-1"}"#),
            Some(ChangeEvent::Hello {
                boot_id: "boot-1".into()
            })
        );
        for (name, kind) in [
            ("looks", ChangeKind::Looks),
            ("software", ChangeKind::Software),
            ("settings", ChangeKind::Settings),
        ] {
            assert_eq!(
                parse_event(&format!(r#"{{"type":"changed","changed":"{name}"}}"#)),
                Some(ChangeEvent::Changed(kind))
            );
        }
        assert_eq!(parse_event(r#"{"type":"changed","changed":"other"}"#), None);
        assert_eq!(parse_event(r#"{"type":"later"}"#), None);
        assert_eq!(parse_event("not json"), None);
    }

    #[test]
    fn decodes_frames_of_the_service() {
        let text = br#"{"type":"changed","changed":"looks"}"#;
        let mut bytes = vec![0x81, text.len() as u8];
        bytes.extend_from_slice(text);
        bytes.extend_from_slice(&[0x89, 5]);
        bytes.extend_from_slice(b"grkit");
        let (frame, used) = decode_frame(&bytes).unwrap().unwrap();
        assert_eq!(frame.opcode, Opcode::Text);
        assert_eq!(frame.payload, text);
        let (ping, rest) = decode_frame(&bytes[used..]).unwrap().unwrap();
        assert_eq!(ping.opcode, Opcode::Ping);
        assert_eq!(ping.payload, b"grkit");
        assert_eq!(used + rest, bytes.len());
        // A frame that is not complete yet waits for more bytes.
        for end in 0..used {
            assert_eq!(decode_frame(&bytes[..end]), Ok(None));
        }
        // An extended length.
        let long = vec![b'x'; 300];
        let mut bytes = vec![0x81, 126, 1, 44];
        bytes.extend_from_slice(&long);
        let (frame, used) = decode_frame(&bytes).unwrap().unwrap();
        assert_eq!((frame.payload.len(), used), (300, 304));
        assert_eq!(decode_frame(&bytes[..100]), Ok(None));
    }

    #[test]
    fn refuses_frames_the_service_never_sends() {
        // FIN=0, a continuation, a mask, a reserved bit.
        assert_eq!(decode_frame(&[0x01, 0]), Err(FrameError::Fragmented));
        assert_eq!(decode_frame(&[0x80, 0]), Err(FrameError::Fragmented));
        assert_eq!(
            decode_frame(&[0x81, 0x80, 0, 0, 0, 0]),
            Err(FrameError::Masked)
        );
        assert_eq!(decode_frame(&[0xC1, 0]), Err(FrameError::Reserved));
        assert_eq!(decode_frame(&[0x83, 0]), Err(FrameError::UnknownOpcode(3)));
        // Too large: a data frame above the limit, a control frame above 125.
        assert_eq!(
            decode_frame(&[0x81, 127, 0, 0, 0, 0, 0, 1, 0, 0]),
            Err(FrameError::TooLarge)
        );
        assert_eq!(
            decode_frame(&[0x89, 126, 0, 126]),
            Err(FrameError::TooLarge)
        );
    }

    #[test]
    fn encodes_masked_frames_of_the_app() {
        let mask = [0x37, 0xFA, 0x21, 0x3D];
        // The masked "Hello" of RFC 6455, 5.7.
        assert_eq!(
            encode_frame(Opcode::Text, b"Hello", mask),
            [
                0x81, 0x85, 0x37, 0xFA, 0x21, 0x3D, 0x7F, 0x9F, 0x4D, 0x51, 0x58
            ]
        );
        let (first, payload) = unmask(&encode_frame(Opcode::Pong, b"grkit", [1, 2, 3, 4]));
        assert_eq!((first, payload.as_slice()), (0x8A, b"grkit".as_slice()));
        let (first, payload) = unmask(&encode_frame(Opcode::Close, &[], [9, 9, 9, 9]));
        assert_eq!((first, payload.len()), (0x88, 0));
        let long = vec![7u8; 200];
        let (_, payload) = unmask(&encode_frame(Opcode::Binary, &long, [5, 6, 7, 8]));
        assert_eq!(payload, long);
        assert_eq!(close_code(&[0x03, 0xE8, b'x']), Some(1000));
        assert_eq!(close_code(&[]), None);
    }

    #[test]
    fn builds_and_checks_the_handshake() {
        let request = handshake_request("192.168.0.1:8080", "dGhlIHNhbXBsZSBub25jZQ==");
        assert!(request.starts_with("GET /v1/changes HTTP/1.1\r\n"));
        assert!(request.contains("\r\nHost: 192.168.0.1:8080\r\n"));
        assert!(request.contains("\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"));
        assert!(request.contains("\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"));
        assert!(request.contains("\r\nSec-WebSocket-Version: 13\r\n"));
        assert!(!request.to_ascii_lowercase().contains("origin"));
        assert!(request.ends_with("\r\n\r\n"));

        let answer = b"HTTP/1.1 101 Switching Protocols\r\nupgrade: websocket\r\nconnection: Upgrade\r\n\r\n\x81";
        let length = head_length(answer).unwrap();
        assert_eq!(answer[length], 0x81);
        assert_eq!(check_handshake(&answer[..length]), Ok(()));
        assert_eq!(head_length(b"HTTP/1.1 101 Switching\r\n"), None);
        assert!(check_handshake(b"HTTP/1.1 403 Forbidden\r\n\r\n").is_err());
        assert!(check_handshake(b"HTTP/1.1 101 Switching Protocols\r\n\r\n").is_err());
    }

    #[test]
    fn encodes_base64() {
        for (bytes, text) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
            ("the sample nonce", "dGhlIHNhbXBsZSBub25jZQ=="),
        ] {
            assert_eq!(base64(bytes.as_bytes()), text);
        }
    }

    #[test]
    fn finds_the_authority_of_the_service() {
        assert_eq!(
            authority("http://192.168.0.1:8080").as_deref(),
            Some("192.168.0.1:8080")
        );
        assert_eq!(
            authority("http://127.0.0.1:8080/").as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            authority("http://camera.test").as_deref(),
            Some("camera.test:80")
        );
        assert_eq!(authority("http://[::1]").as_deref(), Some("[::1]:80"));
        assert_eq!(
            authority("http://[::1]:8080").as_deref(),
            Some("[::1]:8080")
        );
        assert_eq!(authority("https://192.168.0.1:8080"), None);
        assert_eq!(authority("http://"), None);
    }

    #[test]
    fn retries_slower_after_each_failure() {
        let delays: Vec<u64> = (0..5).map(|n| retry_delay(n).as_secs()).collect();
        assert_eq!(delays, [2, 5, 10, 10, 10]);
    }
}
