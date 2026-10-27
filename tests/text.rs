use evalpin::io::{Io, read_lines};
use evalpin::options::{Parsed, Spec, parse};
use evalpin::osio::OsIo;
use evalpin::text::{
    Failure, clip, disp, is_digits, is_id, lines, name, normalize, points, shown, trim_answer,
    trim_blanks,
};
use std::error::Error;

#[derive(Default)]
struct FakeIo {
    files: Vec<(String, Vec<u8>)>,
    writes: Vec<(String, String)>,
    outputs: Vec<String>,
    errors: Vec<String>,
    refuse_writes: bool,
}

impl Io for FakeIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.files
            .iter()
            .find(|(name, _)| name == path)
            .map(|(_, bytes)| bytes.clone())
    }

    fn write(&mut self, path: &str, text: &str) -> bool {
        self.writes.push((path.to_owned(), text.to_owned()));
        !self.refuse_writes
    }

    fn out(&mut self, text: &str) {
        self.outputs.push(text.to_owned());
    }

    fn err(&mut self, text: &str) {
        self.errors.push(text.to_owned());
    }
}

#[test]
fn public_api_and_fake_io_contracts() {
    let failure = Failure("bad".to_owned());
    assert_eq!(failure.to_string(), "bad");
    let _: &dyn Error = &failure;
    assert_eq!(failure.clone(), failure);

    let specs = [
        Spec {
            name: "--only",
            repeat: true,
        },
        Spec {
            name: "--out",
            repeat: false,
        },
        Spec {
            name: "-k",
            repeat: false,
        },
    ];
    assert_eq!(specs.clone(), specs);

    let parsed = Parsed {
        operands: vec!["suite".to_owned()],
        values: vec![("--out".to_owned(), "result.txt".to_owned())],
    };
    assert_eq!(parsed.clone(), parsed);

    let os_io = OsIo;
    assert_eq!(os_io.clone(), os_io);

    let mut fake = FakeIo {
        refuse_writes: true,
        ..FakeIo::default()
    };
    assert!(!fake.write("out.txt", "exact\ntext"));
    fake.out("stdout");
    fake.err("stderr");
    assert_eq!(
        fake.writes,
        vec![("out.txt".to_owned(), "exact\ntext".to_owned())]
    );
    assert_eq!(fake.outputs, vec!["stdout"]);
    assert_eq!(fake.errors, vec!["stderr"]);

    let trait_io: &dyn Io = &fake;
    assert_eq!(
        read_lines(trait_io, "missing"),
        Err(Failure("missing: cannot be read".to_owned()))
    );
}

#[test]
fn shown_names_clipping_and_code_point_counts() {
    assert_eq!(shown("a\0\t\n\r\u{1f}\u{7f}é😀"), "a??????é😀");
    assert_eq!(shown("\u{80}\u{a0}"), "\u{80}\u{a0}");
    assert_eq!(name(""), "(empty)");
    assert_eq!(name("x\t"), "x?");
    assert_eq!(points("é😀"), 2);
    assert_eq!(points(""), 0);
    assert_eq!(clip("abcdefghijklmnopqrstuvwx"), "abcdefghijklmnopqrstuvwx");
    assert_eq!(
        clip("abcdefghijklmnopqrstuvwxy"),
        "abcdefghijklmnopqrstu..."
    );
    assert_eq!(
        clip("😀bcdefghijklmnopqrstuvwxy"),
        "😀bcdefghijklmnopqrstu..."
    );
    assert_eq!(
        disp("\tabcdefghijklmnopqrstuvwx"),
        "?abcdefghijklmnopqrst..."
    );
}

#[test]
fn trimming_and_ascii_identifier_validation() {
    assert_eq!(trim_blanks(" \t a \t "), "a");
    assert_eq!(trim_blanks(" \t\r\na\r\n\t "), "\r\na\r\n");
    assert_eq!(trim_answer(" \t\r\na\r\n\t "), "a");
    assert_eq!(trim_answer(" \t\r\n"), "");
    assert_eq!(trim_blanks("\u{a0}a\u{a0}"), "\u{a0}a\u{a0}");
    assert_eq!(trim_answer("\u{b}a\u{c}"), "\u{b}a\u{c}");

    assert!(is_id("a0_-"));
    assert!(is_id("0"));
    assert!(is_id("abcdefghijklmnopqrstuvwxyz012345"));
    assert!(!is_id(""));
    assert!(!is_id("_a"));
    assert!(!is_id("-a"));
    assert!(!is_id("A"));
    assert!(!is_id("é"));
    assert!(!is_id("a b"));
    assert!(!is_id("abcdefghijklmnopqrstuvwxyz0123456"));

    assert!(is_digits("000012", 6));
    assert!(is_digits("0", 1));
    assert!(!is_digits("", 6));
    assert!(!is_digits("1", 0));
    assert!(!is_digits("1234567", 6));
    assert!(!is_digits("١", 6));
    assert!(!is_digits("+1", 6));
    assert!(!is_digits("1 ", 6));
}

