use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure(pub String);

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

pub fn shown(s: &str) -> String {
    s.chars()
        .map(|character| {
            if character < '\u{20}' || character == '\u{7f}' {
                '?'
            } else {
                character
            }
        })
        .collect()
}

pub fn name(s: &str) -> String {
    if s.is_empty() {
        "(empty)".to_owned()
    } else {
        shown(s)
    }
}

pub fn clip(s: &str) -> String {
    if points(s) <= 24 {
        s.to_owned()
    } else {
        let mut clipped: String = s.chars().take(21).collect();
        clipped.push_str("...");
        clipped
    }
}

pub fn disp(s: &str) -> String {
    shown(&clip(s))
}

pub fn points(s: &str) -> usize {
    s.chars().count()
}

pub fn trim_blanks(s: &str) -> &str {
    s.trim_matches(|character| character == ' ' || character == '\t')
}

pub fn trim_answer(s: &str) -> &str {
    s.trim_matches(|character| {
        character == ' ' || character == '\t' || character == '\r' || character == '\n'
    })
}

pub fn is_id(s: &str) -> bool {
    let mut characters = s.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if first == '_' || first == '-' {
        return false;
    }

    let count = 1 + characters.clone().count();
    count <= 32
        && (first.is_ascii_lowercase() || first.is_ascii_digit() || first == '_' || first == '-')
        && characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

pub fn is_digits(s: &str, max: usize) -> bool {
    let count = s.chars().count();
    count > 0 && count <= max && s.chars().all(|character| character.is_ascii_digit())
}

pub fn normalize(data: &[u8]) -> Option<String> {
    if data.contains(&0) {
        return None;
    }

    let text = std::str::from_utf8(data).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    Some(text.replace("\r\n", "\n"))
}

pub fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        result.pop();
    }
    result
}
