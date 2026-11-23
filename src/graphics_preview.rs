//! Opt-in GTK4 rendering proof of concept.
//!
//! This renders a *user-selected local PNG* inside a Veyl terminal pane as
//! an overlay. It does NOT intercept Codex output, follow the terminal cursor,
//! or advertise Kitty graphics support. The feature is off by default.

use gtk::prelude::*;
use std::path::Path;

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;

fn checked_png(path: &Path) -> Result<gtk::gdk::Texture, String> {
    let metadata = std::fs::metadata(path).map_err(|err| err.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("not a regular file, or PNG exceeds the 8 MiB test limit".into());
    }
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|err| err.to_string())?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("PNG exceeds the 8 MiB test limit".into());
    }
    let (width, height) =
        png_dimensions(&bytes).ok_or("not a bounded PNG with a valid IHDR")?;
    let data = gtk::glib::Bytes::from(&bytes);
    let texture = gtk::gdk::Texture::from_bytes(&data)
        .map_err(|err| format!("PNG could not be decoded: {err}"))?;
    if texture.width() != width as i32 || texture.height() != height as i32 {
        return Err("PNG dimensions changed during decode".into());
    }
    Ok(texture)
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24
        || !data.starts_with(b"\x89PNG\r\n\x1a\n")
        || data.get(8..12)? != [0, 0, 0, 13]
        || data.get(12..16)? != b"IHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(data.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(data.get(20..24)?.try_into().ok()?);
    if width == 0 || height == 0
        || width > 16_384 || height > 16_384
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return None;
    }
    Some((width, height))
}

/// Show a non-interactive preview in the active GTK terminal surface.
/// Each pane owns its own overlay; it is destroyed with its pane.
pub(crate) fn attach(overlay: &gtk::Overlay) {
    let Some(file) = std::env::var_os("VEYL_GRAPHICS_PREVIEW_FILE") else {
        return;
    };
    let texture = match checked_png(Path::new(&file)) {
        Ok(texture) => texture,
        Err(error) => {
            eprintln!("Veyl graphics prototype: {error}");
            return;
        }
    };

    let picture = gtk::Picture::new();
    picture.set_paintable(Some(&texture));
    picture.set_content_fit(gtk::ContentFit::Contain);
    picture.set_size_request(400, 240);
    picture.set_halign(gtk::Align::Start);
    picture.set_valign(gtk::Align::Start);
    picture.set_margin_start(12);
    picture.set_margin_top(12);
    picture.set_can_target(false);
    overlay.add_overlay(&picture);
}

#[cfg(test)]
mod tests {
    use super::png_dimensions;

    #[test]
    fn png_guard_accepts_bounded_dimensions_only() {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&128_u32.to_be_bytes());
        bytes.extend_from_slice(&64_u32.to_be_bytes());
        assert_eq!(png_dimensions(&bytes), Some((128, 64)));
        bytes[16..20].copy_from_slice(&100_000_u32.to_be_bytes());
        assert_eq!(png_dimensions(&bytes), None);
        bytes[16..20].copy_from_slice(&4096_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&4097_u32.to_be_bytes());
        assert_eq!(png_dimensions(&bytes), None);
    }
}
