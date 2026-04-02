use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
}

/// Parse a keybinding string like "ctrl+shift+h" into (KeyCode, KeyModifiers).
/// Supported modifiers: ctrl, shift, alt. The final token is the key character.
pub fn parse_keybinding(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let lowered = s.trim().to_lowercase();
    let parts: Vec<&str> = lowered.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = KeyModifiers::empty();
    for &part in &parts[..parts.len() - 1] {
        match part {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            _ => return None,
        }
    }

    let key_str = parts.last()?;
    let code = if key_str.len() == 1 {
        KeyCode::Char(key_str.chars().next()?)
    } else {
        return None;
    };

    Some((code, modifiers))
}

/// Check whether a KeyEvent matches a keybinding string.
pub fn matches_keybinding(key: KeyEvent, binding: &str) -> bool {
    let Some((code, modifiers)) = parse_keybinding(binding) else {
        return false;
    };
    key.code == code && key.modifiers.contains(modifiers)
}
