use crate::check::{self, Check};
use crate::text::{Failure, is_id, name, trim_blanks};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case {
    pub id: String,
    pub line: usize,
    pub prompt: String,
    pub checks: Vec<Check>,
}

pub fn parse(lines: &[String], file: &str) -> Result<Vec<Case>, Failure> {
    let mut cases = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    let mut current: Option<Case> = None;
    let mut has_prompt = false;

    for (index, raw_line) in lines.iter().enumerate() {
        let line_number = index + 1;
        let line = trim_blanks(raw_line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = keyword_rest(line, "case") {
            if let Some(previous) = current.take() {
                validate_complete(&previous, has_prompt, file)?;
                cases.push(previous);
            }
            has_prompt = false;

            let id = trim_blanks(rest);
            if !is_id(id) {
                return Err(line_failure(
                    file,
                    line_number,
                    format!("case {} is not valid", name(id)),
                ));
            }
            if ids.iter().any(|seen| seen == id) {
                return Err(line_failure(
                    file,
                    line_number,
                    format!("case {} given twice", name(id)),
                ));
            }
            ids.push(id.to_owned());
            current = Some(Case {
                id: id.to_owned(),
                line: line_number,
                prompt: String::new(),
                checks: Vec::new(),
            });
            continue;
        }

        if let Some(prompt_text) = line.strip_prefix("prompt:") {
            if current.is_none() {
                return Err(line_failure(
                    file,
                    line_number,
                    "prompt outside a case".to_owned(),
                ));
            }
            if has_prompt {
                return Err(line_failure(
                    file,
                    line_number,
                    "prompt given twice".to_owned(),
                ));
            }
            let prompt = trim_blanks(prompt_text);
            if prompt.is_empty() {
                return Err(line_failure(
                    file,
                    line_number,
                    "prompt is empty".to_owned(),
                ));
            }
            if let Some(active) = current.as_mut() {
                active.prompt = prompt.to_owned();
            }
            has_prompt = true;
            continue;
        }

        if let Some(rest) = keyword_rest(line, "expect") {
            let Some((kind, value)) = rest.split_once(':') else {
                return Err(line_failure(
                    file,
                    line_number,
                    "expect needs KIND: VALUE".to_owned(),
                ));
            };
            let kind = trim_blanks(kind);
            let value = trim_blanks(value);
            if current.is_none() {
                return Err(line_failure(
                    file,
                    line_number,
                    "expect outside a case".to_owned(),
                ));
            }
            let parsed = check::parse(kind, value)
                .map_err(|message| line_failure(file, line_number, message))?;
            if let Some(active) = current.as_mut() {
                active.checks.push(parsed);
            }
            continue;
        }

        return Err(line_failure(
            file,
            line_number,
            "line is not case, prompt or expect".to_owned(),
        ));
    }

    if let Some(last) = current {
        validate_complete(&last, has_prompt, file)?;
        cases.push(last);
    }

    if cases.is_empty() {
        return Err(Failure(format!("{}: no cases found", name(file))));
    }

    Ok(cases)
}

fn keyword_rest<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if line == keyword {
        return Some("");
    }
    let rest = line.strip_prefix(keyword)?;
    if rest.starts_with(' ') || rest.starts_with('\t') {
        Some(rest)
    } else {
        None
    }
}

fn validate_complete(case: &Case, has_prompt: bool, file: &str) -> Result<(), Failure> {
    let missing = if !has_prompt {
        Some(format!("case {} has no prompt", name(&case.id)))
    } else if case.checks.is_empty() {
        Some(format!("case {} has no expect", name(&case.id)))
    } else {
        None
    };
    if let Some(message) = missing {
        return Err(line_failure(file, case.line, message));
    }
    Ok(())
}

fn line_failure(file: &str, line: usize, message: String) -> Failure {
    Failure(format!("{}:{}: {}", name(file), line, message))
}
