use std::path::Path;

pub fn trailing_frame_number(path: &Path) -> Option<i32> {
    let stem = path.file_stem()?.to_str()?;
    let digits_rev: String = stem
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits_rev.is_empty() {
        return None;
    }
    digits_rev.chars().rev().collect::<String>().parse().ok()
}

pub fn strip_frame_placeholders(path: &str) -> String {
    let split_index = path
        .char_indices()
        .filter(|(_, ch)| *ch == '/' || *ch == '\\')
        .map(|(idx, _)| idx + 1)
        .last()
        .unwrap_or(0);
    let (prefix, file_name) = path.split_at(split_index);

    let mut cleaned = String::with_capacity(file_name.len());
    let mut chars = file_name.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '#' {
            while matches!(chars.peek(), Some('#')) {
                chars.next();
            }
            continue;
        }
        cleaned.push(ch);
    }

    format!("{prefix}{cleaned}")
}
