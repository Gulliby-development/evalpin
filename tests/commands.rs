#![allow(clippy::type_complexity)]

use evalpin::cli;
use evalpin::io::Io;
use std::cell::RefCell;

type Event = (char, String);

#[derive(Default)]
struct FakeIo {
    files: Vec<(String, Vec<u8>)>,
    writes: Vec<(String, String)>,
    outputs: Vec<String>,
    errors: Vec<String>,
    events: RefCell<Vec<Event>>,
    refuse_writes: bool,
}

impl Io for FakeIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.events.borrow_mut().push(('r', path.to_owned()));
        self.files
            .iter()
            .find(|(name, _)| name == path)
            .map(|(_, bytes)| bytes.clone())
    }

    fn write(&mut self, path: &str, text: &str) -> bool {
        self.events.borrow_mut().push(('w', path.to_owned()));
        self.writes.push((path.to_owned(), text.to_owned()));
        !self.refuse_writes
    }

    fn out(&mut self, text: &str) {
        self.events.borrow_mut().push(('o', text.to_owned()));
        self.outputs.push(text.to_owned());
    }

    fn err(&mut self, text: &str) {
        self.events.borrow_mut().push(('e', text.to_owned()));
        self.errors.push(text.to_owned());
    }
}

fn io_with(files: &[(&str, &str)]) -> FakeIo {
    FakeIo {
        files: files
            .iter()
            .map(|(path, contents)| ((*path).to_owned(), contents.as_bytes().to_vec()))
            .collect(),
        ..FakeIo::default()
    }
}

#[test]
fn only_selects_in_suite_order_deduplicates_and_keeps_excluded_baseline_cases_out_of_gone() {
    let suite = "case a\nprompt: p\nexpect equals: ok\ncase b\nprompt: p\nexpect equals: ok\ncase c\nprompt: p\nexpect equals: ok\n";
    let replies = "reply a\n| ok\nreply b\n| ok\nreply c\n| ok\n";
    let baseline = "evalpin-results 1\na 1 1\nb 1 1\nc 1 1\nz 1 1\nd 1 1\n";
    let expected = "CASE  PASSED  RATE  ALL  VS\na     1/1     100%  yes  same\nc     1/1     100%  yes  same\nd     -       -     -    gone\nz     -       -     -    gone\nevalpin: 2 cases x 1 attempts, pass^1 2/2, pass@1 2/2; vs baseline: 0 better, 2 same, 0 worse, 0 new, 2 gone\n";
    let args = [
        "run",
        "s",
        "--replies",
        "r",
        "--only",
        "c",
        "--only",
        "a",
        "--only",
        "c",
        "--baseline",
        "b",
        "--out",
        "o",
    ]
    .map(str::to_owned);
    let mut io = io_with(&[("s", suite), ("r", replies), ("b", baseline)]);

    assert_eq!(cli::run(&args, &mut io), 0);
    assert_eq!(
        io.writes,
        vec![(
            "o".to_owned(),
            "evalpin-results 1\na 1 1\nc 1 1\n".to_owned()
        )]
    );
    assert_eq!(io.outputs, vec![expected.to_owned()]);
    assert!(io.errors.is_empty());
    assert_eq!(
        io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('r', "b".to_owned()),
            ('w', "o".to_owned()),
            ('o', expected.to_owned()),
        ]
    );
}

