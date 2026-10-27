use evalpin::check::{Check, parse};
use evalpin::json::Json;
use evalpin::jsonpath::{Path, Seg};

#[test]
fn public_checks_have_the_requested_shapes_and_preserve_values() {
    let cases = [
        ("equals", "a b", Check::Equals("a b".to_owned())),
        ("contains", "a b", Check::Contains("a b".to_owned())),
        ("not-contains", "a b", Check::NotContains("a b".to_owned())),
        (
            "one-of",
            " yes | no ",
            Check::OneOf(vec!["yes".to_owned(), "no".to_owned()]),
        ),
        ("max-chars", "000000", Check::MaxChars(0)),
        ("min-chars", "000012", Check::MinChars(12)),
        (
            "json",
            " $.a[01] = 1e0 ",
            Check::Json {
                path_text: "$.a[01]".to_owned(),
                path: Path(vec![Seg::Key("a".to_owned()), Seg::Index(1)]),
                expected: Json::Num("1e0".to_owned()),
                expected_text: "1e0".to_owned(),
            },
        ),
    ];

    for (kind, value, expected) in cases {
        let parsed = parse(kind, value);
        assert_eq!(parsed, Ok(expected.clone()));
        assert_eq!(parsed.ok(), Some(expected));
    }
}

#[test]
fn parse_errors_are_ordered_and_whitespace_is_preserved_as_specified() {
    let cases = [
        ("unknown", "", "unknown expect kind unknown"),
        ("Equals", "", "unknown expect kind Equals"),
        ("equals", "", "expect equals needs a value"),
        ("contains", "", "expect contains needs a value"),
        ("not-contains", "", "expect not-contains needs a value"),
        ("one-of", "", "expect one-of needs a value"),
        ("max-chars", "", "expect max-chars needs a value"),
        ("min-chars", "", "expect min-chars needs a value"),
        ("json", "", "expect json needs a value"),
        ("one-of", "|a", "expect one-of has an empty choice"),
        ("one-of", "a|", "expect one-of has an empty choice"),
        ("one-of", "a||b", "expect one-of has an empty choice"),
        ("one-of", "a| |b", "expect one-of has an empty choice"),
        (
            "max-chars",
            "1x",
            "expect max-chars needs a whole number of 0 or more, got 1x",
        ),
        (
            "min-chars",
            "-1",
            "expect min-chars needs a whole number of 0 or more, got -1",
        ),
        (
            "max-chars",
            "1234567",
            "expect max-chars needs a whole number of 0 or more, got 1234567",
        ),
        (
            "min-chars",
            "١",
            "expect min-chars needs a whole number of 0 or more, got ١",
        ),
        ("json", " $.a ", "expect json needs PATH = VALUE"),
        ("json", "$.a\n = []", "json path $.a? is not valid"),
        ("json", "$.. = []", "json path $.. is not valid"),
        ("json", "$ = ", "json value (empty) is not a JSON scalar"),
        (
            "json",
            "$.a = [123456789012345678901234567890]",
            "json value [123456789012345678901234567890] is not a JSON scalar",
        ),
        ("json", "$.a = []", "json value [] is not a JSON scalar"),
        ("json", "$.a = {}", "json value {} is not a JSON scalar"),
        (
            "json",
            "$.a = 1 trailing",
            "json value 1 trailing is not a JSON scalar",
        ),
        ("json", "$.a = 01", "json value 01 is not a JSON scalar"),
        (
            "json",
            "$.a = \"unterminated",
            "json value \"unterminated is not a JSON scalar",
        ),
    ];

    for (kind, value, expected) in cases {
        assert_eq!(parse(kind, value), Err((*expected).to_owned()));
    }

    assert_eq!(
        parse("bad\tkind", ""),
        Err("unknown expect kind bad?kind".to_owned())
    );
    assert_eq!(
        parse("equals", "a  b"),
        Ok(Check::Equals("a  b".to_owned()))
    );
    assert_eq!(parse("equals", " "), Ok(Check::Equals(" ".to_owned())));
    assert_eq!(
        parse("one-of", "  a  b | c "),
        Ok(Check::OneOf(vec!["a  b".to_owned(), "c".to_owned()]))
    );
}

#[test]
fn numeric_limits_accept_zero_leading_zeroes_and_six_ascii_digits_only() {
    let valid = [
        ("max-chars", "0", Check::MaxChars(0)),
        ("max-chars", "000000", Check::MaxChars(0)),
        ("max-chars", "999999", Check::MaxChars(999999)),
        ("min-chars", "000001", Check::MinChars(1)),
        ("min-chars", "123456", Check::MinChars(123456)),
    ];
    for (kind, value, expected) in valid {
        assert_eq!(parse(kind, value), Ok(expected));
    }

    let invalid = [
        ("max-chars", "+1"),
        ("max-chars", " 1"),
        ("max-chars", "1 "),
        ("max-chars", "１２"),
        ("min-chars", "1234567"),
        ("min-chars", "١"),
    ];
    for (kind, value) in invalid {
        assert!(parse(kind, value).is_err());
    }
}

