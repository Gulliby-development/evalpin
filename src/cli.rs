use crate::io::{self, Io};
use crate::options::{self, Spec};
use crate::provider::Scripted;
use crate::replies;
use crate::report;
use crate::results::{self, Row};
use crate::runner;
use crate::score::Score;
use crate::suite;
use crate::text::{self, Failure};

const HELP: &str = "usage: evalpin run SUITE --replies FILE [-k N] [--only ID]... [--out FILE] [--baseline FILE]\n       evalpin diff OLD NEW [--show all|changes]\n       evalpin --help\n";

pub fn run(args: &[String], io: &mut dyn Io) -> i32 {
    let Some((command, command_args)) = args.split_first() else {
        return emit_error(io, "missing command (run or diff)");
    };

    if command == "--help" {
        if command_args.is_empty() {
            io.out(HELP);
            return 0;
        }
        return emit_error(io, &format!("unknown command {}", text::name(command)));
    }

    let result = match command.as_str() {
        "run" => run_command(command_args, io),
        "diff" => diff_command(command_args, io),
        _ => Err(Failure(format!("unknown command {}", text::name(command)))),
    };

    match result {
        Ok((output, status)) => {
            io.out(&output);
            status
        }
        Err(failure) => emit_error(io, &failure.0),
    }
}

fn run_command(args: &[String], io: &mut dyn Io) -> Result<(String, i32), Failure> {
    let specs = [
        Spec {
            name: "--replies",
            repeat: false,
        },
        Spec {
            name: "-k",
            repeat: false,
        },
        Spec {
            name: "--only",
            repeat: true,
        },
        Spec {
            name: "--out",
            repeat: false,
        },
        Spec {
            name: "--baseline",
            repeat: false,
        },
    ];
    let parsed = options::parse(args, &specs, 1)?;
    let suite_path = parsed
        .operands
        .first()
        .ok_or_else(|| Failure("missing SUITE".to_owned()))?;
    let replies_path = parsed
        .value("--replies")
        .ok_or_else(|| Failure("missing --replies".to_owned()))?;

    let attempt_text = parsed.value("-k").unwrap_or("1");
    if !text::is_digits(attempt_text, 2) {
        return Err(invalid_attempts(attempt_text));
    }
    let attempts = match attempt_text.parse::<usize>() {
        Ok(value) if (1..=20).contains(&value) => value,
        _ => return Err(invalid_attempts(attempt_text)),
    };

    let suite_lines = io::read_lines(io, suite_path)?;
    let cases = suite::parse(&suite_lines, suite_path)?;
    let reply_lines = io::read_lines(io, replies_path)?;
    let scripted_replies = replies::parse(&reply_lines, replies_path)?;

    let baseline = match parsed.value("--baseline") {
        Some(path) => {
            let lines = io::read_lines(io, path)?;
            Some(results::read(&lines, path)?)
        }
        None => None,
    };

    let only_ids = parsed.all("--only");
    for id in &only_ids {
        if !cases.iter().any(|case| case.id == *id) {
            return Err(Failure(format!("unknown case {}", text::name(id))));
        }
    }

    let selected_cases: Vec<_> = cases
        .iter()
        .filter(|case| only_ids.is_empty() || only_ids.iter().any(|id| *id == case.id))
        .cloned()
        .collect();
    let suite_ids: Vec<String> = cases.iter().map(|case| case.id.clone()).collect();

    let mut provider = Scripted::new(scripted_replies);
    let runs = runner::run(&selected_cases, &mut provider, attempts);
    let rows: Vec<Row> = runs
        .iter()
        .map(|case_run| {
            let score = Score::of(case_run);
            Row {
                id: case_run.id.clone(),
                passed: score.passed,
                total: score.total,
            }
        })
        .collect();
    let report = report::run_report(&runs, attempts, baseline.as_deref(), &suite_ids);

    if let Some(path) = parsed.value("--out")
        && !io.write(path, &results::write(&rows))
    {
        return Err(Failure(format!("cannot write {}", text::name(path))));
    }

    Ok((report.text, if report.failed { 1 } else { 0 }))
}

fn diff_command(args: &[String], io: &mut dyn Io) -> Result<(String, i32), Failure> {
    let specs = [Spec {
        name: "--show",
        repeat: false,
    }];
    let parsed = options::parse(args, &specs, 2)?;
    let changes_only = match parsed.value("--show") {
        None | Some("all") => false,
        Some("changes") => true,
        Some(other) => {
            return Err(Failure(format!(
                "--show needs all or changes, got {}",
                text::name(other)
            )));
        }
    };
    let old_path = parsed
        .operands
        .first()
        .ok_or_else(|| Failure("missing OLD".to_owned()))?;
    let new_path = parsed
        .operands
        .get(1)
        .ok_or_else(|| Failure("missing NEW".to_owned()))?;

    let old_lines = io::read_lines(io, old_path)?;
    let old = results::read(&old_lines, old_path)?;
    let new_lines = io::read_lines(io, new_path)?;
    let new = results::read(&new_lines, new_path)?;
    let report = report::diff_report_shown(&old, &new, changes_only);
    Ok((report.text, if report.failed { 1 } else { 0 }))
}

fn invalid_attempts(value: &str) -> Failure {
    Failure(format!(
        "-k needs a whole number from 1 to 20, got {}",
        text::name(value)
    ))
}

fn emit_error(io: &mut dyn Io, message: &str) -> i32 {
    io.err(&format!("evalpin: {message}\n"));
    2
}