#[test]
fn baseline_and_no_baseline_outcomes_match_all_eight_fraction_cases() {
    let suite = "case a\nprompt: p\nexpect contains: yes\n";
    let mixed_replies = "reply a\n| yes\nreply a\n| no\n";
    let passing_replies = "reply a\n| yes\nreply a\n| yes\n";
    let no_baseline = "evalpin-results 1\n";
    let before_zero = "evalpin-results 1\na 0 2\n";
    let before_one = "evalpin-results 1\na 1 2\n";
    let before_two = "evalpin-results 1\na 2 2\n";
    let before_gone = "evalpin-results 1\na 2 2\nz 1 1\n";
    // 1/2; none
    // 2/2; none
    // 1/2; missing baseline id
    // 2/2; missing baseline id
    let cases: &[(&[&str], &str, &str, i32, &str, &[&str])] = &[
        (
            &["run", "s", "--replies", "r", "-k", "2"],
            mixed_replies,
            no_baseline,
            1,
            "CASE  PASSED  RATE  ALL\na     1/2     50%   no\n\na: attempt 2: reply does not contain \"yes\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1\n",
            &["s", "r"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2"],
            passing_replies,
            no_baseline,
            0,
            "CASE  PASSED  RATE  ALL\na     2/2     100%  yes\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1\n",
            &["s", "r"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            mixed_replies,
            before_zero,
            0,
            "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   better\n\na: attempt 2: reply does not contain \"yes\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 1 better, 0 same, 0 worse, 0 new, 0 gone\n",
            &["s", "r", "b"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            mixed_replies,
            before_one,
            0,
            "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   same\n\na: attempt 2: reply does not contain \"yes\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 0 better, 1 same, 0 worse, 0 new, 0 gone\n",
            &["s", "r", "b"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            mixed_replies,
            before_two,
            1,
            "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   worse\n\na: attempt 2: reply does not contain \"yes\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 0 better, 0 same, 1 worse, 0 new, 0 gone\n",
            &["s", "r", "b"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            mixed_replies,
            no_baseline,
            1,
            "CASE  PASSED  RATE  ALL  VS\na     1/2     50%   no   new\n\na: attempt 2: reply does not contain \"yes\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1; vs baseline: 0 better, 0 same, 0 worse, 1 new, 0 gone\n",
            &["s", "r", "b"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            passing_replies,
            no_baseline,
            0,
            "CASE  PASSED  RATE  ALL  VS\na     2/2     100%  yes  new\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1; vs baseline: 0 better, 0 same, 0 worse, 1 new, 0 gone\n",
            &["s", "r", "b"],
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "--baseline", "b"],
            passing_replies,
            before_gone,
            0,
            "CASE  PASSED  RATE  ALL  VS\na     2/2     100%  yes  same\nz     -       -     -    gone\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1; vs baseline: 0 better, 1 same, 0 worse, 0 new, 1 gone\n",
            &["s", "r", "b"],
        ),
    ];

    for (words, replies, baseline, expected_status, expected_output, read_paths) in cases {
        let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        let mut io = io_with(&[("s", suite), ("r", replies), ("b", baseline)]);

        assert_eq!(cli::run(&args, &mut io), *expected_status);
        assert_eq!(io.outputs, vec![(*expected_output).to_owned()]);
        assert!(io.errors.is_empty());
        assert!(io.writes.is_empty());

        let expected_reads: Vec<Event> = read_paths
            .iter()
            .map(|path| ('r', (*path).to_owned()))
            .collect();
        assert_eq!(io.events.borrow().len(), expected_reads.len() + 1);
        for (actual, expected_read) in io.events.borrow().iter().zip(expected_reads.iter()) {
            assert_eq!(actual, expected_read);
        }
        assert_eq!(
            io.events.borrow().last(),
            Some(&('o', (*expected_output).to_owned()))
        );
    }
}

#[test]
fn gone_baseline_ids_do_not_fail_a_passing_one_attempt_run() {
    // missing suite id; 1/1
    let suite = "case a\nprompt: p\nexpect equals: ok\n";
    let replies = "reply a\n| ok\n";
    let baseline = "evalpin-results 1\nz 1 1\n";
    let expected = "CASE  PASSED  RATE  ALL  VS\na     1/1     100%  yes  new\nz     -       -     -    gone\nevalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1; vs baseline: 0 better, 0 same, 0 worse, 1 new, 1 gone\n";
    let args = ["run", "s", "--replies", "r", "--baseline", "b"].map(str::to_owned);
    let mut io = io_with(&[("s", suite), ("r", replies), ("b", baseline)]);

    assert_eq!(cli::run(&args, &mut io), 0);
    assert_eq!(io.outputs, vec![expected.to_owned()]);
    assert!(io.errors.is_empty());
    assert!(io.writes.is_empty());
    assert_eq!(
        io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('r', "b".to_owned()),
            ('o', expected.to_owned()),
        ]
    );
}

#[test]
fn diff_handles_empty_equal_fraction_ordering_and_regression_reports() {
    let cases: &[(&str, &str, i32, &str)] = &[
        (
            "evalpin-results 1\n",
            "evalpin-results 1\n",
            0,
            "CASE  OLD  NEW  VS\nevalpin: 0 cases, 0 better, 0 same, 0 worse, 0 new, 0 gone\n",
        ),
        (
            "evalpin-results 1\nz 1 1\na 1 1\nb 1 2\n",
            "evalpin-results 1\nb 2 4\nc 0 1\n",
            0,
            "CASE  OLD  NEW  VS\nb     1/2  2/4  same\nc     -    0/1  new\na     1/1  -    gone\nz     1/1  -    gone\nevalpin: 4 cases, 0 better, 1 same, 0 worse, 1 new, 2 gone\n",
        ),
        (
            "evalpin-results 1\na 1 1\n",
            "evalpin-results 1\na 0 1\n",
            1,
            "CASE  OLD  NEW  VS\na     1/1  0/1  worse\nevalpin: 1 cases, 0 better, 0 same, 1 worse, 0 new, 0 gone\n",
        ),
    ];

    for (old, new, expected_status, expected_output) in cases {
        let mut io = io_with(&[("old", old), ("new", new)]);
        let args = ["diff", "old", "new"].map(str::to_owned);

        assert_eq!(cli::run(&args, &mut io), *expected_status);
        assert_eq!(io.outputs, vec![(*expected_output).to_owned()]);
        assert!(io.errors.is_empty());
        assert!(io.writes.is_empty());
        assert_eq!(
            io.events.borrow().as_slice(),
            &[
                ('r', "old".to_owned()),
                ('r', "new".to_owned()),
                ('o', (*expected_output).to_owned()),
            ]
        );
    }
}

#[test]
fn output_is_written_before_the_single_report_for_passing_and_failing_runs() {
    let suite = "case a\nprompt: p\nexpect equals: ok\n";
    let passing_replies = "reply a\n| ok\n";
    let failing_replies = "reply a\n| no\n";
    let passing_output = "CASE  PASSED  RATE  ALL\na     1/1     100%  yes\nevalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1\n";
    let failing_output = "CASE  PASSED  RATE  ALL\na     0/1     0%    no\n\na: attempt 1: reply \"no\" is not \"ok\"\nevalpin: 1 cases x 1 attempts, pass^1 0/1, pass@1 0/1\n";
    let args = ["run", "s", "--replies", "r", "--out", "o"].map(str::to_owned);

    let mut passing_io = io_with(&[("s", suite), ("r", passing_replies)]);
    assert_eq!(cli::run(&args, &mut passing_io), 0);
    assert_eq!(
        passing_io.writes,
        vec![("o".to_owned(), "evalpin-results 1\na 1 1\n".to_owned())]
    );
    assert_eq!(passing_io.outputs, vec![passing_output.to_owned()]);
    assert!(passing_io.errors.is_empty());
    assert_eq!(
        passing_io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('w', "o".to_owned()),
            ('o', passing_output.to_owned()),
        ]
    );

    let mut failing_io = io_with(&[("s", suite), ("r", failing_replies)]);
    assert_eq!(cli::run(&args, &mut failing_io), 1);
    assert_eq!(
        failing_io.writes,
        vec![("o".to_owned(), "evalpin-results 1\na 0 1\n".to_owned())]
    );
    assert_eq!(failing_io.outputs, vec![failing_output.to_owned()]);
    assert!(failing_io.errors.is_empty());
    assert_eq!(
        failing_io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('w', "o".to_owned()),
            ('o', failing_output.to_owned()),
        ]
    );
}

#[test]
fn refused_output_writes_report_the_safe_path_and_never_emit_a_report() {
    let suite = "case a\nprompt: p\nexpect equals: ok\n";
    let replies = "reply a\n| ok\n";
    let cases: &[(&[&str], &str, &str)] = &[
        (
            &["run", "s", "--replies", "r", "--out", "o"],
            "o",
            "evalpin: cannot write o\n",
        ),
        (
            &["run", "s", "--replies", "r", "--out", ""],
            "",
            "evalpin: cannot write (empty)\n",
        ),
    ];

    for (words, path, expected_error) in cases {
        let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        let mut io = io_with(&[("s", suite), ("r", replies)]);
        io.refuse_writes = true;

        assert_eq!(cli::run(&args, &mut io), 2);
        assert_eq!(
            io.writes,
            vec![((*path).to_owned(), "evalpin-results 1\na 1 1\n".to_owned())]
        );
        assert!(io.outputs.is_empty());
        assert_eq!(io.errors, vec![(*expected_error).to_owned()]);
        assert_eq!(
            io.events.borrow().as_slice(),
            &[
                ('r', "s".to_owned()),
                ('r', "r".to_owned()),
                ('w', (*path).to_owned()),
                ('e', (*expected_error).to_owned()),
            ]
        );
    }
}

#[test]
fn absent_output_and_earlier_filter_errors_do_not_write_files() {
    let suite = "case a\nprompt: p\nexpect equals: ok\n";
    let replies = "reply a\n| ok\n";
    let output = "CASE  PASSED  RATE  ALL\na     1/1     100%  yes\nevalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1\n";
    let args_without_out = ["run", "s", "--replies", "r"].map(str::to_owned);
    let mut no_out_io = io_with(&[("s", suite), ("r", replies)]);

    assert_eq!(cli::run(&args_without_out, &mut no_out_io), 0);
    assert!(no_out_io.writes.is_empty());
    assert_eq!(no_out_io.outputs, vec![output.to_owned()]);
    assert!(no_out_io.errors.is_empty());
    assert_eq!(
        no_out_io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('o', output.to_owned()),
        ]
    );

    let args_with_filter = [
        "run",
        "s",
        "--replies",
        "r",
        "--only",
        "absent",
        "--out",
        "o",
    ]
    .map(str::to_owned);
    let mut filter_io = io_with(&[("s", suite), ("r", replies)]);

    assert_eq!(cli::run(&args_with_filter, &mut filter_io), 2);
    assert!(filter_io.writes.is_empty());
    assert!(filter_io.outputs.is_empty());
    assert_eq!(
        filter_io.errors,
        vec!["evalpin: unknown case absent\n".to_owned()]
    );
    assert_eq!(
        filter_io.events.borrow().as_slice(),
        &[
            ('r', "s".to_owned()),
            ('r', "r".to_owned()),
            ('e', "evalpin: unknown case absent\n".to_owned()),
        ]
    );
}

#[test]
fn readme_runs_use_the_scripted_fixtures_and_the_first_result_as_baseline() {
    let mut first = FakeIo {
        files: vec![
            (
                "suite.txt".to_owned(),
                include_bytes!("../fixtures/readme/suite.txt").to_vec(),
            ),
            (
                "replies.txt".to_owned(),
                include_bytes!("../fixtures/readme/replies.txt").to_vec(),
            ),
        ],
        ..FakeIo::default()
    };
    let first_args = [
        "run",
        "suite.txt",
        "--replies",
        "replies.txt",
        "-k",
        "2",
        "--out",
        "first.txt",
    ]
    .map(str::to_owned);
    let first_output = "CASE   PASSED  RATE  ALL\ngreet  1/2     50%   no\n\ngreet: attempt 2: reply does not contain \"Ana\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1\n";

    assert_eq!(cli::run(&first_args, &mut first), 1);
    assert_eq!(
        first.writes,
        vec![(
            "first.txt".to_owned(),
            "evalpin-results 1\ngreet 1 2\n".to_owned()
        )]
    );
    assert_eq!(first.outputs, vec![first_output.to_owned()]);
    assert!(first.errors.is_empty());
    assert_eq!(
        first.events.borrow().as_slice(),
        &[
            ('r', "suite.txt".to_owned()),
            ('r', "replies.txt".to_owned()),
            ('w', "first.txt".to_owned()),
            ('o', first_output.to_owned()),
        ]
    );

    let first_results = match first.writes.first() {
        Some((_, text)) => text.as_bytes().to_vec(),
        None => Vec::new(),
    };
    assert_eq!(first_results, b"evalpin-results 1\ngreet 1 2\n".to_vec());

    let mut improved = FakeIo {
        files: vec![
            (
                "suite.txt".to_owned(),
                include_bytes!("../fixtures/readme/suite.txt").to_vec(),
            ),
            (
                "replies.txt".to_owned(),
                include_bytes!("../fixtures/readme/improved-replies.txt").to_vec(),
            ),
            ("first.txt".to_owned(), first_results),
        ],
        ..FakeIo::default()
    };
    let improved_args = [
        "run",
        "suite.txt",
        "--replies",
        "replies.txt",
        "-k",
        "2",
        "--out",
        "second.txt",
        "--baseline",
        "first.txt",
    ]
    .map(str::to_owned);
    let improved_output = "CASE   PASSED  RATE  ALL  VS\ngreet  2/2     100%  yes  better\nevalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1; vs baseline: 1 better, 0 same, 0 worse, 0 new, 0 gone\n";

    assert_eq!(cli::run(&improved_args, &mut improved), 0);
    assert_eq!(
        improved.writes,
        vec![(
            "second.txt".to_owned(),
            "evalpin-results 1\ngreet 2 2\n".to_owned()
        )]
    );
    assert_eq!(improved.outputs, vec![improved_output.to_owned()]);
    assert!(improved.errors.is_empty());
    assert_eq!(
        improved.events.borrow().as_slice(),
        &[
            ('r', "suite.txt".to_owned()),
            ('r', "replies.txt".to_owned()),
            ('r', "first.txt".to_owned()),
            ('w', "second.txt".to_owned()),
            ('o', improved_output.to_owned()),
        ]
    );
}
