//! The picture of Shutdown Screen: `GRKIT/GOODBYE.JPG` on the card,
//! which the camera shows when it turns off, in place of the picture of the
//! firmware (`runtime/goodbye.h`). The firmware decodes the file as its own
//! picture, so the file has the frame of those pictures: a baseline JPEG of
//! 720 by 480 pixels with 4:2:0 chroma.

use crate::jpeg;
use anyhow::{bail, ensure, Result};

/// The size of the picture, which the screen shows whole.
pub const WIDTH: usize = 720;
pub const HEIGHT: usize = 480;
/// The most bytes of the file: the buffer that the firmware reads it into.
pub const MAX_BYTES: usize = WIDTH * HEIGHT * 2;
/// The file on the card, in the data folder.
pub const FILE: &str = concat!("GRKIT", "/GOODBYE.JPG");
/// The qualities that `encode` tries, best first.
const QUALITIES: [u8; 4] = [92, 85, 75, 60];

/// Encode `rgb`, the 8-bit RGB pixels of a picture of `WIDTH` by `HEIGHT` in
/// row order, as the file: at the best quality that fits `MAX_BYTES`.
pub fn encode(rgb: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        rgb.len() == WIDTH * HEIGHT * 3,
        "The shutdown picture must be {WIDTH} by {HEIGHT} pixels"
    );
    for quality in QUALITIES {
        let bytes = jpeg::encode(rgb, WIDTH, HEIGHT, quality)?;
        if bytes.len() <= MAX_BYTES {
            return Ok(bytes);
        }
    }
    bail!("The shutdown picture does not fit the file")
}

/// Whether the camera shows `bytes` as the file. A file that it does not
/// show changes nothing: the camera shows the picture of the firmware.
pub fn usable(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() <= MAX_BYTES
        && jpeg::baseline_frame(bytes)
            == Some((WIDTH as u16, HEIGHT as u16, vec![0x22, 0x11, 0x11]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_one_that_the_camera_shows() {
        // Noise is the picture that compresses least.
        let mut state = 1u32;
        let noise: Vec<u8> = (0..WIDTH * HEIGHT * 3)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        for rgb in [vec![128; WIDTH * HEIGHT * 3], noise] {
            let bytes = encode(&rgb).unwrap();
            assert!(usable(&bytes));
            // The camera reads the frame header from the first bytes of the
            // file (GOODBYE_HEAD).
            assert!(bytes.windows(2).position(|w| w == [0xff, 0xc0]).unwrap() + 19 <= 184);
        }
        assert!(encode(&[0; 16 * 16 * 3]).is_err());
        assert!(!usable(
            &jpeg::encode(&[0; 16 * 16 * 3], 16, 16, 90).unwrap()
        ));
    }

    /// The camera and the tools agree on the file and its limits.
    #[test]
    fn the_runtime_names_the_same_file() {
        let runtime = include_str!("../../../runtime/goodbye.h");
        assert!(runtime.contains(&format!("\"C:\\\\{}\"", FILE.replace('/', "\\\\"))));
        assert!(runtime.contains(&format!("#define GOODBYE_WIDTH {WIDTH}U")));
        assert!(runtime.contains(&format!("#define GOODBYE_HEIGHT {HEIGHT}U")));
        assert!(runtime.contains("#define GOODBYE_BYTES_MAX (GOODBYE_WIDTH*GOODBYE_HEIGHT*2U)"));
        assert!(runtime.contains("#define GOODBYE_HEAD 184U"));
    }
}
