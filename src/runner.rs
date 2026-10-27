use crate::provider::Provider;
use crate::suite::Case;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attempt {
    pub passed: bool,
    pub note: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseRun {
    pub id: String,
    pub attempts: Vec<Attempt>,
}

pub fn run(cases: &[Case], provider: &mut dyn Provider, attempts: usize) -> Vec<CaseRun> {
    cases
        .iter()
        .map(|case| {
            let mut case_attempts = Vec::new();
            for attempt_number in 1..=attempts {
                let attempt = match provider.reply(&case.id, &case.prompt, attempt_number) {
                    Err(error) => Attempt {
                        passed: false,
                        note: format!("provider: {error}"),
                    },
                    Ok(reply) => {
                        let failure = case.checks.iter().find_map(|check| check.eval(&reply));
                        match failure {
                            Some(note) => Attempt {
                                passed: false,
                                note,
                            },
                            None => Attempt {
                                passed: true,
                                note: String::new(),
                            },
                        }
                    }
                };
                case_attempts.push(attempt);
            }
            CaseRun {
                id: case.id.clone(),
                attempts: case_attempts,
            }
        })
        .collect()
}
