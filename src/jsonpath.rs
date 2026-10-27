use crate::json::Json;
use crate::text::is_digits;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Seg {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path(pub Vec<Seg>);

pub fn parse(text: &str) -> Option<Path> {
    let bytes = text.as_bytes();
    if bytes.first().copied() != Some(b'$') {
        return None;
    }

    let mut position = 1;
    let mut segments = Vec::new();
    while position < bytes.len() {
        match bytes.get(position).copied()? {
            b'.' => {
                position += 1;
                let start = position;
                while matches!(
                    bytes.get(position).copied(),
                    Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
                ) {
                    position += 1;
                }
                if position == start {
                    return None;
                }
                let key = text.get(start..position)?;
                segments.push(Seg::Key(key.to_owned()));
            }
            b'[' => {
                position += 1;
                let start = position;
                while matches!(bytes.get(position).copied(), Some(b'0'..=b'9')) {
                    position += 1;
                }
                let digits = text.get(start..position)?;
                if !is_digits(digits, 6) || bytes.get(position).copied() != Some(b']') {
                    return None;
                }
                position += 1;
                segments.push(Seg::Index(digits.parse::<usize>().ok()?));
            }
            _ => return None,
        }
    }

    Some(Path(segments))
}

pub fn lookup<'a>(json: &'a Json, path: &Path) -> Option<&'a Json> {
    let mut current = json;
    for segment in &path.0 {
        current = match (current, segment) {
            (Json::Obj(pairs), Seg::Key(name)) => {
                let (_, value) = pairs.iter().find(|(key, _)| key == name)?;
                value
            }
            (Json::Arr(values), Seg::Index(index)) => values.get(*index)?,
            _ => return None,
        };
    }
    Some(current)
}
