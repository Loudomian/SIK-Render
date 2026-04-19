use anyhow::Result;
use std::path::PathBuf;

/// Known Blender installation found on this system.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BlenderInstall {
    pub version: String,
    pub executable: PathBuf,
}

/// Search common install paths for Blender executables.
pub fn discover() -> Vec<BlenderInstall> {
    let candidates = default_search_paths();
    let mut found = Vec::new();

    for path in candidates {
        if path.exists() {
            if let Ok(version) = query_version(&path) {
                found.push(BlenderInstall {
                    version,
                    executable: path,
                });
            }
        }
    }

    found
}

fn default_search_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            PathBuf::from(r"C:\Program Files\Blender Foundation\Blender 4.2\blender.exe"),
            PathBuf::from(r"C:\Program Files\Blender Foundation\Blender 4.1\blender.exe"),
            PathBuf::from(r"C:\Program Files\Blender Foundation\Blender 4.0\blender.exe"),
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin/blender"),
            PathBuf::from("/usr/local/bin/blender"),
        ]
    }
}

fn query_version(executable: &PathBuf) -> Result<String> {
    let output = std::process::Command::new(executable)
        .arg("--version")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .lines()
        .next()
        .unwrap_or("unknown")
        .replace("Blender ", "")
        .trim()
        .to_string();
    Ok(version)
}
