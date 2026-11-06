//! The firmware update file of GR III, GR IIIx, and GR IV.
//!
//! The file has a 128-byte header, the encoded payload, padding to a word,
//! metadata records, and a footer of six words: the project, a marker, the
//! type, the encoded size, the decoded size, and a check word. The 32-bit
//! little-endian word sum of the file is 0, and so is that of the decoded
//! payload. The last word of each is its compensation word.
//!
//! The encoded payload is a list of frames, then a zero tag. A frame starts
//! with a big-endian tag: bit 15 marks a literal frame, and bits 14 to 0 give
//! the encoded length. A coded frame has groups of 16 flags, most significant
//! first, each followed by its items: flag 0 is one literal byte, and flag 1
//! is a back reference of two bytes (5 bits of the distance high part, 3 bits
//! of count, then 8 bits of the distance). Count 7 continues with bytes until
//! one is not 255. A back reference copies count + 3 bytes, and distance 0
//! ends the frame.
//!
//! The decoded payload is a list of sections: an 8-byte name, two half
//! words, the size, then the body. `RES` is the last section that the
//! firmware names.

use anyhow::{bail, ensure, Context, Result};
use std::ops::Range;

const HEADER: usize = 128;
const FOOTER: usize = 24;
const MARKER: u32 = 0xa55a_5aa5;
/// The most bytes of a literal frame that the file uses.
const LITERAL_FRAME: usize = 0x6000;

/// The 32-bit little-endian word sum of `data`, a whole number of words.
pub fn sum32(data: &[u8]) -> u32 {
    data.chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .fold(0, u32::wrapping_add)
}

/// Sets the last word of `data` so that its word sum is 0.
fn compensate(data: &mut [u8]) {
    let end = data.len() - 4;
    let rest = sum32(&data[..end]);
    data[end..].copy_from_slice(&rest.wrapping_neg().to_le_bytes());
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

/// One frame of the encoded payload.
#[derive(Clone, Debug)]
struct Frame {
    /// The frame in the encoded payload, its tag included.
    encoded: Range<usize>,
    literal: bool,
    /// The bytes of the frame in the decoded payload.
    decoded: Range<usize>,
}

/// Decodes the body of one frame onto `out`, which holds the bytes of the
/// earlier frames.
fn decode_frame(body: &[u8], literal: bool, out: &mut Vec<u8>) -> Result<()> {
    if literal {
        out.extend_from_slice(body);
        return Ok(());
    }
    let byte = |pos: usize| body.get(pos).copied().context("A frame ends early");
    let mut pos = 0;
    while pos < body.len() {
        let flags = u16::from_be_bytes([byte(pos)?, byte(pos + 1)?]);
        pos += 2;
        for bit in (0..16).rev() {
            if pos == body.len() {
                break;
            }
            if flags >> bit & 1 == 0 {
                out.push(byte(pos)?);
                pos += 1;
                continue;
            }
            let (high, low) = (byte(pos)?, byte(pos + 1)?);
            pos += 2;
            let distance = (usize::from(high & 0xf8) << 5) | usize::from(low);
            let mut count = usize::from(high & 7);
            if count == 7 {
                loop {
                    let more = byte(pos)?;
                    pos += 1;
                    count += usize::from(more);
                    if more != 255 {
                        break;
                    }
                }
            }
            if distance == 0 {
                return Ok(());
            }
            ensure!(
                distance <= out.len(),
                "A back reference starts before the payload"
            );
            for _ in 0..count + 3 {
                out.push(out[out.len() - distance]);
            }
        }
    }
    Ok(())
}

/// The frames of an encoded payload and its decoded bytes. The payload must
/// end with its zero tag.
fn decode(payload: &[u8]) -> Result<(Vec<Frame>, Vec<u8>)> {
    let (mut pos, mut out, mut frames) = (0, Vec::new(), Vec::new());
    loop {
        let tag = payload
            .get(pos..pos + 2)
            .map(|tag| u16::from_be_bytes([tag[0], tag[1]]))
            .context("The payload has no end")?;
        if tag == 0 {
            ensure!(
                pos + 2 == payload.len(),
                "The payload has bytes after its end"
            );
            return Ok((frames, out));
        }
        let end = pos + 2 + usize::from(tag & 0x7fff);
        let body = payload
            .get(pos + 2..end)
            .context("A frame passes the payload")?;
        let start = out.len();
        let literal = tag & 0x8000 != 0;
        decode_frame(body, literal, &mut out)?;
        frames.push(Frame {
            encoded: pos..end,
            literal,
            decoded: start..out.len(),
        });
        pos = end;
    }
}

/// An update file, decoded.
pub struct Update<'a> {
    file: &'a [u8],
    footer: [u32; 6],
    payload: &'a [u8],
    metadata: Range<usize>,
    frames: Vec<Frame>,
    /// The decoded payload.
    pub decoded: Vec<u8>,
}

