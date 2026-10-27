use evalpin::report::{Report, diff_report, run_report, table};
use evalpin::results::Row;
use evalpin::runner::{Attempt, CaseRun};

fn attempt(passed: bool, note: &str) -> Attempt {
    Attempt {
        passed,
        note: note.to_owned(),
    }
}

fn case_run(id: &str, attempts: Vec<Attempt>) -> CaseRun {
    CaseRun {
        id: id.to_owned(),
        attempts,
    }
}

fn row(id: &str, passed: usize, total: usize) -> Row {
    Row {
        id: id.to_owned(),
        passed,
        total,
    }
}

fn assert_report(report: Report, expected_text: &str, expected_failed: bool) {
    assert_eq!(report.text, expected_text);
    assert_eq!(report.failed, expected_failed);
}

fn assert_report_for(scenario: &str, report: Report, expected_text: &str, expected_failed: bool) {
    let expected = Report {
        text: expected_text.to_owned(),
        failed: expected_failed,
    };
    assert_eq!(report, expected, "failed report scenario: {}", scenario);
}

#[test]
fn table_formats_empty_ragged_unicode_and_trailing_space_rows() {
    assert_eq!(table(&[]), "");
    assert_eq!(table(&[vec![]]), "\n");
    assert_eq!(
        table(&[
            vec!["a".to_owned(), "b".to_owned()],
            vec!["long".to_owned(), "c".to_owned()],
        ]),
        "a     b\nlong  c\n"
    );
    assert_eq!(
        table(&[
            vec!["😀".to_owned(), "x".to_owned()],
            vec!["ab".to_owned(), "y".to_owned()],
        ]),
        "😀   x\nab  y\n"
    );
    assert_eq!(
        table(&[vec!["a".to_owned(), "b".to_owned()], vec!["z".to_owned()]]),
        "a  b\nz\n"
    );
    assert_eq!(
        table(&[vec!["a ".to_owned(), "b ".to_owned(), "".to_owned()]]),
        "a   b\n"
    );
}

#[test]
fn report_public_api_derives_clone_debug_and_equality() {
    let report = Report {
        text: "report".to_owned(),
        failed: true,
    };
    let cloned = report.clone();
    let _: &dyn std::fmt::Debug = &report;
    assert_eq!(cloned, report);
    assert_eq!(
        Report {
            text: "report".to_owned(),
            failed: true,
        },
        report
    );
}

