use regex::Regex;
use std::sync::OnceLock;

/// Progress parsed from a single line of Blender stdout.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameProgress {
    pub frame: u32,
    pub time_elapsed: f32,
    pub memory_mb: f32,
}

static FRAME_RE: OnceLock<Regex> = OnceLock::new();

/// Try to parse a Blender stdout line into frame progress.
/// Example line: `Fra:42 Mem:128.50M (Peak 200.00M) | Time:00:04.12 | ...`
pub fn parse_line(line: &str) -> Option<FrameProgress> {
    let re = FRAME_RE.get_or_init(|| {
        Regex::new(r"Fra:(\d+).*?Mem:([\d.]+)M.*?Time:([\d:]+\.[\d]+)").unwrap()
    });

    let caps = re.captures(line)?;
    let frame: u32 = caps[1].parse().ok()?;
    let memory_mb: f32 = caps[2].parse().ok()?;
    let time_str = &caps[3];

    let elapsed = parse_time(time_str);

    Some(FrameProgress {
        frame,
        time_elapsed: elapsed,
        memory_mb,
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