impl<'a> Update<'a> {
    /// Decodes `file` and checks its sums and sizes.
    pub fn read(file: &'a [u8]) -> Result<Self> {
        ensure!(
            file.len() > HEADER + FOOTER && file.len().is_multiple_of(4) && sum32(file) == 0,
            "Not a firmware update file: its size or word sum is not valid"
        );
        let footer: [u32; 6] = std::array::from_fn(|i| word(file, file.len() - FOOTER + 4 * i));
        ensure!(footer[1] == MARKER, "Not a firmware update file: no footer");
        let (encoded, decoded) = (footer[3] as usize, footer[4] as usize);
        let metadata_start = (HEADER + encoded).next_multiple_of(4);
        ensure!(
            metadata_start <= file.len() - FOOTER,
            "The footer gives an encoded size past the file"
        );
        let payload = &file[HEADER..HEADER + encoded];
        let (frames, bytes) = decode(payload)?;
        ensure!(
            bytes.len() == decoded && decoded.is_multiple_of(4) && sum32(&bytes) == 0,
            "The payload does not decode as the footer declares"
        );
        Ok(Self {
            file,
            footer,
            payload,
            metadata: metadata_start..file.len() - FOOTER,
            frames,
            decoded: bytes,
        })
    }

    /// The bytes of the section named `name` in the decoded payload.
    pub fn section(&self, name: &str) -> Result<Range<usize>> {
        let mut pos = 0;
        while pos + 16 <= self.decoded.len() {
            let tag = &self.decoded[pos..pos + 8];
            let found = tag
                .iter()
                .position(|&b| b == 0)
                .map_or(tag, |end| &tag[..end]);
            let size = word(&self.decoded, pos + 12) as usize;
            let body = pos + 16..pos + 16 + size;
            ensure!(
                body.end <= self.decoded.len(),
                "A section passes the payload"
            );
            if found == name.as_bytes() {
                return Ok(body);
            }
            if found == b"RES" {
                break;
            }
            pos = body.end;
        }
        bail!("The payload has no {name} section")
    }