#[test]
fn text_checks_trim_answers_compare_exactly_and_report_safe_clipped_text() {
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        ("equals", "4", "5", Some("reply \"5\" is not \"4\"")),
        ("equals", "4", "\n4\n", None),
        (
            "equals",
            "ab",
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            Some("reply \"xxxxxxxxxxxxxxxxxxxxx...\" is not \"ab\""),
        ),
        (
            "contains",
            "Ana",
            "Hello",
            Some("reply does not contain \"Ana\""),
        ),
        (
            "not-contains",
            "error",
            "an error",
            Some("reply contains \"error\""),
        ),
        (
            "one-of",
            "yes | no",
            "maybe",
            Some("reply \"maybe\" is none of the choices"),
        ),
        ("one-of", "yes | no", "no", None),
        (
            "contains",
            "ana",
            "Ana",
            Some("reply does not contain \"ana\""),
        ),
        (
            "equals",
            "x",
            "abcdefghijklmnopqrstuvwx",
            Some("reply \"abcdefghijklmnopqrstuvwx\" is not \"x\""),
        ),
        (
            "equals",
            "x",
            "abcdefghijklmnopqrstuvwxy",
            Some("reply \"abcdefghijklmnopqrstu...\" is not \"x\""),
        ),
        ("equals", "x", "a\nb", Some("reply \"a?b\" is not \"x\"")),
        ("equals", "a\nb", "x", Some("reply \"x\" is not \"a?b\"")),
    ];

    for (kind, value, reply, expected) in cases {
        assert_eq!(
            parse(kind, value).map(|check| check.eval(reply)),
            Ok(expected.map(str::to_owned))
        );
    }
}

#[test]
fn character_checks_are_inclusive_and_count_unicode_code_points() {
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        (
            "max-chars",
            "3",
            "abcd",
            Some("reply has 4 chars, more than 3"),
        ),
        ("max-chars", "3", "😀😀😀", None),
        (
            "max-chars",
            "2",
            "😀😀😀",
            Some("reply has 3 chars, more than 2"),
        ),
        ("max-chars", "0", " \t\r\n", None),
        (
            "min-chars",
            "2",
            " a ",
            Some("reply has 1 chars, fewer than 2"),
        ),
        ("min-chars", "2", "😀a", None),
        ("min-chars", "0", "", None),
        (
            "min-chars",
            "1",
            "",
            Some("reply has 0 chars, fewer than 1"),
        ),
    ];

    for (kind, value, reply, expected) in cases {
        assert_eq!(
            parse(kind, value).map(|check| check.eval(reply)),
            Ok(expected.map(str::to_owned))
        );
    }
}

