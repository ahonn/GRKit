//! Reads with a size limit, for files that a person or another program can
//! make larger than the reader takes.

use std::io::{self, Read};

/// The bytes of `reader`, at most `limit`. A longer input is an
/// `InvalidData` error, read only to `limit + 1` bytes.
pub fn read_to_limit(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the input exceeds its size limit",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_input_over_its_limit_is_an_error() {
        assert_eq!(super::read_to_limit(&b"abc"[..], 3).unwrap(), b"abc");
        assert!(super::read_to_limit(&b"abcd"[..], 3).is_err());
    }
}