    /// The update file with the decoded payload `target`, which has the
    /// size of the decoded payload. The last word of `target` becomes its
    /// compensation word. Each frame that still decodes to its part of the
    /// target stays as it is; the others become literal frames.
    pub fn encode(&self, target: &[u8]) -> Result<Vec<u8>> {
        ensure!(
            target.len() == self.decoded.len(),
            "The payload changes size"
        );
        let mut target = target.to_vec();
        compensate(&mut target);
        let (mut encoded, mut out) = (Vec::with_capacity(self.payload.len()), Vec::new());
        for frame in &self.frames {
            let want = &target[frame.decoded.clone()];
            let start = out.len();
            let body = &self.payload[frame.encoded.start + 2..frame.encoded.end];
            let same = decode_frame(body, frame.literal, &mut out).is_ok() && out[start..] == *want;
            if same {
                encoded.extend_from_slice(&self.payload[frame.encoded.clone()]);
                continue;
            }
            out.truncate(start);
            out.extend_from_slice(want);
            for chunk in want.chunks(LITERAL_FRAME) {
                encoded.extend_from_slice(&(0x8000 | chunk.len() as u16).to_be_bytes());
                encoded.extend_from_slice(chunk);
            }
        }
        encoded.extend_from_slice(&[0, 0]);
        let (_, again) = decode(&encoded)?;
        ensure!(
            again == target,
            "The new payload does not decode to the target"
        );
        let mut file = self.file[..HEADER].to_vec();
        file.extend_from_slice(&encoded);
        file.resize(file.len().next_multiple_of(4), 0);
        file.extend_from_slice(&self.file[self.metadata.clone()]);
        let footer = [
            self.footer[0],
            self.footer[1],
            self.footer[2],
            u32::try_from(encoded.len())?,
            u32::try_from(target.len())?,
            0,
        ];
        for value in footer {
            file.extend_from_slice(&value.to_le_bytes());
        }
        compensate(&mut file);
        Ok(file)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A coded frame of literal bytes only.
    fn coded(bytes: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        for group in bytes.chunks(16) {
            body.extend_from_slice(&0u16.to_be_bytes());
            body.extend_from_slice(group);
        }
        let mut frame = (body.len() as u16).to_be_bytes().to_vec();
        frame.extend(body);
        frame
    }

    /// An update file with two sections, `RTOS` and `RES`: a coded frame of
    /// the first 20 bytes, then a literal frame.
    pub(crate) fn sample(rtos: &[u8]) -> Vec<u8> {
        let mut decoded = Vec::new();
        for (name, body) in [(&b"RTOS"[..], rtos), (b"RES", &[0u8; 8][..])] {
            let mut tag = [0u8; 8];
            tag[..name.len()].copy_from_slice(name);
            decoded.extend_from_slice(&tag);
            decoded.extend_from_slice(&[0; 4]);
            decoded.extend_from_slice(&(body.len() as u32).to_le_bytes());
            decoded.extend_from_slice(body);
        }
        compensate(&mut decoded);
        let mut payload = coded(&decoded[..20]);
        let rest = &decoded[20..];
        payload.extend_from_slice(&(0x8000 | rest.len() as u16).to_be_bytes());
        payload.extend_from_slice(rest);
        payload.extend_from_slice(&[0, 0]);
        let mut file = vec![0x47; HEADER];
        file.extend_from_slice(&payload);
        file.resize(file.len().next_multiple_of(4), 0);
        file.extend_from_slice(b"META");
        let footer = [
            0x024c_024c,
            MARKER,
            2,
            payload.len() as u32,
            decoded.len() as u32,
            0,
        ];
        for value in footer {
            file.extend_from_slice(&value.to_le_bytes());
        }
        compensate(&mut file);
        file
    }

    #[test]
    fn back_references_copy_earlier_bytes() {
        // "abcd", then distance 4 with count 1 (4 bytes), then distance 2
        // with count 7 + 1 (11 bytes), then the end.
        let body = [
            0x0e, 0x00, b'a', b'b', b'c', b'd', 0x01, 0x04, 0x07, 0x02, 0x01, 0x00, 0x00,
        ];
        let mut out = Vec::new();
        decode_frame(&body, false, &mut out).unwrap();
        assert_eq!(out, b"abcdabcdcdcdcdcdcdc");
        let mut out = Vec::new();
        assert!(decode_frame(&[0x80, 0x00, 0x01, 0x09], false, &mut out).is_err());
    }

    #[test]
    fn a_sample_decodes_and_encodes_unchanged() {
        let file = sample(&[7u8; 64]);
        let update = Update::read(&file).unwrap();
        let rtos = update.section("RTOS").unwrap();
        assert_eq!(&update.decoded[rtos], &[7u8; 64]);
        assert_eq!(update.encode(&update.decoded.clone()).unwrap(), file);
    }

    #[test]
    fn a_change_replaces_only_its_frames() {
        let file = sample(&[7u8; 64]);
        let update = Update::read(&file).unwrap();
        let mut target = update.decoded.clone();
        let rtos = update.section("RTOS").unwrap();
        target[rtos.start + 40] = 9;
        let new = update.encode(&target).unwrap();
        let again = Update::read(&new).unwrap();
        assert_eq!(again.decoded[rtos.start + 40], 9);
        assert_eq!(
            again.decoded[..rtos.start + 40],
            update.decoded[..rtos.start + 40]
        );
        // The coded first frame stays.
        assert_eq!(new[HEADER..HEADER + 4], file[HEADER..HEADER + 4]);
        assert_eq!(sum32(&new), 0);
        assert_eq!(sum32(&again.decoded), 0);
    }

    #[test]
    fn damaged_files_are_refused() {
        let file = sample(&[7u8; 64]);
        let mut sum = file.clone();
        sum[200] ^= 1;
        assert!(Update::read(&sum).is_err());
        assert!(Update::read(&file[..file.len() - 4]).is_err());
        assert!(Update::read(&[0; 256]).is_err());
    }
}
