use crate::replies::{Entry, Replies};
use crate::text::{disp, name};

pub trait Provider {
    fn reply(&mut self, case_id: &str, prompt: &str, attempt: usize) -> Result<String, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scripted {
    replies: Replies,
}

impl Scripted {
    pub fn new(replies: Replies) -> Scripted {
        Scripted { replies }
    }
}

impl Provider for Scripted {
    fn reply(&mut self, case_id: &str, _prompt: &str, attempt: usize) -> Result<String, String> {
        let Some(entries) = self.replies.0.get(case_id) else {
            return Err(format!("no reply for case {}", name(case_id)));
        };
        if entries.is_empty() {
            return Err(format!("no reply for case {}", name(case_id)));
        }

        let index = attempt.saturating_sub(1) % entries.len();
        match entries.get(index) {
            Some(Entry::Text(text)) => Ok(text.clone()),
            Some(Entry::Fail(message)) => Err(disp(message)),
            None => Err(format!("no reply for case {}", name(case_id))),
        }
    }
}
