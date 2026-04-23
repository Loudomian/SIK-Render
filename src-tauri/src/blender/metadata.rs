use std::path::Path;

const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const READ_LIMIT: usize = 8192;

/// Read the `RenderTime` field from a PNG file's tEXt chunks.
/// Returns the time in seconds, or None if unavailable or unsupported format.
pub fn read_png_render_time(path: &Path) -> Option<f32> {
    let data = read_head(path)?;
    parse_png_render_time(&data)
}

fn read_head(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; READ_LIMIT];
    let n = f.read(&mut buf).ok()?;
    buf.truncate(n);
    Some(buf)
}

fn parse_png_render_time(data: &[u8]) -> Option<f32> {
    if data.len() < 8 || &data[..8] != PNG_SIG {
        return None;
    }
    let mut pos = 8usize;
    while pos + 12 <= data.len() {
        let length = u32::from_be_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let data_end = pos + 8 + length;
        if data_end + 4 > data.len() {
            break; // truncated read — stop gracefully
        }
        if chunk_type == b"tEXt" {
            let chunk_data = &data[pos + 8..data_end];
            if let Some(nul) = chunk_data.iter().position(|&b| b == 0) {
                if &chunk_data[..nul] == b"RenderTime" {
                    if let Ok(s) = std::str::from_utf8(&chunk_data[nul + 1..]) {
                        return parse_time(s.trim());
                    }
                }
            }
        }
        pos = data_end + 4; // advance past CRC
    }
    None
}

fn parse_time(s: &str) -> Option<f32> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [mm, ss] => {
            let m: f32 = mm.parse().ok()?;
            let s: f32 = ss.parse().ok()?;
            Some(m * 60.0 + s)
        }
        [hh, mm, ss] => {
            let h: f32 = hh.parse().ok()?;
            let m: f32 = mm.parse().ok()?;
            let s: f32 = ss.parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_png_render_time, parse_time};

    #[test]
    fn parses_mm_ss() {
        assert!((parse_time("01:23.45").unwrap() - 83.45).abs() < 0.01);
    }

    #[test]
    fn parses_hh_mm_ss() {
        assert!((parse_time("01:02:03.00").unwrap() - 3723.0).abs() < 0.01);
    }

    #[test]
    fn rejects_non_png() {
        assert!(parse_png_render_time(b"not a png").is_none());
    }

    #[test]
    fn finds_render_time_in_text_chunk() {
        let mut data = Vec::new();
        data.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        // minimal fake tEXt chunk: length=19, type=tEXt, "RenderTime\000:09.23", CRC=0
        let payload = b"RenderTime\x0009:23.00";
        let length = (payload.len() as u32).to_be_bytes();
        data.extend_from_slice(&length);
        data.extend_from_slice(b"tEXt");
        data.extend_from_slice(payload);
        data.extend_from_slice(&[0u8; 4]); // fake CRC
        let t = parse_png_render_time(&data).unwrap();
        assert!((t - 563.0).abs() < 0.1, "got {t}");
    }
}
