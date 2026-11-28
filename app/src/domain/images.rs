//! Images for image views: a module's pixel icon as SVG, and `data:` URIs.

/// A module's pixel icon (a PNG) as SVG markup of its pixels, one
/// rectangle per run of a color in a row, so that it stays sharp at any
/// size: image views smooth a scaled bitmap. Transparent pixels are left
/// out. `None` for bytes that are not a PNG.
pub fn pixel_icon_svg(bytes: &[u8]) -> Option<String> {
    use std::fmt::Write as _;
    let (width, height, rgba) = grkit_core::png::decode_rgba8(bytes).ok()?;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" shape-rendering="crispEdges">"#
    );
    let pixel = |x: usize, y: usize| {
        let at = (y * width + x) * 4;
        rgba.get(at..at + 4)
            .filter(|p| p[3] != 0)
            .map(|p| [p[0], p[1], p[2], p[3]])
    };
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let Some(color) = pixel(x, y) else {
                x += 1;
                continue;
            };
            let start = x;
            while x < width && pixel(x, y) == Some(color) {
                x += 1;
            }
            let [r, g, b, a] = color;
            let _ = write!(
                svg,
                r##"<rect x="{start}" y="{y}" width="{}" height="1" fill="#{r:02x}{g:02x}{b:02x}""##,
                x - start
            );
            if a != 255 {
                let _ = write!(svg, r#" fill-opacity="{:.3}""#, f32::from(a) / 255.0);
            }
            svg.push_str("/>");
        }
    }
    svg.push_str("</svg>");
    Some(svg)
}

/// A JPEG as a `data:` URI for an image view. `None` for other bytes.
pub fn jpeg_data_uri(bytes: &[u8]) -> Option<String> {
    bytes
        .starts_with(&[0xFF, 0xD8])
        .then(|| data_uri("image/jpeg", bytes))
}

/// `bytes` of the media type `mime` as a `data:` URI for an image view.
pub fn data_uri(mime: &str, bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut uri = String::with_capacity(mime.len() + 13 + bytes.len().div_ceil(3) * 4);
    uri.push_str("data:");
    uri.push_str(mime);
    uri.push_str(";base64,");
    for chunk in bytes.chunks(3) {
        let at = |i: usize| chunk.get(i).copied().unwrap_or(0) as u32;
        let word = at(0) << 16 | at(1) << 8 | at(2);
        for index in 0..4 {
            if index <= chunk.len() {
                uri.push(ALPHABET[(word >> (18 - index * 6) & 63) as usize] as char);
            } else {
                uri.push('=');
            }
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_icon_becomes_one_rectangle_per_run_of_its_pixels() {
        let png = include_bytes!("../../../modules/grain/tiles/0-module.png");
        let (width, height, rgba) = grkit_core::png::decode_rgba8(png).unwrap();
        let svg = pixel_icon_svg(png).unwrap();
        assert!(svg.contains(&format!(r#"viewBox="0 0 {width} {height}""#)));
        assert!(svg.contains(r#"shape-rendering="crispEdges""#));
        let drawn: usize = svg
            .split(r#" width=""#)
            .skip(2)
            .map(|rest| rest[..rest.find('"').unwrap()].parse::<usize>().unwrap())
            .sum();
        let opaque = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] != 0).count();
        assert_eq!(drawn, opaque);
        assert_eq!(pixel_icon_svg(b"not a png"), None);
    }
}
