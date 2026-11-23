//! Isolated Kitty PNG transfer assembler for Codex CLI 0.171.0.
//!
//! Codex sends a PNG in base64 as one or more Kitty APC frames. Recognizing
//! a frame is not sufficient: all chunks must be assembled before decoding.
//! This module is test-only until PTY interception and rendering are proven.

use std::collections::BTreeMap;

const MAX_ENCODED: usize = 12 * 1024 * 1024;
const MAX_PNG: usize = 8 * 1024 * 1024;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    Malformed,
    Unsupported,
    Oversized,
    MissingStart,
    InvalidPng,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PngTransfer {
    pub(super) id: u32,
    pub(super) columns: u16,
    pub(super) rows: u16,
    pub(super) data: Vec<u8>,
}

struct Pending {
    id: u32,
    columns: u16,
    rows: u16,
    encoded: Vec<u8>,
}

pub(super) struct Assembler {
    pending: Option<Pending>,
}

impl Assembler {
    pub(super) fn new() -> Self {
        Self { pending: None }
    }

    pub(super) fn accept(&mut self, frame: &[u8]) -> Result<Option<PngTransfer>, Error> {
        let result = self.accept_inner(frame);
        if result.is_err() {
            self.pending = None;
        }
        result
    }

    fn accept_inner(&mut self, frame: &[u8]) -> Result<Option<PngTransfer>, Error> {
        let semi = frame.iter().position(|byte| *byte == b';').ok_or(Error::Malformed)?;
        let params = std::str::from_utf8(&frame[..semi]).map_err(|_| Error::Malformed)?;
        let encoded = &frame[semi + 1..];
        let mut fields = BTreeMap::new();
        for pair in params.split(',') {
            let (name, value) = pair.split_once('=').ok_or(Error::Malformed)?;
            if value.is_empty() || fields.insert(name, value).is_some() {
                return Err(Error::Malformed);
            }
            if !matches!(name, "a" | "t" | "f" | "i" | "c" | "r" | "m" | "q" | "C") {
                return Err(Error::Unsupported);
            }
        }
        let more = match fields.get("m").copied().unwrap_or("0") {
            "0" => false,
            "1" => true,
            _ => return Err(Error::Malformed),
        };

        if let Some(action) = fields.get("a") {
            if *action != "T" || fields.get("t") != Some(&"d") || fields.get("f") != Some(&"100") {
                return Err(Error::Unsupported);
            }
            let id = parse_u32(fields.get("i"))?;
            let columns = parse_u16(fields.get("c"))?;
            let rows = parse_u16(fields.get("r"))?;
            if columns == 0 || rows == 0 {
                return Err(Error::Malformed);
            }
            self.pending = Some(Pending {
                id,
                columns,
                rows,
                encoded: Vec::new(),
            });
        } else if self.pending.is_none() || fields.keys().any(|key| *key != &"m" && *key != &"q") {
            return Err(Error::MissingStart);
        }

        let pending = self.pending.as_mut().ok_or(Error::MissingStart)?;
        if pending.encoded.len().saturating_add(encoded.len()) > MAX_ENCODED {
            return Err(Error::Oversized);
        }
        if !encoded.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')) {
            return Err(Error::Malformed);
        }
        pending.encoded.extend_from_slice(encoded);
        if more {
            return Ok(None);
        }
        let pending = self.pending.take().ok_or(Error::MissingStart)?;
        let encoded = pending.encoded;
        if encoded.is_empty() || encoded.len() % 4 != 0 {
            return Err(Error::Malformed);
        }
        let padding = encoded.iter().rev().take_while(|byte| **byte == b'=').count();
        if padding > 2 || encoded[..encoded.len() - padding].contains(&b'=') {
            return Err(Error::Malformed);
        }
        if encoded.len() / 4 * 3 > MAX_PNG + 2 {
            return Err(Error::Oversized);
        }
        let text = std::str::from_utf8(&encoded).map_err(|_| Error::Malformed)?;
        let png = gtk::glib::base64_decode(text);
        if png.len() > MAX_PNG {
            return Err(Error::Oversized);
        }
        if !is_bounded_png(&png) {
            return Err(Error::InvalidPng);
        }
        Ok(Some(PngTransfer {
            id: pending.id,
            columns: pending.columns,
            rows: pending.rows,
            data: png,
        }))
    }
}

