//! The error body that each v1 resource answers.

use serde::Serialize;

/// An RFC 9457 response body. The HTTP response uses `application/problem+json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Problem {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub title: &'static str,
    pub status: u16,
    pub code: &'static str,
    pub detail: String,
}

impl Problem {
    pub fn new(status: u16, code: &'static str, detail: impl Into<String>) -> Self {
        let title = crate::reason(status);
        Self {
            kind: "about:blank",
            title,
            status,
            code,
            detail: detail.into(),
        }
    }
}

/// The form of the ID of a software request on the card.
pub fn valid_request_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
