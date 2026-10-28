use evalpin::cli::run;
use evalpin::io::Io;
use std::cell::RefCell;

#[derive(Default)]
struct FakeIo {
    files: Vec<(String, Vec<u8>)>,
    reads: RefCell<Vec<String>>,
    writes: Vec<(String, String)>,
    outputs: Vec<String>,
    errors: Vec<String>,
    events: Vec<&'static str>,
    refuse_writes: bool,
}

impl Io for FakeIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.reads.borrow_mut().push(path.to_owned());
        self.files
            .iter()
            .find(|(name, _)| name == path)
            .map(|(_, bytes)| bytes.clone())
    }

    fn write(&mut self, path: &str, text: &str) -> bool {
        self.events.push("write");
        self.writes.push((path.to_owned(), text.to_owned()));
        !self.refuse_writes
    }

    fn out(&mut self, text: &str) {
        self.events.push("out");
        self.outputs.push(text.to_owned());
    }

    fn err(&mut self, text: &str) {
        self.events.push("err");
        self.errors.push(text.to_owned());
    }
}

fn standard_io() -> FakeIo {
    FakeIo {
        files: vec![
            (
                "s".to_owned(),
                b"case a\nprompt: p\nexpect equals: x\n".to_vec(),
            ),
            ("r".to_owned(), b"reply a\n| x\n".to_vec()),
            ("b".to_owned(), b"evalpin-results 1\n".to_vec()),
            ("bad".to_owned(), vec![0xFF]),
        ],
        ..FakeIo::default()
    }
}

fn invoke(io: &mut FakeIo, words: &[&str]) -> i32 {
    let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    run(&args, io)
}

fn assert_cli_error(words: &[&str], message: &str) {
    let mut io = standard_io();
    assert_eq!(invoke(&mut io, words), 2, "arguments: {words:?}");
    assert!(io.outputs.is_empty(), "arguments: {words:?}");
    assert_eq!(io.errors, vec![message], "arguments: {words:?}");
    assert!(io.writes.is_empty(), "arguments: {words:?}");
    assert_eq!(io.events, vec!["err"], "arguments: {words:?}");
}

#[test]
fn help_and_command_diagnostics_use_one_exact_io_call() {
    let mut io = standard_io();
    assert_eq!(invoke(&mut io, &["--help"]), 0);
    assert_eq!(
        io.outputs,
        vec![
            "usage: evalpin run SUITE --replies FILE [-k N] [--only ID]... [--out FILE] [--baseline FILE]\n       evalpin diff OLD NEW [--show all|changes]\n       evalpin --help\n"
        ]
    );
    assert!(io.errors.is_empty());
    assert!(io.writes.is_empty());
    assert!(io.reads.borrow().is_empty());
    assert_eq!(io.events, vec!["out"]);

    let command_errors: &[(&[&str], &str)] = &[
        (&[], "evalpin: missing command (run or diff)\n"),
        (&["--help", "x"], "evalpin: unknown command --help\n"),
        (&[""], "evalpin: unknown command (empty)\n"),
    ];
    for (words, expected) in command_errors {
        assert_cli_error(words, expected);
    }
}