fn parse_u32(value: Option<&&str>) -> Result<u32, Error> {
    value.ok_or(Error::Malformed)?.parse().map_err(|_| Error::Malformed)
}

fn parse_u16(value: Option<&&str>) -> Result<u16, Error> {
    value.ok_or(Error::Malformed)?.parse().map_err(|_| Error::Malformed)
}

/// Header-only bounding; GDK must validate the full PNG before rendering.
fn is_bounded_png(data: &[u8]) -> bool {
    if data.len() < 24
        || !data.starts_with(b"\x89PNG\r\n\x1a\n")
        || data.get(8..12) != Some(&[0, 0, 0, 13][..])
        || data.get(12..16) != Some(&b"IHDR"[..])
    {
        return false;
    }
    let width = u32::from_be_bytes(data[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(data[20..24].try_into().unwrap());
    width > 0 && height > 0
        && width <= 16_384 && height <= 16_384
        && u64::from(width) * u64::from(height) <= MAX_PIXELS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_header() -> Vec<u8> {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13_u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&2_u32.to_be_bytes());
        png.extend_from_slice(&3_u32.to_be_bytes());
        png
    }

    #[test]
    fn codex_two_chunk_stream_round_trips() {
        let png = png_header();
        let encoded = gtk::glib::base64_encode(&png).to_string();
        let first = format!("a=T,t=d,f=100,c=4,r=3,q=2,i=7,m=1,C=1;{}", &encoded[..16]);
        let second = format!("m=0;{}", &encoded[16..]);
        let mut decoder = Assembler::new();
        assert_eq!(decoder.accept(first.as_bytes()), Ok(None));
        assert_eq!(decoder.accept(second.as_bytes()), Ok(Some(PngTransfer {
            id: 7, columns: 4, rows: 3, data: png
        })));
    }

    #[test]
    fn one_chunk_png_round_trips() {
        let png = png_header();
        let encoded = gtk::glib::base64_encode(&png);
        let command = format!("a=T,t=d,f=100,c=2,r=2,i=3,m=0;{encoded}");
        assert_eq!(Assembler::new().accept(command.as_bytes()).unwrap().unwrap().data, png);
    }

    #[test]
    fn reject_bad_chunk_and_clear_pending() {
        let mut decoder = Assembler::new();
        assert_eq!(decoder.accept(b"m=0;AAAA"), Err(Error::MissingStart));
        assert_eq!(decoder.accept(b"a=T,t=d,f=100,c=2,r=2,i=3,m=1;AAAA"), Ok(None));
        assert_eq!(decoder.accept(b"m=0;not_base64!"), Err(Error::Malformed));
        assert_eq!(decoder.accept(b"m=0;AAAA"), Err(Error::MissingStart));
    }

    #[test]
    fn reject_unsupported_transfer_and_malformed_params() {
        let mut decoder = Assembler::new();
        assert_eq!(decoder.accept(b"a=T,t=f,f=100,c=2,r=2,i=3;AAAA"), Err(Error::Unsupported));
        assert_eq!(decoder.accept(b"a=T,a=T,t=d,f=100,c=2,r=2,i=3;AAAA"), Err(Error::Malformed));
    }

    #[test]
    fn reject_oversized_payload() {
        let mut decoder = Assembler::new();
        assert_eq!(decoder.accept(b"a=T,t=d,f=100,c=2,r=2,i=3,m=1;AAAA"), Ok(None));
        let huge = format!("m=0;{}", "A".repeat(MAX_ENCODED + 1));
        assert_eq!(decoder.accept(huge.as_bytes()), Err(Error::Oversized));
    }

    #[test]
    fn reject_png_dimension_bomb() {
        let mut png = png_header();
        png[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
        assert!(!is_bounded_png(&png));
        png[16..20].copy_from_slice(&4096_u32.to_be_bytes());
        png[20..24].copy_from_slice(&4097_u32.to_be_bytes());
        assert!(!is_bounded_png(&png));
    }
}
