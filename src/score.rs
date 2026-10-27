use crate::runner::CaseRun;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Score {
    pub passed: usize,
    pub total: usize,
}

impl Score {
    pub fn of(run: &CaseRun) -> Score {
        Score {
            passed: run.attempts.iter().filter(|attempt| attempt.passed).count(),
            total: run.attempts.len(),
        }
    }

    pub fn rate(&self) -> usize {
        if self.total == 0 {
            return 0;
        }

        let passed = self.passed as u128;
        let total = self.total as u128;
        let rate = (200 * passed + total) / (2 * total);
        match usize::try_from(rate) {
            Ok(rate) => rate,
            Err(_) => usize::MAX,
        }
    }

    pub fn all(&self) -> bool {
        self.passed == self.total
    }

    pub fn some(&self) -> bool {
        self.passed >= 1
    }

    pub fn cell(&self) -> String {
        format!("{}/{}", self.passed, self.total)
    }
}
