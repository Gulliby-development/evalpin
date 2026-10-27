use std::collections::BTreeMap;

use crate::text::{Failure, is_id, name, trim_blanks};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Entry {
    Text(String),
    Fail(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Replies(pub BTreeMap<String, Vec<Entry>>);

struct OpenReply {
    id: String,
    text: String,
    has_text_line: bool,
}

fn close_reply(open: &mut Option<OpenReply>, entries: &mut BTreeMap<String, Vec<Entry>>) {
    if let Some(reply) = open.take() {
        entries
            .entry(reply.id)
            .or_default()
            .push(Entry::Text(reply.text));
    }
}

fn failure(file: &str, line: usize, message: &str) -> Failure {
    Failure(format!("{}:{}: {}", name(file), line, message))
}

fn invalid_id(file: &str, line: usize, id: &str) -> Failure {
    failure(file, line, &format!("case {} is not valid", name(id)))
}

fn directive_remainder<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    let remainder = line.strip_prefix(word)?;
    match remainder.chars().next() {
        None | Some(' ' | '\t') => Some(remainder),
        Some(_) => None,
    }
}

fn is_blank(character: char) -> bool {
    character == ' ' || character == '\t'
}

pub fn parse(lines: &[String], file: &str) -> Result<Replies, Failure> {
    let mut entries = BTreeMap::new();
    let mut open: Option<OpenReply> = None;

    for (index, line) in lines.iter().enumerate() {
        let line_number = index + 1;

        if let Some(text) = line.strip_prefix('|') {
            let Some(reply) = open.as_mut() else {
                return Err(failure(file, line_number, "text line outside a reply"));
            };
            let fragment = text.strip_prefix(' ').unwrap_or(text);
            if reply.has_text_line {
                reply.text.push('\n');
            }
            reply.text.push_str(fragment);
            reply.has_text_line = true;
            continue;
        }

        close_reply(&mut open, &mut entries);
        let directive = trim_blanks(line);
        if directive.is_empty() || directive.starts_with('#') {
            continue;
        }

        if let Some(remainder) = directive_remainder(directive, "reply") {
            let id = trim_blanks(remainder);
            if !is_id(id) {
                return Err(invalid_id(file, line_number, id));
            }
            open = Some(OpenReply {
                id: id.to_owned(),
                text: String::new(),
                has_text_line: false,
            });
            continue;
        }

        if let Some(remainder) = directive_remainder(directive, "fail") {
            let fields = trim_blanks(remainder);
            let (id, message) = match fields.split_once(is_blank) {
                Some((id, message)) => (id, trim_blanks(message)),
                None => (fields, ""),
            };
            if !is_id(id) {
                return Err(invalid_id(file, line_number, id));
            }
            if message.is_empty() {
                return Err(failure(file, line_number, "fail needs a message"));
            }
            entries
                .entry(id.to_owned())
                .or_insert_with(Vec::new)
                .push(Entry::Fail(message.to_owned()));
            continue;
        }

        return Err(failure(
            file,
            line_number,
            "line is not reply, fail or text",
        ));
    }

    close_reply(&mut open, &mut entries);
    Ok(Replies(entries))
}
