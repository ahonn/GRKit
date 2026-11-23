//! Decode null-padded Wi-Fi credential bytes from GATT characteristics.

/// Strip trailing NUL padding from a UTF-8 credential buffer.
pub fn decode_cred(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end])
        .trim_end_matches('\0')
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_nul_padding() {
        let mut buf = [0u8; 33];
        buf[..5].copy_from_slice(b"GR_AP");
        assert_eq!(decode_cred(&buf), "GR_AP");
    }

    #[test]
    fn empty_when_all_nul() {
        assert_eq!(decode_cred(&[0u8; 63]), "");
    }

    #[test]
    fn keeps_interior_spaces() {
        assert_eq!(decode_cred(b"GR III\0\0"), "GR III");
    }
}