#[test]
fn argument_table_diagnostics_are_exact_and_have_no_early_side_effects() {
    let cases: &[(&[&str], &str)] = &[
        (&["run"], "evalpin: missing SUITE\n"),
        (&["run", "s"], "evalpin: missing --replies\n"),
        (
            &["run", "s", "--replies"],
            "evalpin: --replies needs a value\n",
        ),
        (
            &["run", "s", "--replies", "r", "--replies", "r"],
            "evalpin: --replies given twice\n",
        ),
        (
            &["run", "s", "--replies=r"],
            "evalpin: unknown option --replies=r\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "0"],
            "evalpin: -k needs a whole number from 1 to 20, got 0\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "x\t"],
            "evalpin: -k needs a whole number from 1 to 20, got x?\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "2", "-k", "3"],
            "evalpin: -k given twice\n",
        ),
        (
            &["run", "nosuch", "--replies", "r"],
            "evalpin: nosuch: cannot be read\n",
        ),
        (
            &["run", "bad", "--replies", "r"],
            "evalpin: bad: not valid UTF-8\n",
        ),
        (
            &["run", "s", "--replies", "r", "--only", "a", "--only", ""],
            "evalpin: unknown case (empty)\n",
        ),
        (
            &["run", "", "--replies", "r"],
            "evalpin: (empty): cannot be read\n",
        ),
        (&["diff"], "evalpin: missing OLD\n"),
        (&["diff", "b"], "evalpin: missing NEW\n"),
        (&["diff", "b", "b", "x"], "evalpin: unexpected argument x\n"),
        (&["diff", "b", "-k", "2"], "evalpin: unknown option -k\n"),
        (
            &["run", "s", "-k5", "--replies", "r"],
            "evalpin: unexpected argument -k5\n",
        ),
        (
            &["run", "s", "--out=x", "--replies", "r"],
            "evalpin: unknown option --out=x\n",
        ),
        (
            &["run", "s", "-x", "--replies", "r"],
            "evalpin: unexpected argument -x\n",
        ),
        (
            &["run", "s", "-", "--replies", "r"],
            "evalpin: unexpected argument -\n",
        ),
    ];

    for (words, expected) in cases {
        assert_cli_error(words, expected);
    }
}

#[test]
fn attempts_validate_ascii_boundaries_and_accept_option_ordering() {
    let attempts: &[(&[&str], &str, &str)] = &[
        (
            &["run", "s", "--replies", "r", "-k", "01", "--out", "o"],
            "evalpin-results 1\na 1 1\n",
            "evalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "1", "--out", "o"],
            "evalpin-results 1\na 1 1\n",
            "evalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "20", "--out", "o"],
            "evalpin-results 1\na 20 20\n",
            "evalpin: 1 cases x 20 attempts, pass^20 1/1, pass@20 1/1\n",
        ),
        (
            &["run", "s", "--replies", "r", "--out", "o"],
            "evalpin-results 1\na 1 1\n",
            "evalpin: 1 cases x 1 attempts, pass^1 1/1, pass@1 1/1\n",
        ),
    ];

    for (words, expected_results, expected_summary) in attempts {
        let mut io = standard_io();
        assert_eq!(invoke(&mut io, words), 0, "arguments: {words:?}");
        assert_eq!(
            io.writes,
            vec![("o".to_owned(), (*expected_results).to_owned())],
            "arguments: {words:?}"
        );
        assert_eq!(io.outputs.len(), 1);
        assert!(io.outputs[0].ends_with(expected_summary));
        assert!(io.errors.is_empty());
        assert_eq!(io.events, vec!["write", "out"]);
    }

    let mut interleaved = standard_io();
    assert_eq!(invoke(&mut interleaved, &["run", "--replies", "r", "s"]), 0);
    assert_eq!(interleaved.outputs.len(), 1);
    assert!(interleaved.errors.is_empty());

    let mut option_value = standard_io();
    assert_eq!(
        invoke(
            &mut option_value,
            &["run", "s", "--out", "--only", "--replies", "r"]
        ),
        0
    );
    assert_eq!(
        option_value.writes,
        vec![("--only".to_owned(), "evalpin-results 1\na 1 1\n".to_owned())]
    );
    assert_eq!(option_value.events, vec!["write", "out"]);

    let invalid_attempts: &[(&[&str], &str)] = &[
        (
            &["run", "s", "--replies", "r", "-k", "00"],
            "evalpin: -k needs a whole number from 1 to 20, got 00\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "21"],
            "evalpin: -k needs a whole number from 1 to 20, got 21\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "VALUE"],
            "evalpin: -k needs a whole number from 1 to 20, got VALUE\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "001"],
            "evalpin: -k needs a whole number from 1 to 20, got 001\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "+1"],
            "evalpin: -k needs a whole number from 1 to 20, got +1\n",
        ),
        (
            &["run", "s", "--replies", "r", "-k", "１"],
            "evalpin: -k needs a whole number from 1 to 20, got １\n",
        ),
    ];
    for (words, expected) in invalid_attempts {
        assert_cli_error(words, expected);
    }
}