#[test]
fn json_scalars_decode_strings_and_compare_numbers_by_exact_spelling() {
    let cases: &[(&str, &str, Option<&str>)] = &[
        ("$ = null", "null", None),
        ("$ = true", "true", None),
        ("$ = false", "false", None),
        ("$ = 0", "0", None),
        ("$ = -0", "-0", None),
        ("$ = 1.0", "1.0", None),
        ("$ = 1e0", "1e0", None),
        (r#"$ = "a=y""#, r#""a=y""#, None),
        (r#"$ = "A😀""#, r#""\u0041\uD83D\uDE00""#, None),
        (r#"$ = "\u0000""#, r#""\u0000""#, None),
        ("$ = 1", "1.0", Some("$ is 1.0, expected 1")),
        ("$ = 1.0", "1", Some("$ is 1, expected 1.0")),
    ];

    for (spec, reply, expected) in cases {
        assert_eq!(
            parse("json", spec).map(|check| check.eval(reply)),
            Ok(expected.map(str::to_owned))
        );
    }
}
#[test]
fn json_lookup_handles_root_nesting_arrays_and_first_duplicate_key() {
    let cases: &[(&str, &str, Option<&str>)] = &[
        ("$.a = 1", r#"{"a":1}"#, None),
        ("$ = 1", "1", None),
        ("$.a = \"x=y\"", r#"{"a":"x=y"}"#, None),
        ("$.a = \"A\"", r#"{"a":"\u0041"}"#, None),
        ("$.a = true", r#"{"a":true}"#, None),
        ("$.a = false", r#"{"a":false}"#, None),
        ("$.a = null", r#"{"a":null}"#, None),
        ("$.l[1] = \"b\"", r#"{"l":["a","b"]}"#, None),
        ("$.a = 1", r#"{"a":1,"a":2}"#, None),
        ("$.a.b[0] = \"x\"", r#"{"a":{"b":["x"]}}"#, None),
        ("$.missing = 1", r#"{"a":1}"#, Some("$.missing is missing")),
        ("$.a[2] = 1", r#"{"a":[1]}"#, Some("$.a[2] is missing")),
        ("$.a.b = 1", r#"{"a":1}"#, Some("$.a.b is missing")),
        ("$.a = 1", r#"{"a":"1"}"#, Some("$.a is \"1\", expected 1")),
    ];

    for (spec, reply, expected) in cases {
        assert_eq!(
            parse("json", spec).map(|check| check.eval(reply)),
            Ok(expected.map(str::to_owned))
        );
    }
}

#[test]
fn json_diagnostics_cover_invalid_answers_types_and_display_sanitization() {
    let cases: &[(&str, &str, &str)] = &[
        ("$.a = 1", "nope", "reply is not JSON"),
        ("$.a = 1", r#"{"b":1}"#, "$.a is missing"),
        ("$.a = 1", r#"{"a":"1"}"#, "$.a is \"1\", expected 1"),
        ("$.a = 1", r#"{"a":1.0}"#, "$.a is 1.0, expected 1"),
        (
            "$.a = null",
            r#"{"a":{}}"#,
            "$.a is an object, expected null",
        ),
        (
            "$.a = null",
            r#"{"a":[]}"#,
            "$.a is an array, expected null",
        ),
        (
            "$.a = true",
            r#"{"a":false}"#,
            "$.a is false, expected true",
        ),
        (
            "$.l[1] = \"b\"",
            r#"{"l":["a","c"]}"#,
            "$.l[1] is \"c\", expected \"b\"",
        ),
        ("$.a = 1", r#"{"a":"\u0000"}"#, "$.a is \"?\", expected 1"),
        ("$.a = 1", r#"{"a":1} trailing"#, "reply is not JSON"),
        ("$.a = 1", r#"{"a":1,}"#, "reply is not JSON"),
    ];
    for (spec, reply, expected) in cases {
        assert_eq!(
            parse("json", spec).map(|check| check.eval(reply)),
            Ok(Some((*expected).to_owned()))
        );
    }

    let depth_twenty = "[[[[[ [[[[[ [[[[[ [[[[[ 0 ]]]]] ]]]]] ]]]]] ]]]]]";
    let depth_twenty_one = "[[[[[ [[[[[ [[[[[ [[[[[ [ 0 ] ]]]]] ]]]]] ]]]]] ]]]]]";
    assert_eq!(
        parse("json", "$ = 0").map(|check| check.eval(depth_twenty)),
        Ok(Some("$ is an array, expected 0".to_owned()))
    );
    assert_eq!(
        parse("json", "$ = 0").map(|check| check.eval(depth_twenty_one)),
        Ok(Some("reply is not JSON".to_owned()))
    );

    assert_eq!(
        parse("json", "$.a = 123456789012345678901234567890").map(|check| check.eval(r#"{"a":0}"#)),
        Ok(Some(
            "$.a is 0, expected 123456789012345678901...".to_owned()
        ))
    );
}

#[test]
fn json_fences_require_exact_boundaries_and_keep_interior_content() {
    let cases: &[(&str, Option<&str>)] = &[
        ("```json\n{\"a\":1}\n```", None),
        ("```anything goes\n{\"a\":1}\n```", None),
        ("```\n```", Some("reply is not JSON")),
        ("```json", Some("reply is not JSON")),
        ("```\n{\"a\":1}", Some("reply is not JSON")),
        ("```\n{\"a\":1}\n```\n```", Some("reply is not JSON")),
        ("```\n{\"a\":1}\n ```", Some("reply is not JSON")),
        ("```json\n\n{\"a\":1}\n```", None),
        ("\r\n```json\r\n{\"a\":1}\r\n```\r\n", None),
        ("{\"a\":1}\r2", Some("reply is not JSON")),
        ("```json\n{\"a\":1}\n```\n", None),
        ("```json\n{\"a\":1}\n```\n\n", None),
        (
            "```json\n{\"a\":1}\n```\ntrailing",
            Some("reply is not JSON"),
        ),
    ];

    for (reply, expected) in cases {
        assert_eq!(
            parse("json", "$.a = 1").map(|check| check.eval(reply)),
            Ok(expected.map(str::to_owned))
        );
    }
}

#[test]
fn json_path_validation_precedes_scalar_validation_and_retains_literal_text() {
    let cases = [
        ("$.a = []", "json value [] is not a JSON scalar"),
        ("$.bad\npath = []", "json path $.bad?path is not valid"),
        ("$.a = [1,]", "json value [1,] is not a JSON scalar"),
        (
            "$.a = true false",
            "json value true false is not a JSON scalar",
        ),
        ("$[١] = 1", "json path $[١] is not valid"),
        ("$.é = 1", "json path $.é is not valid"),
        ("$.. = [", "json path $.. is not valid"),
    ];

    for (spec, expected) in cases {
        assert_eq!(parse("json", spec), Err(expected.to_owned()));
    }
    assert_eq!(
        parse("json", " $.a[01] = 1e0 "),
        Ok(Check::Json {
            path_text: "$.a[01]".to_owned(),
            path: Path(vec![Seg::Key("a".to_owned()), Seg::Index(1)]),
            expected: Json::Num("1e0".to_owned()),
            expected_text: "1e0".to_owned(),
        })
    );
}
