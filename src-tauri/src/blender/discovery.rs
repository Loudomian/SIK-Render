use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Known Blender installation found on this system.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlenderInstall {
    pub version: String,
    pub executable: PathBuf,
}

/// Validate a user-supplied executable path and return a BlenderInstall.
pub fn blender_install_at(path: &std::path::Path) -> anyhow::Result<BlenderInstall> {
    let version = query_version(&path.to_path_buf())?;
    Ok(BlenderInstall {
        version,
        executable: path.to_path_buf(),
    })
}

/// Search common install paths for Blender executables.
pub fn discover() -> Vec<BlenderInstall> {
    let candidates = default_search_paths();
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();

    for path in candidates {
        if path.exists() && seen.insert(path.clone()) {
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
        discover_windows_paths()
    }
    #[cfg(target_os = "macos")]
    {
        let mut candidates = vec![PathBuf::from(
            "/Applications/Blender.app/Contents/MacOS/Blender",
        )];
        candidates.extend(command_search_paths("which", &["-a", "blender"]));
        candidates
    }
    #[cfg(target_os = "linux")]
    {
        let mut candidates = vec![
            PathBuf::from("/usr/bin/blender"),
            PathBuf::from("/usr/local/bin/blender"),
        ];
        candidates.extend(command_search_paths("which", &["-a", "blender"]));
        candidates
    }
}

#[cfg(target_os = "windows")]
fn discover_windows_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for env_key in ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"] {
        if let Ok(root) = std::env::var(env_key) {
            let root = PathBuf::from(root);
            candidates.extend(scan_for_blender(&root));
        }
    }

    candidates.extend(command_search_paths("where.exe", &["blender"]));
    candidates
}

#[cfg(target_os = "windows")]
fn scan_for_blender(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();

    for relative in [
        PathBuf::from("Blender Foundation"),
        PathBuf::from("Programs").join("Blender Foundation"),
    ] {
        let base = root.join(relative);
        if !base.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let blender_exe = entry.path().join("blender.exe");
                if blender_exe.exists() {
                    results.push(blender_exe);
                }
            }
        }
    }

    results
}

fn command_search_paths(program: &str, args: &[&str]) -> Vec<PathBuf> {
    let Ok(output) = std::process::Command::new(program).args(args).output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn query_version(executable: &PathBuf) -> Result<String> {
    let output = std::process::Command::new(executable)
        .arg("--version")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // First line: "Blender 4.2.1 (hash ...)" — take the second token only.
    let version = stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("unknown")
        .to_string();
    Ok(version)
}