#[test]
fn competing_failures_follow_validation_and_sequential_read_order() {
    let mut attempts_before_reads = FakeIo::default();
    assert_eq!(
        invoke(
            &mut attempts_before_reads,
            &["run", "nosuch", "--replies", "bad", "-k", "0"]
        ),
        2
    );
    assert_eq!(
        attempts_before_reads.errors,
        vec!["evalpin: -k needs a whole number from 1 to 20, got 0\n"]
    );
    assert!(attempts_before_reads.reads.borrow().is_empty());
    assert!(attempts_before_reads.outputs.is_empty());
    assert!(attempts_before_reads.writes.is_empty());

    let mut suite_first = FakeIo {
        files: vec![
            ("s".to_owned(), b"case a\n".to_vec()),
            ("r".to_owned(), vec![0xff]),
            ("b".to_owned(), b"wrong header\n".to_vec()),
        ],
        ..FakeIo::default()
    };
    assert_eq!(
        invoke(
            &mut suite_first,
            &[
                "run",
                "s",
                "--replies",
                "r",
                "--baseline",
                "b",
                "--only",
                "absent"
            ]
        ),
        2
    );
    assert_eq!(
        suite_first.errors,
        vec!["evalpin: s:1: case a has no prompt\n"]
    );
    assert_eq!(*suite_first.reads.borrow(), vec!["s"]);
    assert!(suite_first.outputs.is_empty());
    assert!(suite_first.writes.is_empty());

    let mut replies_before_baseline = FakeIo {
        files: vec![
            (
                "s".to_owned(),
                b"case a\nprompt: p\nexpect equals: x\n".to_vec(),
            ),
            ("r".to_owned(), vec![0xff]),
            ("b".to_owned(), b"wrong header\n".to_vec()),
        ],
        ..FakeIo::default()
    };
    assert_eq!(
        invoke(
            &mut replies_before_baseline,
            &[
                "run",
                "s",
                "--replies",
                "r",
                "--baseline",
                "b",
                "--only",
                "absent"
            ]
        ),
        2
    );
    assert_eq!(
        replies_before_baseline.errors,
        vec!["evalpin: r: not valid UTF-8\n"]
    );
    assert_eq!(*replies_before_baseline.reads.borrow(), vec!["s", "r"]);
    assert!(replies_before_baseline.outputs.is_empty());
    assert!(replies_before_baseline.writes.is_empty());

    let mut baseline_before_filter = FakeIo {
        files: vec![
            (
                "s".to_owned(),
                b"case a\nprompt: p\nexpect equals: x\n".to_vec(),
            ),
            ("r".to_owned(), b"reply a\n| x\n".to_vec()),
            ("b".to_owned(), b"wrong header\n".to_vec()),
        ],
        ..FakeIo::default()
    };
    assert_eq!(
        invoke(
            &mut baseline_before_filter,
            &[
                "run",
                "s",
                "--replies",
                "r",
                "--baseline",
                "b",
                "--only",
                "absent"
            ]
        ),
        2
    );
    assert_eq!(
        baseline_before_filter.errors,
        vec!["evalpin: b:1: not an evalpin results file\n"]
    );
    assert_eq!(*baseline_before_filter.reads.borrow(), vec!["s", "r", "b"]);
    assert!(baseline_before_filter.outputs.is_empty());
    assert!(baseline_before_filter.writes.is_empty());

    let mut old_before_new = FakeIo {
        files: vec![
            ("old".to_owned(), b"wrong header\n".to_vec()),
            ("new".to_owned(), vec![0xff]),
        ],
        ..FakeIo::default()
    };
    assert_eq!(invoke(&mut old_before_new, &["diff", "old", "new"]), 2);
    assert_eq!(
        old_before_new.errors,
        vec!["evalpin: old:1: not an evalpin results file\n"]
    );
    assert_eq!(*old_before_new.reads.borrow(), vec!["old"]);
    assert!(old_before_new.outputs.is_empty());
    assert!(old_before_new.writes.is_empty());

    let mut only_ids_left_to_right = standard_io();
    assert_eq!(
        invoke(
            &mut only_ids_left_to_right,
            &["run", "s", "--replies", "r", "--only", "z", "--only", "q"]
        ),
        2
    );
    assert_eq!(
        only_ids_left_to_right.errors,
        vec!["evalpin: unknown case z\n"]
    );
    assert_eq!(*only_ids_left_to_right.reads.borrow(), vec!["s", "r"]);
    assert!(only_ids_left_to_right.outputs.is_empty());
    assert!(only_ids_left_to_right.writes.is_empty());
}

