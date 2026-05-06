use std::path::{Path, PathBuf};

const MAX_FILE_STEM_LEN: usize = 160;
const TRUNCATED_HEX_LEN: usize = MAX_FILE_STEM_LEN - 2 - 1 - 16;
const MAX_HOSTNAME_LEN: usize = 48;
const MAX_IP_LEN: usize = 48;
const NODE_ID_PREFIX_LEN: usize = 4;

pub fn node_file_path_for_info(
    dir: &Path,
    hostname: &str,
    ip_address: &str,
    node_id: &str,
    extension: &str,
) -> PathBuf {
    dir.join(format!(
        "{}.{}",
        readable_node_file_stem(hostname, ip_address, node_id),
        extension
    ))
}

pub fn node_file_path(dir: &Path, node_id: &str, extension: &str) -> PathBuf {
    node_file_path_for_info(dir, "unknown", "unknown", node_id, extension)
}

pub fn node_file_candidates_for_info(
    dir: &Path,
    hostname: &str,
    ip_address: &str,
    node_id: &str,
    extension: &str,
) -> Vec<PathBuf> {
    let mut candidates = vec![node_file_path_for_info(
        dir, hostname, ip_address, node_id, extension,
    )];
    candidates.extend(node_file_candidates(dir, node_id, extension));
    dedupe_paths(candidates)
}

pub fn node_file_candidates(dir: &Path, node_id: &str, extension: &str) -> Vec<PathBuf> {
    dedupe_paths(vec![
        node_file_path(dir, node_id, extension),
        dir.join(format!(
            "{}.{}",
            encoded_node_id_file_stem(node_id),
            extension
        )),
        dir.join(format!(
            "{}.{}",
            legacy_node_id_file_stem(node_id),
            extension
        )),
    ])
}

fn readable_node_file_stem(hostname: &str, ip_address: &str, node_id: &str) -> String {
    format!(
        "node.{}.{}.{}",
        sanitize_file_part(hostname, MAX_HOSTNAME_LEN),
        sanitize_file_part(ip_address, MAX_IP_LEN),
        node_id_file_prefix(node_id),
    )
}

fn node_id_file_prefix(node_id: &str) -> String {
    let prefix = sanitize_file_part(node_id, NODE_ID_PREFIX_LEN);
    if prefix.len() == NODE_ID_PREFIX_LEN {
        prefix
    } else {
        "node".to_string()
    }
}

fn sanitize_file_part(value: &str, max_len: usize) -> String {
    let mut output = String::with_capacity(value.len().min(max_len));
    let mut previous_was_separator = false;

    for ch in value.chars() {
        let safe = if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            ch
        } else {
            '-'
        };

        if safe == '-' && previous_was_separator {
            continue;
        }

        output.push(safe);
        previous_was_separator = safe == '-';
        if output.len() >= max_len {
            break;
        }
    }

    let trimmed = output.trim_matches(|ch| ch == '-' || ch == '_' || ch == '.');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.contains(&path) {
            deduped.push(path);
        }
    }
    deduped
}

fn encoded_node_id_file_stem(node_id: &str) -> String {
    let encoded = hex_encode(node_id.as_bytes());
    if encoded.len() <= MAX_FILE_STEM_LEN - 2 {
        return format!("n_{encoded}");
    }

    let prefix = &encoded[..TRUNCATED_HEX_LEN];
    format!("n_{prefix}_{:016x}", fnv1a64(node_id.as_bytes()))
}

fn legacy_node_id_file_stem(node_id: &str) -> String {
    node_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
