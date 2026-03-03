use protocol::AttentionLevel;

pub fn needs_flash(level: AttentionLevel) -> bool {
    matches!(level, AttentionLevel::NeedsInput | AttentionLevel::Error)
}

pub fn detect_needs_input(output_bytes: &[u8]) -> bool {
    if output_bytes.is_empty() {
        return false;
    }
    let text = String::from_utf8_lossy(output_bytes);
    detect_needs_input_text(&text)
}

pub fn detect_needs_input_text(text: &str) -> bool {
    let text = normalize_for_match(text);
    let patterns = [
        "press enter",
        "press return",
        "enter to continue",
        "waiting for input",
        "waiting for your input",
        "requires your input",
        "choose an option",
        "select an option",
        "confirm",
        "[y/n]",
        "(y/n)",
        " y/n",
        "continue?",
        "this command requires approval",
        "requires approval",
        "do you want to proceed",
        "yes, and don't ask again",
        "yes, and dont ask again",
        "esc to cancel",
        "tab to amend",
    ];
    patterns.iter().any(|p| text.contains(p))
}

pub fn append_recent_output(recent: &mut String, output_bytes: &[u8]) {
    if output_bytes.is_empty() {
        return;
    }
    let chunk = String::from_utf8_lossy(output_bytes);
    let cleaned = normalize_for_match(&chunk);
    if cleaned.is_empty() {
        return;
    }
    if !recent.is_empty() {
        recent.push(' ');
    }
    recent.push_str(&cleaned);
    const MAX_RECENT: usize = 4096;
    if recent.len() > MAX_RECENT {
        trim_to_last_bytes_at_char_boundary(recent, MAX_RECENT);
    }
}

fn normalize_for_match(input: &str) -> String {
    let no_ansi = strip_ansi(input);
    no_ansi
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if let Some('[') = chars.peek().copied() {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            } else {
                let _ = chars.next();
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_to_last_bytes_at_char_boundary(s: &mut String, max_bytes: usize) {
    if s.len() <= max_bytes {
        return;
    }
    let mut start = s.len().saturating_sub(max_bytes);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    if start >= s.len() {
        s.clear();
        return;
    }
    s.drain(..start);
}
