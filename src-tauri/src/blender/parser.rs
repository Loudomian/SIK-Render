use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimeProgress {
    pub time_elapsed: f32,
    pub remaining_secs: Option<f32>,
}

static FRAME_RE: OnceLock<Regex> = OnceLock::new();
static RENDERING_FRAME_RE: OnceLock<Regex> = OnceLock::new();
static TIME_RE: OnceLock<Regex> = OnceLock::new();

pub fn parse_frame(line: &str) -> Option<u32> {
    let re = FRAME_RE.get_or_init(|| Regex::new(r"Fra:\s*(\d+)").unwrap());
    if let Some(caps) = re.captures(line) {
        return caps[1].parse().ok();
    }

    let re = RENDERING_FRAME_RE.get_or_init(|| Regex::new(r"Rendering frame\s+(\d+)").unwrap());
    let caps = re.captures(line)?;
    caps[1].parse().ok()
}

pub fn parse_time_progress(line: &str) -> Option<TimeProgress> {
    let re = TIME_RE.get_or_init(|| {
        Regex::new(r"Time:\s*([\d:]+\.\d+)(?:.*?Remaining:\s*([\d:]+\.\d+))?").unwrap()
    });

    let caps = re.captures(line)?;
    let elapsed = parse_time(&caps[1]);
    let remaining_secs = caps.get(2).map(|m| parse_time(m.as_str()));

    Some(TimeProgress {
        time_elapsed: elapsed,
        remaining_secs,
    })
}

fn parse_time(s: &str) -> f32 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [mm, ss] => {
            let m: f32 = mm.parse().unwrap_or(0.0);
            let s: f32 = ss.parse().unwrap_or(0.0);
            m * 60.0 + s
        }
        [hh, mm, ss] => {
            let h: f32 = hh.parse().unwrap_or(0.0);
            let m: f32 = mm.parse().unwrap_or(0.0);
            let s: f32 = ss.parse().unwrap_or(0.0);
            h * 3600.0 + m * 60.0 + s
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_frame, parse_time_progress};

    #[test]
    fn parses_frame_from_fra_line() {
        assert_eq!(parse_frame("Fra:42 Mem:123.00M"), Some(42));
    }

    #[test]
    fn parses_frame_from_rendering_frame_line() {
        assert_eq!(parse_frame("Rendering frame 100, view layer"), Some(100));
    }

    #[test]
    fn ignores_lines_without_frame_number() {
        assert_eq!(parse_frame("no frame here"), None);
    }

    #[test]
    fn parses_elapsed_and_remaining_time() {
        let progress =
            parse_time_progress("Time: 00:01.50 | Remaining: 00:03.00").expect("time progress");
        assert!((progress.time_elapsed - 1.5).abs() < f32::EPSILON);
        assert_eq!(progress.remaining_secs, Some(3.0));
    }
}
