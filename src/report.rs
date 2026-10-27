use crate::compare::{self, Vs};
use crate::results::Row;
use crate::runner::CaseRun;
use crate::score::Score;
use crate::text;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub text: String,
    pub failed: bool,
}

pub fn table(rows: &[Vec<String>]) -> String {
    let mut widths = Vec::new();
    for row in rows {
        if row.len() > widths.len() {
            widths.resize(row.len(), 0);
        }
        for (column, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(column) {
                *width = (*width).max(text::points(cell));
            }
        }
    }

    let mut output = String::new();
    for row in rows {
        let mut rendered = String::new();
        for column in 0..widths.len() {
            if column > 0 {
                rendered.push_str("  ");
            }
            let cell = row.get(column).map(String::as_str).unwrap_or("");
            rendered.push_str(cell);
            let width = widths.get(column).copied().unwrap_or(0);
            for _ in 0..width.saturating_sub(text::points(cell)) {
                rendered.push(' ');
            }
        }
        while rendered.ends_with(' ') {
            rendered.pop();
        }
        output.push_str(&rendered);
        output.push('\n');
    }
    output
}

pub fn diff_report(old: &[Row], new: &[Row]) -> Report {
    let mut rows = vec![vec![
        "CASE".to_owned(),
        "OLD".to_owned(),
        "NEW".to_owned(),
        "VS".to_owned(),
    ]];
    let mut counts = [0usize; 5];
    let mut failed = false;

    for new_row in new {
        let old_row = old.iter().find(|row| row.id == new_row.id);
        let comparison = compare::vs(
            Some((new_row.passed, new_row.total)),
            old_row.map(|row| (row.passed, row.total)),
        );
        record_comparison(comparison.clone(), &mut counts);
        if comparison == Vs::Worse {
            failed = true;
        }
        rows.push(vec![
            new_row.id.clone(),
            old_row.map(row_cell).unwrap_or_else(|| "-".to_owned()),
            row_cell(new_row),
            comparison.word().to_owned(),
        ]);
    }

    let mut gone: Vec<&Row> = old
        .iter()
        .filter(|old_row| !new.iter().any(|new_row| new_row.id == old_row.id))
        .collect();
    gone.sort_by(|left, right| left.id.chars().cmp(right.id.chars()));
    for old_row in gone {
        let comparison = compare::vs(None, Some((old_row.passed, old_row.total)));
        record_comparison(comparison.clone(), &mut counts);
        rows.push(vec![
            old_row.id.clone(),
            row_cell(old_row),
            "-".to_owned(),
            comparison.word().to_owned(),
        ]);
    }

    let case_count = rows.len().saturating_sub(1);
    let mut output = table(&rows);
    output.push_str(&format!(
        "evalpin: {case_count} cases, {} better, {} same, {} worse, {} new, {} gone\n",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    ));

    Report {
        text: output,
        failed,
    }
}

pub fn run_report(
    runs: &[CaseRun],
    attempts: usize,
    baseline: Option<&[Row]>,
    suite_ids: &[String],
) -> Report {
    let mut rows = vec![vec![
        "CASE".to_owned(),
        "PASSED".to_owned(),
        "RATE".to_owned(),
        "ALL".to_owned(),
    ]];
    if baseline.is_some() {
        rows[0].push("VS".to_owned());
    }

    let mut scores = Vec::with_capacity(runs.len());
    let mut comparisons = Vec::with_capacity(runs.len());
    let mut counts = [0usize; 5];

    for run in runs {
        let score = Score::of(run);
        let comparison = baseline.map(|baseline_rows| {
            let before = baseline_rows
                .iter()
                .find(|row| row.id == run.id)
                .map(|row| (row.passed, row.total));
            compare::vs(Some((score.passed, score.total)), before)
        });
        if let Some(comparison) = comparison.as_ref() {
            record_comparison(comparison.clone(), &mut counts);
        }

        let mut row = vec![
            run.id.clone(),
            score.cell(),
            format!("{}%", score.rate()),
            if score.all() {
                "yes".to_owned()
            } else {
                "no".to_owned()
            },
        ];
        if let Some(comparison) = comparison.as_ref() {
            row.push(comparison.clone().word().to_owned());
        }
        rows.push(row);
        scores.push(score);
        comparisons.push(comparison);
    }

    let mut gone = Vec::new();
    if let Some(baseline_rows) = baseline {
        for row in baseline_rows {
            if !suite_ids.iter().any(|id| id == &row.id) {
                gone.push(row);
            }
        }
        gone.sort_by(|left, right| left.id.chars().cmp(right.id.chars()));
        for row in &gone {
            let comparison = compare::vs(None, Some((row.passed, row.total)));
            record_comparison(comparison, &mut counts);
            rows.push(vec![
                row.id.clone(),
                "-".to_owned(),
                "-".to_owned(),
                "-".to_owned(),
                Vs::Gone.word().to_owned(),
            ]);
        }
    }

    let mut output = table(&rows);
    let mut has_failure_note = false;
    for run in runs {
        if run.attempts.iter().any(|attempt| !attempt.passed) {
            has_failure_note = true;
            break;
        }
    }
    if has_failure_note {
        output.push('\n');
        for run in runs {
            if let Some((index, attempt)) = run
                .attempts
                .iter()
                .enumerate()
                .find(|(_, attempt)| !attempt.passed)
            {
                output.push_str(&format!(
                    "{}: attempt {}: {}\n",
                    run.id,
                    index.saturating_add(1),
                    attempt.note
                ));
            }
        }
    }

    let all = scores.iter().filter(|score| score.all()).count();
    let some = scores.iter().filter(|score| score.some()).count();
    output.push_str(&format!(
        "evalpin: {} cases x {} attempts, pass^{} {}/{}, pass@{} {}/{}",
        runs.len(),
        attempts,
        attempts,
        all,
        runs.len(),
        attempts,
        some,
        runs.len()
    ));

    if baseline.is_some() {
        output.push_str(&format!(
            "; vs baseline: {} better, {} same, {} worse, {} new, {} gone",
            counts[0], counts[1], counts[2], counts[3], counts[4]
        ));
    }
    output.push('\n');

    let failed =
        match baseline {
            None => scores.iter().any(|score| !score.all()),
            Some(_) => scores
                .iter()
                .zip(comparisons.iter())
                .any(|(score, comparison)| match comparison {
                    Some(Vs::Worse) => true,
                    Some(Vs::New) => !score.all(),
                    _ => false,
                }),
        };

    Report {
        text: output,
        failed,
    }
}

fn row_cell(row: &Row) -> String {
    format!("{}/{}", row.passed, row.total)
}

fn record_comparison(comparison: Vs, counts: &mut [usize; 5]) {
    let index = match comparison {
        Vs::Better => 0,
        Vs::Same => 1,
        Vs::Worse => 2,
        Vs::New => 3,
        Vs::Gone => 4,
    };
    if let Some(count) = counts.get_mut(index) {
        *count = count.saturating_add(1);
    }
}