#[test]
fn strict_normalization_and_line_splitting() {
    let normalization_cases: &[(&[u8], Option<&str>)] = &[
        (b"", Some("")),
        ("\u{feff}a\r\nb\rc\n".as_bytes(), Some("a\nb\rc\n")),
        (
            "\u{feff}\u{feff}x\u{feff}".as_bytes(),
            Some("\u{feff}x\u{feff}"),
        ),
        ("é😀".as_bytes(), Some("é😀")),
        (b"a\0b", None),
        (&[0xff], None),
        (&[0xc0, 0xaf], None),
        (&[0xed, 0xa0, 0x80], None),
        (&[0xf4, 0x90, 0x80, 0x80], None),
        (&[0xe2, 0x82], None),
    ];
    for (input, expected) in normalization_cases {
        assert_eq!(normalize(input).as_deref(), *expected);
    }

    let line_cases: &[(&str, &[&str])] = &[
        ("", &[]),
        ("\n", &[""]),
        ("a\n\nb\n", &["a", "", "b"]),
        ("a\n\n", &["a", ""]),
        ("a\rb", &["a\rb"]),
        ("a\nb", &["a", "b"]),
    ];
    for (input, expected) in line_cases {
        assert_eq!(lines(input), expected.to_vec());
    }
}

#[test]
fn read_lines_normalizes_owned_contents_and_has_no_output_side_effects() {
    let io = FakeIo {
        files: vec![
            ("s".to_owned(), "\u{feff}a\r\n\r\nb\r".as_bytes().to_vec()),
            ("empty".to_owned(), b"".to_vec()),
            ("bad".to_owned(), vec![0xff]),
            ("nul".to_owned(), b"a\0b".to_vec()),
        ],
        ..FakeIo::default()
    };
    let trait_io: &dyn Io = &io;

    assert_eq!(
        read_lines(trait_io, "s"),
        Ok(vec!["a".to_owned(), "".to_owned(), "b\r".to_owned()])
    );
    assert_eq!(read_lines(trait_io, "empty"), Ok(Vec::new()));
    assert_eq!(
        read_lines(trait_io, "missing\t"),
        Err(Failure("missing?: cannot be read".to_owned()))
    );
    assert_eq!(
        read_lines(trait_io, ""),
        Err(Failure("(empty): cannot be read".to_owned()))
    );
    assert_eq!(
        read_lines(trait_io, "bad"),
        Err(Failure("bad: not valid UTF-8".to_owned()))
    );
    assert_eq!(
        read_lines(trait_io, "nul"),
        Err(Failure("nul: not valid UTF-8".to_owned()))
    );
    assert!(io.writes.is_empty());
    assert!(io.outputs.is_empty());
    assert!(io.errors.is_empty());
}

#[test]
fn options_preserve_order_values_and_operands() {
    let specs = [
        Spec {
            name: "--only",
            repeat: true,
        },
        Spec {
            name: "--out",
            repeat: false,
        },
        Spec {
            name: "-k",
            repeat: false,
        },
    ];
    let cases: &[(&[&str], usize, Parsed)] = &[
        (
            &["s", "--only", "a", "-k", "02", "--only", "b"],
            1,
            Parsed {
                operands: vec!["s".to_owned()],
                values: vec![
                    ("--only".to_owned(), "a".to_owned()),
                    ("-k".to_owned(), "02".to_owned()),
                    ("--only".to_owned(), "b".to_owned()),
                ],
            },
        ),
        (
            &["--out", "--unknown", "s"],
            1,
            Parsed {
                operands: vec!["s".to_owned()],
                values: vec![("--out".to_owned(), "--unknown".to_owned())],
            },
        ),
        (
            &["--out", ""],
            1,
            Parsed {
                operands: Vec::new(),
                values: vec![("--out".to_owned(), "".to_owned())],
            },
        ),
        (
            &[],
            1,
            Parsed {
                operands: Vec::new(),
                values: Vec::new(),
            },
        ),
        (
            &["-", "-x", "-k5", ""],
            4,
            Parsed {
                operands: vec![
                    "-".to_owned(),
                    "-x".to_owned(),
                    "-k5".to_owned(),
                    "".to_owned(),
                ],
                values: Vec::new(),
            },
        ),
    ];

    for (words, allowance, expected) in cases {
        let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        assert_eq!(parse(&args, &specs, *allowance), Ok(expected.clone()));
    }

    let args = ["s", "--only", "a", "-k", "02", "--only", "b"].map(str::to_owned);
    let parsed = parse(&args, &specs, 1);
    assert_eq!(parsed, Ok(cases[0].2.clone()));
    if let Ok(parsed) = parsed {
        assert_eq!(parsed.value("--only"), Some("b"));
        assert_eq!(parsed.all("--only"), vec!["a", "b"]);
        assert_eq!(parsed.value("--out"), None);
        assert_eq!(parsed.all("--out"), Vec::<&str>::new());
    }
}

#[test]
fn options_report_first_validation_error() {
    let specs = [
        Spec {
            name: "--only",
            repeat: true,
        },
        Spec {
            name: "--out",
            repeat: false,
        },
        Spec {
            name: "-k",
            repeat: false,
        },
    ];
    let cases: &[(&[&str], usize, &str)] = &[
        (&["--out=x"], 1, "unknown option --out=x"),
        (&["--"], 1, "unknown option --"),
        (&["--bad\t"], 1, "unknown option --bad?"),
        (&["--out"], 1, "--out needs a value"),
        (&["--out", "a", "--out"], 1, "--out needs a value"),
        (&["--out", "a", "--out", "--bad"], 1, "--out given twice"),
        (&["-k", "2", "-k", "3"], 1, "-k given twice"),
        (&["s", "-k5"], 1, "unexpected argument -k5"),
        (&["s", ""], 1, "unexpected argument (empty)"),
        (&["s", "x\t", "--bad"], 1, "unexpected argument x?"),
    ];

    for (words, allowance, expected) in cases {
        let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        assert_eq!(
            parse(&args, &specs, *allowance),
            Err(Failure((*expected).to_owned()))
        );
    }

    let args = ["-k", "2"].map(str::to_owned);
    assert_eq!(
        parse(&args, &[], 2),
        Err(Failure("unknown option -k".to_owned()))
    );
}