#[test]
fn run_reports_cover_empty_partial_all_pass_all_fail_mixed_and_zero_attempts() {
    assert_report(
        run_report(&[], 2, None, &[]),
        "CASE  PASSED  RATE  ALL\nevalpin: 0 cases x 2 attempts, pass^2 0/0, pass@2 0/0\n",
        false,
    );

    assert_report(
        run_report(
            &[case_run(
                "greet",
                vec![
                    attempt(true, ""),
                    attempt(false, "reply does not contain \"Ana\""),
                ],
            )],
            2,
            None,
            &["greet".to_owned()],
        ),
        "CASE   PASSED  RATE  ALL\ngreet  1/2     50%   no\n\ngreet: attempt 2: reply does not contain \"Ana\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1\n",
        true,
    );

    assert_report(
        run_report(
            &[case_run(
                "ready",
                vec![attempt(true, ""), attempt(true, "")],
            )],
            2,
            None,
            &["ready".to_owned()],
        ),
        "CASE   PASSED  RATE  ALL\nready  2/2     100%  yes\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1\n",
        false,
    );

    assert_report(
        run_report(
            &[case_run(
                "lost",
                vec![
                    attempt(false, "first failure"),
                    attempt(false, "later failure"),
                ],
            )],
            2,
            None,
            &["lost".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL\nlost  0/2     0%    no\n\nlost: attempt 1: first failure\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 0/1\n",
        true,
    );

    assert_report(
        run_report(
            &[
                case_run("steady", vec![attempt(true, ""), attempt(true, "")]),
                case_run(
                    "mixed",
                    vec![
                        attempt(true, ""),
                        attempt(false, "first failure"),
                        attempt(false, "later failure"),
                    ],
                ),
            ],
            3,
            None,
            &["steady".to_owned(), "mixed".to_owned()],
        ),
        "CASE    PASSED  RATE  ALL\nsteady  2/2     100%  yes\nmixed   1/3     33%   no\n\nmixed: attempt 2: first failure\nevalpin: 2 cases x 3 attempts, pass^3 1/2, pass@3 2/2\n",
        true,
    );

    assert_report(
        run_report(
            &[case_run("zero", Vec::new())],
            2,
            None,
            &["zero".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL\nzero  0/0     0%    yes\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 0/1\n",
        false,
    );
}

#[test]
fn run_report_matches_the_literal_worked_baseline_example() {
    assert_report(
        run_report(
            &[
                case_run(
                    "greet",
                    vec![
                        attempt(true, ""),
                        attempt(false, "reply does not contain \"Ana\""),
                    ],
                ),
                case_run("sum", vec![attempt(true, ""), attempt(true, "")]),
                case_run("profile", vec![attempt(true, ""), attempt(true, "")]),
                case_run(
                    "brief",
                    vec![
                        attempt(false, "provider: rate limited"),
                        attempt(false, "reply has 22 chars, more than 10"),
                    ],
                ),
            ],
            2,
            Some(&[row("greet", 2, 2), row("sum", 1, 2), row("old", 1, 1)]),
            &[
                "greet".to_owned(),
                "sum".to_owned(),
                "profile".to_owned(),
                "brief".to_owned(),
            ],
        ),
        "CASE     PASSED  RATE  ALL  VS\ngreet    1/2     50%   no   worse\nsum      2/2     100%  yes  better\nprofile  2/2     100%  yes  new\nbrief    0/2     0%    no   new\nold      -       -     -    gone\n\ngreet: attempt 2: reply does not contain \"Ana\"\nbrief: attempt 1: provider: rate limited\nevalpin: 4 cases x 2 attempts, pass^2 2/4, pass@2 3/4; vs baseline: 1 better, 0 same, 1 worse, 2 new, 1 gone\n",
        true,
    );
}

#[test]
fn baseline_statuses_control_failed_independently_of_failure_notes() {
    assert_report_for(
        "a:1/2 versus a:0/2",
        run_report(
            &[case_run(
                "a",
                vec![attempt(true, ""), attempt(false, "note")],
            )],
            2,
            Some(&[row("a", 0, 2)]),
            &["a".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   better\n\na: attempt 2: note\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 1 better, 0 same, 0 worse, 0 new, 0 gone\n",
        false,
    );

    assert_report_for(
        "a:0/2 versus a:0/1",
        run_report(
            &[case_run(
                "a",
                vec![attempt(false, "same note"), attempt(false, "later")],
            )],
            2,
            Some(&[row("a", 0, 1)]),
            &["a".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     0/2     0%    no   same\n\na: attempt 1: same note\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 0/1; vs baseline: 0 better, 1 same, 0 worse, 0 new, 0 gone\n",
        false,
    );

    assert_report_for(
        "a:0/2 versus a:1/2",
        run_report(
            &[case_run(
                "a",
                vec![attempt(false, "regression"), attempt(false, "later")],
            )],
            2,
            Some(&[row("a", 1, 2)]),
            &["a".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     0/2     0%    no   worse\n\na: attempt 1: regression\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 0/1; vs baseline: 0 better, 0 same, 1 worse, 0 new, 0 gone\n",
        true,
    );

    assert_report_for(
        "a:1/2 versus []",
        run_report(
            &[case_run(
                "a",
                vec![attempt(true, ""), attempt(false, "incomplete")],
            )],
            2,
            Some(&[]),
            &["a".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   new\n\na: attempt 2: incomplete\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 0 better, 0 same, 0 worse, 1 new, 0 gone\n",
        true,
    );

    assert_report_for(
        "a:2/2 versus []",
        run_report(
            &[case_run("a", vec![attempt(true, ""), attempt(true, "")])],
            2,
            Some(&[]),
            &["a".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     2/2     100%  yes  new\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1; vs baseline: 0 better, 0 same, 0 worse, 1 new, 0 gone\n",
        false,
    );

    assert_report(
        run_report(
            &[],
            2,
            Some(&[row("z", 1, 1), row("a", 1, 1), row("kept", 1, 1)]),
            &["kept".to_owned()],
        ),
        "CASE  PASSED  RATE  ALL  VS\na     -       -     -    gone\nz     -       -     -    gone\nevalpin: 0 cases x 2 attempts, pass^2 0/0, pass@2 0/0; vs baseline: 0 better, 0 same, 0 worse, 0 new, 2 gone\n",
        false,
    );

    assert_report(
        run_report(&[], 2, Some(&[]), &[]),
        "CASE  PASSED  RATE  ALL  VS\nevalpin: 0 cases x 2 attempts, pass^2 0/0, pass@2 0/0; vs baseline: 0 better, 0 same, 0 worse, 0 new, 0 gone\n",
        false,
    );
}

#[test]
fn diff_report_matches_empty_worked_and_sorted_gone_fixtures() {
    assert_report(
        diff_report(&[], &[]),
        "CASE  OLD  NEW  VS\nevalpin: 0 cases, 0 better, 0 same, 0 worse, 0 new, 0 gone\n",
        false,
    );

    assert_report(
        diff_report(
            &[row("greet", 2, 2), row("sum", 1, 2), row("old", 1, 1)],
            &[
                row("greet", 1, 2),
                row("sum", 2, 2),
                row("profile", 2, 2),
                row("brief", 0, 2),
            ],
        ),
        "CASE     OLD  NEW  VS\ngreet    2/2  1/2  worse\nsum      1/2  2/2  better\nprofile  -    2/2  new\nbrief    -    0/2  new\nold      1/1  -    gone\nevalpin: 5 cases, 1 better, 0 same, 1 worse, 2 new, 1 gone\n",
        true,
    );

    assert_report(
        diff_report(
            &[row("z", 1, 1), row("a", 1, 1), row("s", 1, 2)],
            &[row("s", 2, 4), row("n", 0, 1)],
        ),
        "CASE  OLD  NEW  VS\ns     1/2  2/4  same\nn     -    0/1  new\na     1/1  -    gone\nz     1/1  -    gone\nevalpin: 4 cases, 0 better, 1 same, 0 worse, 1 new, 2 gone\n",
        false,
    );
}
