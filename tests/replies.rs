use evalpin::provider::{Provider, Scripted};
use evalpin::replies::{Entry, Replies, parse};
use evalpin::text::Failure;
use std::collections::BTreeMap;

#[test]
fn public_contracts_derive_and_scripted_retains_replies() {
    let text = Entry::Text("reply".to_owned());
    let fail = Entry::Fail("failed".to_owned());
    assert_eq!(text.clone(), text);
    assert_eq!(fail.clone(), fail);
    assert!(format!("{text:?}").contains("Text"));
    assert!(format!("{fail:?}").contains("Fail"));

    let supplied = Replies(BTreeMap::from([(
        "a".to_owned(),
        vec![Entry::Text("exact".to_owned())],
    )]));
    assert_eq!(supplied.clone(), supplied);
    assert!(format!("{supplied:?}").contains("Replies"));

    let mut scripted = Scripted::new(supplied.clone());
    assert_eq!(scripted.clone(), scripted);
    assert!(format!("{scripted:?}").contains("Scripted"));
    let provider: &mut dyn Provider = &mut scripted;
    assert_eq!(
        provider.reply("a", "ignored prompt", 1),
        Ok("exact".to_owned())
    );
}

#[test]
fn parses_empty_comments_text_and_ordered_entries() {
    let cases: &[(&[&str], Replies)] = &[
        (&[], Replies(BTreeMap::new())),
        (&[" # comment", "\t"], Replies(BTreeMap::new())),
        (
            &["reply a"],
            Replies(BTreeMap::from([(
                "a".to_owned(),
                vec![Entry::Text("".to_owned())],
            )])),
        ),
        (
            &["reply a", "| x", "|  y ", "|\tz", "|", "|😀"],
            Replies(BTreeMap::from([(
                "a".to_owned(),
                vec![Entry::Text("x\n y \n\tz\n\n😀".to_owned())],
            )])),
        ),
        (
            &["reply a0_-"],
            Replies(BTreeMap::from([(
                "a0_-".to_owned(),
                vec![Entry::Text("".to_owned())],
            )])),
        ),
        (
            &["reply 0"],
            Replies(BTreeMap::from([(
                "0".to_owned(),
                vec![Entry::Text("".to_owned())],
            )])),
        ),
        // Input ["reply aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] is a valid 32-character ID.
        (
            &["reply aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            Replies(BTreeMap::from([(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                vec![Entry::Text("".to_owned())],
            )])),
        ),
        (
            &[
                "\treply\ta\t",
                "| x",
                "# close",
                "fail\ta\t boom \t",
                "reply b",
                "reply a",
                "| y",
            ],
            Replies(BTreeMap::from([
                (
                    "a".to_owned(),
                    vec![
                        Entry::Text("x".to_owned()),
                        Entry::Fail("boom".to_owned()),
                        Entry::Text("y".to_owned()),
                    ],
                ),
                ("b".to_owned(), vec![Entry::Text("".to_owned())]),
            ])),
        ),
        (
            &["reply unused", "| x", ""],
            Replies(BTreeMap::from([(
                "unused".to_owned(),
                vec![Entry::Text("x".to_owned())],
            )])),
        ),
        (
            &["reply a", "|", "| x", "|"],
            Replies(BTreeMap::from([(
                "a".to_owned(),
                vec![Entry::Text("\nx\n".to_owned())],
            )])),
        ),
    ];

    for (input, expected) in cases {
        let lines = input
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(parse(&lines, "r"), Ok(expected.clone()));
    }

    let multiline = ["reply a", "| x", "|  y ", "|\tz", "|", "|😀"].map(str::to_owned);
    let parsed = parse(&multiline, "r");
    assert_eq!(
        parsed
            .as_ref()
            .ok()
            .and_then(|replies| replies.0.get("a"))
            .map(|entries| format!("a: {entries:?}"))
            .as_deref(),
        Some(r#"a: [Text("x\n y \n\tz\n\n😀")]"#)
    );

    let allowed_id = ["reply a0_-"].map(str::to_owned);
    let parsed = parse(&allowed_id, "r");
    assert_eq!(
        parsed
            .as_ref()
            .ok()
            .and_then(|replies| replies.0.get("a0_-"))
            .map(|entries| format!("a0_-: {entries:?}"))
            .as_deref(),
        Some(r#"a0_-: [Text("")]"#)
    );
}

#[test]
fn parser_returns_exact_first_errors_and_line_numbers() {
    let cases: &[(&[&str], &str, &str)] = &[
        (&["| x"], "NAME", "NAME:1: text line outside a reply"),
        (
            &["reply a", "| x", "", "| y"],
            "NAME",
            "NAME:4: text line outside a reply",
        ),
        (
            &["reply a", "# close", "| y"],
            "NAME",
            "NAME:3: text line outside a reply",
        ),
        (
            &["reply a", "fail a boom", "| y"],
            "NAME",
            "NAME:3: text line outside a reply",
        ),
        (
            &["reply a", " | x"],
            "NAME",
            "NAME:2: line is not reply, fail or text",
        ),
        (&["reply a b"], "NAME", "NAME:1: case a b is not valid"),
        (&["fail a"], "NAME", "NAME:1: fail needs a message"),
        (&["text"], "NAME", "NAME:1: line is not reply, fail or text"),
        (
            &["replies a"],
            "NAME",
            "NAME:1: line is not reply, fail or text",
        ),
        (
            &["failures a boom"],
            "NAME",
            "NAME:1: line is not reply, fail or text",
        ),
        (&["fail A boom"], "NAME", "NAME:1: case A is not valid"),
        (&["fail A", "text"], "NAME", "NAME:1: case A is not valid"),
        (&["reply"], "NAME", "NAME:1: case (empty) is not valid"),
        (&["fail"], "NAME", "NAME:1: case (empty) is not valid"),
        (
            &["reply\u{a0}a"],
            "NAME",
            "NAME:1: line is not reply, fail or text",
        ),
        (
            &["reply aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "r",
            "r:1: case aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa is not valid",
        ),
        (&["reply _a"], "r", "r:1: case _a is not valid"),
        (&["reply -a"], "r", "r:1: case -a is not valid"),
        (&["reply é"], "r", "r:1: case é is not valid"),
        (&["reply a\rb"], "r\t", "r?:1: case a?b is not valid"),
        (&["text"], "", "(empty):1: line is not reply, fail or text"),
    ];

    for (input, file, expected) in cases {
        let lines = input
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(parse(&lines, file), Err(Failure((*expected).to_owned())));
    }
}

#[test]
fn scripted_cycles_independently_and_handles_attempt_zero() {
    let replies = Replies(BTreeMap::from([
        (
            "a".to_owned(),
            vec![
                Entry::Text(" first\n".to_owned()),
                Entry::Fail("boom".to_owned()),
                Entry::Text("third".to_owned()),
            ],
        ),
        ("b".to_owned(), vec![Entry::Text("other".to_owned())]),
    ]));
    let a_entries = replies.0.get("a").map(|entries| format!("{entries:?}"));
    let b_entries = replies.0.get("b").map(|entries| format!("{entries:?}"));
    assert_eq!(
        format!(
            "a: {}, b: {}",
            a_entries.as_deref().unwrap_or("missing"),
            b_entries.as_deref().unwrap_or("missing")
        ),
        r#"a: [Text(" first\n"), Fail("boom"), Text("third")], b: [Text("other")]"#
    );

    let mut scripted = Scripted::new(replies);
    let provider: &mut dyn Provider = &mut scripted;
    let calls: &[(&str, &str, usize, Result<String, String>)] = &[
        ("a", "p", 1, Ok(" first\n".to_owned())),
        ("a", "q", 3, Ok("third".to_owned())),
        ("b", "", 2, Ok("other".to_owned())),
        ("a", "", 2, Err("boom".to_owned())),
        ("a", "changed", 4, Ok(" first\n".to_owned())),
        ("a", "p", 2, Err("boom".to_owned())),
        ("a", "p", 0, Ok(" first\n".to_owned())),
    ];

    for (case_id, prompt, attempt, expected) in calls {
        assert_eq!(provider.reply(case_id, prompt, *attempt), expected.clone());
    }
}

#[test]
fn scripted_displays_failures_and_reports_missing_entries() {
    let mut empty_text = Scripted::new(Replies(BTreeMap::from([(
        "a".to_owned(),
        vec![Entry::Text("".to_owned())],
    )])));
    assert_eq!(empty_text.reply("a", "", 1), Ok("".to_owned()));

    let fail_entry = Entry::Fail("bad\tnews\r\n".to_owned());
    assert_eq!(
        format!("{{a: [{fail_entry:?}]}}, ({:?}, {:?}, {})", "a", "", 1),
        r#"{a: [Fail("bad\tnews\r\n")]}, ("a", "", 1)"#
    );
    let mut controls = Scripted::new(Replies(BTreeMap::from([(
        "a".to_owned(),
        vec![fail_entry],
    )])));
    assert_eq!(controls.reply("a", "", 1), Err("bad?news??".to_owned()));

    let mut clipped = Scripted::new(Replies(BTreeMap::from([(
        "a".to_owned(),
        vec![Entry::Fail("abcdefghijklmnopqrstuvwxy".to_owned())],
    )])));
    assert_eq!(
        clipped.reply("a", "", 1),
        Err("abcdefghijklmnopqrstu...".to_owned())
    );

    let mut absent = Scripted::new(Replies(BTreeMap::new()));
    assert_eq!(
        absent.reply("missing", "", 1),
        Err("no reply for case missing".to_owned())
    );
    assert_eq!(
        absent.reply("", "", 0),
        Err("no reply for case (empty)".to_owned())
    );
    assert_eq!(
        absent.reply("a\t", "", 1),
        Err("no reply for case a?".to_owned())
    );

    let mut no_entries = Scripted::new(Replies(BTreeMap::from([("a".to_owned(), Vec::new())])));
    assert_eq!(
        no_entries.reply("a", "", 1),
        Err("no reply for case a".to_owned())
    );
    assert_eq!(
        no_entries.reply("a", "", 0),
        Err("no reply for case a".to_owned())
    );
}
