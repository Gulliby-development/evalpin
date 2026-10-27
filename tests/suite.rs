use evalpin::check::Check;
use evalpin::json::Json;
use evalpin::jsonpath::{Path, Seg};
use evalpin::suite::{Case, parse};
use evalpin::text::Failure;

fn parse_suite(lines: &[&str], file: &str) -> Result<Vec<Case>, Failure> {
    let owned: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    parse(&owned, file)
}

#[test]
fn supplied_suite_error_table_is_exact() {
    let cases: &[(&[&str], &str)] = &[
        (&["case a b"], "NAME:1: case a b is not valid"),
        (&["case a"], "NAME:1: case a has no prompt"),
        (
            &["case a", "prompt: p", "expect equals: x", "case a"],
            "NAME:4: case a given twice",
        ),
        (&["expect equals: x"], "NAME:1: expect outside a case"),
        (
            &["case a", "prompt: p", "prompt: q"],
            "NAME:3: prompt given twice",
        ),
        (&["case a", "prompt:"], "NAME:2: prompt is empty"),
        (&["case a", "prompt: p"], "NAME:1: case a has no expect"),
        (
            &["case a", "prompt: p", "expect"],
            "NAME:3: expect needs KIND: VALUE",
        ),
        (
            &["case a", "prompt: p", "expect Equals: x"],
            "NAME:3: unknown expect kind Equals",
        ),
        (
            &["case a", "prompt: p", "expect equals:"],
            "NAME:3: expect equals needs a value",
        ),
        (
            &["case a", "prompt: p", "expect max-chars: 1x"],
            "NAME:3: expect max-chars needs a whole number of 0 or more, got 1x",
        ),
        (
            &["case a", "prompt: p", "expect one-of: a|"],
            "NAME:3: expect one-of has an empty choice",
        ),
        (
            &["case a", "prompt: p", "expect json: $.a"],
            "NAME:3: expect json needs PATH = VALUE",
        ),
        (
            &["case a", "prompt: p", "expect json: $.. = 1"],
            "NAME:3: json path $.. is not valid",
        ),
        (
            &["case a", "prompt: p", "expect json: $.a = [1]"],
            "NAME:3: json value [1] is not a JSON scalar",
        ),
        (&["# nothing"], "NAME: no cases found"),
    ];

    for (lines, expected) in cases {
        assert_eq!(
            parse_suite(lines, "NAME"),
            Err(Failure((*expected).to_owned()))
        );
    }
}

#[test]
fn suite_keywords_have_exact_ascii_boundaries_and_ordered_context_errors() {
    let cases: &[(&[&str], &str)] = &[
        (&["casex"], "F:1: line is not case, prompt or expect"),
        (
            &["case\u{00a0}a"],
            "F:1: line is not case, prompt or expect",
        ),
        (&["case\ra"], "F:1: line is not case, prompt or expect"),
        (&["prompt : x"], "F:1: line is not case, prompt or expect"),
        (&["expect: x"], "F:1: line is not case, prompt or expect"),
        (
            &["expectation equals: x"],
            "F:1: line is not case, prompt or expect",
        ),
        (&["expect\ra: x"], "F:1: line is not case, prompt or expect"),
        (&["expect"], "F:1: expect needs KIND: VALUE"),
        (&["expect\t"], "F:1: expect needs KIND: VALUE"),
        (&["expect\t: x"], "F:1: expect outside a case"),
        (&["expect equals:"], "F:1: expect outside a case"),
        (&["prompt: x"], "F:1: prompt outside a case"),
        (&["other"], "F:1: line is not case, prompt or expect"),
    ];

    for (lines, expected) in cases {
        assert_eq!(
            parse_suite(lines, "F"),
            Err(Failure((*expected).to_owned()))
        );
    }
}

#[test]
fn keyword_end_and_ascii_space_or_tab_are_recognized() {
    assert_eq!(
        parse_suite(
            &[
                "case\talpha",
                "expect\tequals: value",
                "prompt:prompt starts immediately",
            ],
            "suite"
        ),
        Ok(vec![Case {
            id: "alpha".to_owned(),
            line: 1,
            prompt: "prompt starts immediately".to_owned(),
            checks: vec![Check::Equals("value".to_owned())],
        }])
    );

    assert_eq!(
        parse_suite(&["case", "prompt: p", "expect equals: x"], "F"),
        Err(Failure("F:1: case (empty) is not valid".to_owned()))
    );
    assert_eq!(
        parse_suite(&["case a", "prompt: p", "expect"], "F"),
        Err(Failure("F:3: expect needs KIND: VALUE".to_owned()))
    );
}