#[test]
fn all_six_literal_fixtures_match_their_utf8_bytes() {
    assert_eq!(
        include_bytes!("../fixtures/worked/suite.txt").as_slice(),
        "# demo suite\ncase greet\nprompt: Greet Ana.\nexpect contains: Ana\nexpect not-contains: error\n\ncase sum\nprompt: What is 2+2?\nexpect one-of: 4 | four\n\ncase profile\nprompt: Return the user as JSON.\nexpect json: $.user.name = \"Ana\"\nexpect json: $.user.tags[1] = \"ops\"\nexpect max-chars: 80\n\ncase brief\nprompt: Say it short.\nexpect max-chars: 10\n"
            .as_bytes()
    );
    assert_eq!(
        include_bytes!("../fixtures/worked/replies.txt").as_slice(),
        "reply greet\n| Hello Ana!\nreply greet\n| Hi there.\nreply sum\n| 4\nreply profile\n| ```json\n| {\"user\": {\"name\": \"Ana\", \"tags\": [\"dev\", \"ops\"]}}\n| ```\nfail brief rate limited\nreply brief\n| This is much too long.\n"
            .as_bytes()
    );
    assert_eq!(
        include_bytes!("../fixtures/worked/base.txt").as_slice(),
        b"evalpin-results 1\ngreet 2 2\nsum 1 2\nold 1 1\n"
    );
    assert_eq!(
        include_bytes!("../fixtures/readme/suite.txt").as_slice(),
        b"case greet\nprompt: Greet Ana.\nexpect contains: Ana\n"
    );
    assert_eq!(
        include_bytes!("../fixtures/readme/replies.txt").as_slice(),
        b"reply greet\n| Hello Ana!\nreply greet\n| Hi there.\n"
    );
    assert_eq!(
        include_bytes!("../fixtures/readme/improved-replies.txt").as_slice(),
        b"reply greet\n| Hello Ana!\nreply greet\n| Hello again, Ana.\n"
    );
}

