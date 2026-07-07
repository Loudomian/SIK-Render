use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::str::FromStr;
use std::time::Duration;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoEncoder {
    Auto,
    Cpu,
    Nvenc,
    Qsv,
    Amf,
}

impl VideoEncoder {
    pub fn codec(self) -> &'static str {
        match self {
            Self::Auto | Self::Cpu => "libx264",
            Self::Nvenc => "h264_nvenc",
            Self::Qsv => "h264_qsv",
            Self::Amf => "h264_amf",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Cpu => "CPU (libx264)",
            Self::Nvenc => "NVIDIA NVENC",
            Self::Qsv => "Intel Quick Sync",
            Self::Amf => "AMD AMF",
        }
    }

    pub fn hardware_candidates() -> &'static [Self] {
        &[Self::Nvenc, Self::Qsv, Self::Amf]
    }

    pub fn is_hardware(self) -> bool {
        matches!(self, Self::Nvenc | Self::Qsv | Self::Amf)
    }
}

impl fmt::Display for VideoEncoder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Nvenc => "nvenc",
            Self::Qsv => "qsv",
            Self::Amf => "amf",
        })
    }
}

impl FromStr for VideoEncoder {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" | "libx264" => Ok(Self::Cpu),
            "nvenc" | "h264_nvenc" => Ok(Self::Nvenc),
            "qsv" | "h264_qsv" => Ok(Self::Qsv),
            "amf" | "h264_amf" => Ok(Self::Amf),
            _ => Err(()),
        }
    }
}

pub fn default_encoder() -> String {
    VideoEncoder::Auto.to_string()
}

pub fn normalize_encoder(value: &str) -> String {
    value
        .parse::<VideoEncoder>()
        .unwrap_or(VideoEncoder::Auto)
        .to_string()
}

pub fn normalize_actual_encoder(value: &str) -> String {
    let parts = value
        .split("->")
        .map(normalize_encoder)
        .filter(|part| part != "auto")
        .collect::<Vec<_>>();

    if parts.is_empty() {
        normalize_encoder(value)
    } else {
        parts.join("->")
    }
}

pub fn parse_encoder(value: &str) -> VideoEncoder {
    value.parse().unwrap_or(VideoEncoder::Auto)
}

pub fn resolve_auto_encoder(available: &[VideoEncoder]) -> VideoEncoder {
    VideoEncoder::hardware_candidates()
        .iter()
        .copied()
        .find(|candidate| available.contains(candidate))
        .unwrap_or(VideoEncoder::Cpu)
}

pub async fn probe_hardware_encoders(ffmpeg_executable: &Path) -> Vec<VideoEncoder> {
    let (nvenc, qsv, amf) = tokio::join!(
        probe_encoder(ffmpeg_executable, VideoEncoder::Nvenc),
        probe_encoder(ffmpeg_executable, VideoEncoder::Qsv),
        probe_encoder(ffmpeg_executable, VideoEncoder::Amf),
    );

    [
        (VideoEncoder::Nvenc, nvenc),
        (VideoEncoder::Qsv, qsv),
        (VideoEncoder::Amf, amf),
    ]
    .into_iter()
    .filter_map(|(encoder, available)| available.then_some(encoder))
    .collect()
}

async fn probe_encoder(ffmpeg_executable: &Path, encoder: VideoEncoder) -> bool {
    let mut command = TokioCommand::new(ffmpeg_executable);
    command
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("color=c=black:s=256x256:d=1")
        .arg("-frames:v")
        .arg("1")
        .arg("-c:v")
        .arg(encoder.codec())
        .arg("-f")
        .arg("null")
        .arg("-");
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command.kill_on_drop(true);
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    matches!(
        tokio::time::timeout(Duration::from_secs(5), command.status()).await,
        Ok(Ok(status)) if status.success()
    )
}

pub fn encoder_args(kind: VideoEncoder, crf: u32, preset: &str) -> Vec<(&'static str, String)> {
    let crf = crf.min(51).to_string();
    match kind {
        VideoEncoder::Auto | VideoEncoder::Cpu => vec![
            ("-c:v", "libx264".into()),
            ("-preset", preset.into()),
            ("-crf", crf),
        ],
        VideoEncoder::Nvenc => vec![
            ("-c:v", "h264_nvenc".into()),
            ("-rc", "vbr".into()),
            ("-cq", crf),
            ("-b:v", "0".into()),
            ("-preset", nvenc_preset(preset).into()),
        ],
        VideoEncoder::Qsv => vec![
            ("-c:v", "h264_qsv".into()),
            ("-preset", qsv_preset(preset).into()),
            ("-global_quality", crf),
        ],
        VideoEncoder::Amf => {
            let quality = amf_quality(preset).to_string();
            vec![
                ("-c:v", "h264_amf".into()),
                ("-rc", "cqp".into()),
                ("-qp_i", crf.clone()),
                ("-qp_p", crf.clone()),
                ("-qp_b", crf),
                ("-quality", quality),
            ]
        }
    }
}

fn nvenc_preset(preset: &str) -> &'static str {
    match preset {
        "ultrafast" | "superfast" | "veryfast" => "p1",
        "faster" | "fast" => "p3",
        "slow" => "p5",
        "slower" => "p6",
        "veryslow" => "p7",
        _ => "p4",
    }
}

fn qsv_preset(preset: &str) -> &'static str {
    match preset {
        "ultrafast" | "superfast" | "veryfast" => "veryfast",
        "faster" => "faster",
        "fast" => "fast",
        "slow" => "slow",
        "slower" => "slower",
        "veryslow" => "veryslow",
        _ => "medium",
    }
}

fn amf_quality(preset: &str) -> &'static str {
    match preset {
        "ultrafast" | "superfast" | "veryfast" | "faster" | "fast" => "speed",
        "slow" | "slower" | "veryslow" => "quality",
        _ => "balanced",
    }
}