#[test]
fn ids_enforce_empty_invalid_and_32_character_boundaries() {
    let id_32 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let id_33 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    assert_eq!(
        parse_suite(
            &[
                "case aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "prompt: p",
                "expect equals: x"
            ],
            "F"
        ),
        Ok(vec![Case {
            id: id_32.to_owned(),
            line: 1,
            prompt: "p".to_owned(),
            checks: vec![Check::Equals("x".to_owned())],
        }])
    );

    let invalid = [
        (&["case"][..], "F:1: case (empty) is not valid"),
        (&["case _a"][..], "F:1: case _a is not valid"),
        (&["case -a"][..], "F:1: case -a is not valid"),
        (&["case A"][..], "F:1: case A is not valid"),
        (&["case a.b"][..], "F:1: case a.b is not valid"),
        (&["case é"][..], "F:1: case é is not valid"),
        (
            &["case aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "F:1: case aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa is not valid",
        ),
    ];
    for (lines, expected) in invalid {
        assert_eq!(parse_suite(lines, "F"), Err(Failure(expected.to_owned())));
    }
    assert_eq!(id_33.chars().count(), 33);
}

#[test]
fn valid_cases_preserve_order_checks_prompts_and_original_provenance() {
    assert_eq!(
        parse_suite(
            &[
                "# suite",
                "case alpha",
                "expect equals: left  right: \\x41",
                "",
                "expect contains: colon: remains",
                "expect not-contains: error",
                "expect one-of: yes |  two words",
                "expect max-chars: 000000",
                "expect min-chars: 000003",
                "prompt:   Send  literal: \\n  ",
                "# second case",
                "case beta",
                "prompt: Return JSON.",
                "expect json: $.value = \"x=y\"",
            ],
            "suite.txt"
        ),
        Ok(vec![
            Case {
                id: "alpha".to_owned(),
                line: 2,
                prompt: "Send  literal: \\n".to_owned(),
                checks: vec![
                    Check::Equals("left  right: \\x41".to_owned()),
                    Check::Contains("colon: remains".to_owned()),
                    Check::NotContains("error".to_owned()),
                    Check::OneOf(vec!["yes".to_owned(), "two words".to_owned()]),
                    Check::MaxChars(0),
                    Check::MinChars(3),
                ],
            },
            Case {
                id: "beta".to_owned(),
                line: 12,
                prompt: "Return JSON.".to_owned(),
                checks: vec![Check::Json {
                    path_text: "$.value".to_owned(),
                    path: Path(vec![Seg::Key("value".to_owned())]),
                    expected: Json::Str("x=y".to_owned()),
                    expected_text: "\"x=y\"".to_owned(),
                }],
            },
        ])
    );
}

#[test]
fn completion_and_next_case_errors_follow_file_order_and_precedence() {
    let cases: &[(&[&str], &str)] = &[
        (&["case a"], "F:1: case a has no prompt"),
        (&["case a", "expect equals: x"], "F:1: case a has no prompt"),
        (
            &["case a", "prompt: p", "case b"],
            "F:1: case a has no expect",
        ),
        (&["case a", "case A"], "F:1: case a has no prompt"),
        (
            &["case a", "prompt: p", "expect equals: x", "case A"],
            "F:4: case A is not valid",
        ),
        (
            &["case a", "prompt: p", "expect equals: x", "case a"],
            "F:4: case a given twice",
        ),
        (
            &["case a", "prompt: p", "expect equals: x", "case b"],
            "F:4: case b has no prompt",
        ),
    ];

    for (lines, expected) in cases {
        assert_eq!(
            parse_suite(lines, "F"),
            Err(Failure((*expected).to_owned()))
        );
    }
}

#[test]
fn prompt_duplicate_precedes_empty_and_first_line_error_wins() {
    assert_eq!(
        parse_suite(&["case a", "prompt: p", "prompt:"], "F"),
        Err(Failure("F:3: prompt given twice".to_owned()))
    );
    assert_eq!(
        parse_suite(&["prompt: outside", "case A"], "F"),
        Err(Failure("F:1: prompt outside a case".to_owned()))
    );
    assert_eq!(
        parse_suite(&["case a", "prompt: p", "expect equals: x", "prompt:"], "F"),
        Err(Failure("F:4: prompt given twice".to_owned()))
    );
    assert_eq!(
        parse_suite(&["case a", "prompt:", "prompt: p"], "F"),
        Err(Failure("F:2: prompt is empty".to_owned()))
    );
}

#[test]
fn empty_comment_only_input_and_diagnostic_text_are_safe() {
    assert_eq!(
        parse_suite(&[], ""),
        Err(Failure("(empty): no cases found".to_owned()))
    );
    assert_eq!(
        parse_suite(&["", "  ", "# comment", "\t# comment"], "suite"),
        Err(Failure("suite: no cases found".to_owned()))
    );
    assert_eq!(
        parse_suite(&["case A\u{7f}"], "bad\tname"),
        Err(Failure("bad?name:1: case A? is not valid".to_owned()))
    );
}
