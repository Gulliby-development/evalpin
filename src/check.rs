use crate::json::{self, Json};
use crate::jsonpath::{self, Path};
use crate::text::{disp, is_digits, name, points, trim_answer, trim_blanks};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Check {
    Equals(String),
    Contains(String),
    NotContains(String),
    OneOf(Vec<String>),
    MaxChars(usize),
    MinChars(usize),
    Json {
        path_text: String,
        path: Path,
        expected: Json,
        expected_text: String,
    },
}

pub fn parse(kind: &str, value: &str) -> Result<Check, String> {
    let check = match kind {
        "equals" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            Check::Equals(value.to_owned())
        }
        "contains" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            Check::Contains(value.to_owned())
        }
        "not-contains" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            Check::NotContains(value.to_owned())
        }
        "one-of" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            let choices: Vec<String> = value
                .split('|')
                .map(trim_blanks)
                .map(str::to_owned)
                .collect();
            if choices.iter().any(String::is_empty) {
                return Err("expect one-of has an empty choice".to_owned());
            }
            Check::OneOf(choices)
        }
        "max-chars" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            let Some(limit) = parse_count(value) else {
                return Err(number_error(kind, value));
            };
            Check::MaxChars(limit)
        }
        "min-chars" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            let Some(limit) = parse_count(value) else {
                return Err(number_error(kind, value));
            };
            Check::MinChars(limit)
        }
        "json" => {
            if value.is_empty() {
                return Err(needs_value(kind));
            }
            parse_json_check(value)?
        }
        _ => return Err(format!("unknown expect kind {}", name(kind))),
    };
    Ok(check)
}

fn needs_value(kind: &str) -> String {
    format!("expect {} needs a value", name(kind))
}

fn number_error(kind: &str, value: &str) -> String {
    format!(
        "expect {} needs a whole number of 0 or more, got {}",
        name(kind),
        name(value)
    )
}

fn parse_count(value: &str) -> Option<usize> {
    if !is_digits(value, 6) {
        return None;
    }
    value.parse::<usize>().ok()
}

fn parse_json_check(value: &str) -> Result<Check, String> {
    let Some((path_text, expected_text)) = value.split_once('=') else {
        return Err("expect json needs PATH = VALUE".to_owned());
    };
    let path_text = trim_blanks(path_text);
    let expected_text = trim_blanks(expected_text);
    let Some(path) = jsonpath::parse(path_text) else {
        return Err(format!("json path {} is not valid", name(path_text)));
    };
    let expected = json::parse(expected_text)
        .filter(is_scalar)
        .ok_or_else(|| format!("json value {} is not a JSON scalar", name(expected_text)))?;
    Ok(Check::Json {
        path_text: path_text.to_owned(),
        path,
        expected,
        expected_text: expected_text.to_owned(),
    })
}

fn is_scalar(value: &Json) -> bool {
    matches!(
        value,
        Json::Null | Json::Bool(_) | Json::Num(_) | Json::Str(_)
    )
}

impl Check {
    pub fn eval(&self, reply: &str) -> Option<String> {
        let answer = trim_answer(reply);
        match self {
            Check::Equals(expected) => {
                if answer == expected {
                    None
                } else {
                    Some(format!(
                        "reply \"{}\" is not \"{}\"",
                        disp(answer),
                        disp(expected)
                    ))
                }
            }
            Check::Contains(needle) => {
                if answer.contains(needle) {
                    None
                } else {
                    Some(format!("reply does not contain \"{}\"", disp(needle)))
                }
            }
            Check::NotContains(needle) => {
                if answer.contains(needle) {
                    Some(format!("reply contains \"{}\"", disp(needle)))
                } else {
                    None
                }
            }
            Check::OneOf(choices) => {
                if choices.iter().any(|choice| answer == choice) {
                    None
                } else {
                    Some(format!("reply \"{}\" is none of the choices", disp(answer)))
                }
            }
            Check::MaxChars(limit) => {
                let count = points(answer);
                if count <= *limit {
                    None
                } else {
                    Some(format!("reply has {count} chars, more than {limit}"))
                }
            }
            Check::MinChars(limit) => {
                let count = points(answer);
                if count >= *limit {
                    None
                } else {
                    Some(format!("reply has {count} chars, fewer than {limit}"))
                }
            }
            Check::Json {
                path_text,
                path,
                expected,
                expected_text,
            } => eval_json(answer, path_text, path, expected, expected_text),
        }
    }
}

fn eval_json(
    answer: &str,
    path_text: &str,
    path: &Path,
    expected: &Json,
    expected_text: &str,
) -> Option<String> {
    let json_text = unwrap_fence(answer);
    let Some(parsed) = json::parse(&json_text) else {
        return Some("reply is not JSON".to_owned());
    };
    let Some(found) = jsonpath::lookup(&parsed, path) else {
        return Some(format!("{} is missing", name(path_text)));
    };
    if scalar_equal(found, expected) {
        return None;
    }
    Some(format!(
        "{} is {}, expected {}",
        name(path_text),
        display_json(found),
        disp(expected_text)
    ))
}

fn unwrap_fence(answer: &str) -> String {
    let lines = crate::text::lines(answer);
    if lines.len() < 2 {
        return answer.to_owned();
    }
    let Some(first) = lines.first() else {
        return answer.to_owned();
    };
    let Some(last) = lines.last() else {
        return answer.to_owned();
    };
    if !first.starts_with("```") || *last != "```" {
        return answer.to_owned();
    }
    lines
        .iter()
        .skip(1)
        .take(lines.len() - 2)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

fn scalar_equal(found: &Json, expected: &Json) -> bool {
    match (found, expected) {
        (Json::Null, Json::Null) => true,
        (Json::Bool(found), Json::Bool(expected)) => found == expected,
        (Json::Num(found), Json::Num(expected)) => found == expected,
        (Json::Str(found), Json::Str(expected)) => found == expected,
        _ => false,
    }
}

fn display_json(value: &Json) -> String {
    match value {
        Json::Null => "null".to_owned(),
        Json::Bool(true) => "true".to_owned(),
        Json::Bool(false) => "false".to_owned(),
        Json::Num(number) => disp(number),
        Json::Str(text) => format!("\"{}\"", disp(text)),
        Json::Arr(_) => "an array".to_owned(),
        Json::Obj(_) => "an object".to_owned(),
    }
}