#[test]
fn worked_example_run_and_diff_match_exact_bytes_and_write_before_output() {
    let mut run_io = FakeIo {
        files: vec![
            (
                "suite.txt".to_owned(),
                include_bytes!("../fixtures/worked/suite.txt").to_vec(),
            ),
            (
                "replies.txt".to_owned(),
                include_bytes!("../fixtures/worked/replies.txt").to_vec(),
            ),
            (
                "base.txt".to_owned(),
                include_bytes!("../fixtures/worked/base.txt").to_vec(),
            ),
        ],
        ..FakeIo::default()
    };
    assert_eq!(
        invoke(
            &mut run_io,
            &[
                "run",
                "suite.txt",
                "--replies",
                "replies.txt",
                "-k",
                "2",
                "--out",
                "new.txt",
                "--baseline",
                "base.txt"
            ]
        ),
        1
    );
    assert_eq!(
        *run_io.reads.borrow(),
        vec!["suite.txt", "replies.txt", "base.txt"]
    );
    assert_eq!(
        run_io.writes,
        vec![(
            "new.txt".to_owned(),
            "evalpin-results 1\ngreet 1 2\nsum 2 2\nprofile 2 2\nbrief 0 2\n".to_owned()
        )]
    );
    assert_eq!(
        run_io.outputs,
        vec![
            "CASE     PASSED  RATE  ALL  VS\ngreet    1/2     50%   no   worse\nsum      2/2     100%  yes  better\nprofile  2/2     100%  yes  new\nbrief    0/2     0%    no   new\nold      -       -     -    gone\n\ngreet: attempt 2: reply does not contain \"Ana\"\nbrief: attempt 1: provider: rate limited\nevalpin: 4 cases x 2 attempts, pass^2 2/4, pass@2 3/4; vs baseline: 1 better, 0 same, 1 worse, 2 new, 1 gone\n"
        ]
    );
    assert!(run_io.errors.is_empty());
    assert_eq!(run_io.events, vec!["write", "out"]);

    let mut diff_io = FakeIo {
        files: vec![
            (
                "base.txt".to_owned(),
                include_bytes!("../fixtures/worked/base.txt").to_vec(),
            ),
            (
                "new.txt".to_owned(),
                b"evalpin-results 1\ngreet 1 2\nsum 2 2\nprofile 2 2\nbrief 0 2\n".to_vec(),
            ),
        ],
        ..FakeIo::default()
    };
    assert_eq!(invoke(&mut diff_io, &["diff", "base.txt", "new.txt"]), 1);
    assert_eq!(*diff_io.reads.borrow(), vec!["base.txt", "new.txt"]);
    assert_eq!(
        diff_io.outputs,
        vec![
            "CASE     OLD  NEW  VS\ngreet    2/2  1/2  worse\nsum      1/2  2/2  better\nprofile  -    2/2  new\nbrief    -    0/2  new\nold      1/1  -    gone\nevalpin: 5 cases, 1 better, 0 same, 1 worse, 2 new, 1 gone\n"
        ]
    );
    assert!(diff_io.errors.is_empty());
    assert!(diff_io.writes.is_empty());
    assert_eq!(diff_io.events, vec!["out"]);
}

#[test]
fn prescribed_readme_process_run_has_exact_output_status_and_results() {
    use std::process::Command;

    let directory = std::env::temp_dir().join(std::process::id().to_string());
    let mut created = false;
    let execution = (|| {
        std::fs::create_dir(&directory)?;
        created = true;
        std::fs::write(
            directory.join("suite.txt"),
            b"case greet\nprompt: Greet Ana.\nexpect contains: Ana\n",
        )?;
        std::fs::write(
            directory.join("replies.txt"),
            b"reply greet\n| Hello Ana!\nreply greet\n| Hi there.\n",
        )?;
        let output = Command::new(env!("CARGO_BIN_EXE_evalpin"))
            .args([
                "run",
                "suite.txt",
                "--replies",
                "replies.txt",
                "-k",
                "2",
                "--out",
                "results.txt",
            ])
            .current_dir(&directory)
            .output()?;
        let results = std::fs::read(directory.join("results.txt"))?;
        Ok::<_, std::io::Error>((output, results))
    })();
    let cleanup = if created {
        std::fs::remove_dir_all(&directory)
    } else {
        Ok(())
    };
    assert!(cleanup.is_ok(), "temporary directory cleanup failed");

    let (output, results) = match execution {
        Ok(execution) => execution,
        Err(error) => panic!("process test setup or execution failed: {error}"),
    };
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, b"");
    assert_eq!(
        output.stdout,
        b"CASE   PASSED  RATE  ALL\ngreet  1/2     50%   no\n\ngreet: attempt 2: reply does not contain \"Ana\"\nevalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1\n"
    );
    assert!(!output.stdout.contains(&b'\r'));
    assert_eq!(results, b"evalpin-results 1\ngreet 1 2\n");
}
