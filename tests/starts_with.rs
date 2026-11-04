use evalpin::check::{Check, parse};

#[test]
fn starts_with_keeps_its_value_exactly() {
    assert_eq!(
        parse("starts-with", " Dear Ana"),
        Ok(Check::StartsWith(" Dear Ana".to_owned()))
    );
    assert_eq!(
        parse("starts-with", "a|b"),
        Ok(Check::StartsWith("a|b".to_owned()))
    );
}

#[test]
fn starts_with_needs_a_value() {
    assert_eq!(
        parse("starts-with", ""),
        Err("expect starts-with needs a value".to_owned())
    );
}

#[test]
fn starts_with_matches_the_trimmed_reply_from_its_first_character() {
    let cases: &[(&str, &str, Option<&str>)] = &[
        ("Dear", "Dear Ana", None),
        ("Dear", "\n\t Dear Ana \r\n", None),
        ("Dear Ana", "Dear Ana", None),
        ("D", "D", None),
        ("\u{e9}t\u{e9}", "\u{e9}t\u{e9} chaud", None),
        (
            "Dear",
            "dear Ana",
            Some("reply does not start with \"Dear\""),
        ),
        ("Ana", "Dear Ana", Some("reply does not start with \"Ana\"")),
        (
            "Dear Ana!",
            "Dear Ana",
            Some("reply does not start with \"Dear Ana!\""),
        ),
        (
            "Dear Ana",
            "Dear  Ana",
            Some("reply does not start with \"Dear Ana\""),
        ),
        ("x", "", Some("reply does not start with \"x\"")),
        (
            "abcdefghijklmnopqrstuvwxyz0123",
            "abc",
            Some("reply does not start with \"abcdefghijklmnopqrstu...\""),
        ),
    ];

    for (value, reply, expected) in cases {
        let check = parse("starts-with", value).unwrap();
        assert_eq!(
            check.eval(reply),
            expected.map(str::to_owned),
            "value {value:?} reply {reply:?}"
        );
    }
}

#[test]
fn starts_with_does_not_match_inside_the_reply() {
    let check = Check::StartsWith("Ana".to_owned());
    assert_eq!(
        check.eval("Hello Ana"),
        Some("reply does not start with \"Ana\"".to_owned())
    );
}
