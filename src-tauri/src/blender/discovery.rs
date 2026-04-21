use anyhow::Result;
use std::collections::BTreeSet;
use std::path::PathBuf;

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
        candidates.extend(find_in_path(&["Blender", "blender"]));
        candidates
    }
    #[cfg(target_os = "linux")]
    {
        let mut candidates = vec![
            PathBuf::from("/usr/bin/blender"),
            PathBuf::from("/usr/local/bin/blender"),
        ];
        candidates.extend(find_in_path(&["blender"]));
        candidates
    }
}

#[cfg(target_os = "windows")]
fn discover_windows_paths() -> Vec<PathBuf> {
    find_via_file_association().into_iter().collect()
}

#[cfg(target_os = "windows")]
fn find_via_file_association() -> Option<PathBuf> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::UI::Shell::{AssocQueryStringW, ASSOCF_NONE, ASSOCSTR_EXECUTABLE};

    let ext: Vec<u16> = ".blend\0".encode_utf16().collect();
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    let mut len = 0u32;

    unsafe {
        let _ = AssocQueryStringW(
            ASSOCF_NONE,
            ASSOCSTR_EXECUTABLE,
            PCWSTR(ext.as_ptr()),
            PCWSTR(verb.as_ptr()),
            PWSTR::null(),
            &mut len,
        );
    }

    if len == 0 {
        return None;
    }

    let mut buffer = vec![0u16; len as usize];
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF_NONE,
            ASSOCSTR_EXECUTABLE,
            PCWSTR(ext.as_ptr()),
            PCWSTR(verb.as_ptr()),
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
    };

    if result.is_err() || len == 0 {
        return None;
    }

    let raw = String::from_utf16_lossy(&buffer[..len.saturating_sub(1) as usize]);
    let path = normalize_windows_blender_path(PathBuf::from(raw.trim_end_matches('\0')));
    path.exists().then_some(path)
}

#[cfg(target_os = "windows")]
fn normalize_windows_blender_path(path: PathBuf) -> PathBuf {
    let is_launcher = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("blender-launcher.exe"))
        .unwrap_or(false);

    if !is_launcher {
        return path;
    }

    let blender = path.with_file_name("blender.exe");
    if blender.exists() {
        blender
    } else {
        path
    }
}

#[cfg(not(target_os = "windows"))]
fn find_in_path(names: &[&str]) -> Vec<PathBuf> {
    let Ok(path_var) = std::env::var("PATH") else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for dir in std::env::split_paths(&path_var) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.exists() {
                results.push(candidate);
            }
        }
    }
    results
}

fn query_version(executable: &PathBuf) -> Result<String> {
    let mut cmd = std::process::Command::new(executable);
    cmd.arg("--version");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = cmd.output()?;
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
