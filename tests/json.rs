use evalpin::json::{Json, parse as parse_json};
use evalpin::jsonpath::{Path, Seg, lookup, parse as parse_path};
use std::fmt::Debug;

#[test]
fn public_types_and_scalar_values_preserve_their_shapes() {
    let values = [
        Json::Null,
        Json::Bool(true),
        Json::Num("1.0".to_owned()),
        Json::Str("x".to_owned()),
        Json::Arr(vec![Json::Null]),
        Json::Obj(vec![("k".to_owned(), Json::Bool(false))]),
    ];
    for value in &values {
        let _: &dyn Debug = value;
        assert_eq!(value.clone(), value.to_owned());
    }

    let segments = [Seg::Key("a".to_owned()), Seg::Index(0)];
    let path = Path(segments.to_vec());
    let _: &dyn Debug = &segments[0];
    let _: &dyn Debug = &path;
    assert_eq!(segments[0].clone(), Seg::Key("a".to_owned()));
    assert_eq!(path.clone(), Path(segments.to_vec()));

    let scalar_cases: &[(&str, Json)] = &[
        ("null", Json::Null),
        ("true", Json::Bool(true)),
        ("false", Json::Bool(false)),
    ];
    for (input, expected) in scalar_cases {
        assert_eq!(parse_json(input), Some(expected.clone()));
    }

    let number_cases = [
        "0",
        "-0",
        "1",
        "-1",
        "1234567890123456789012345678901234567890",
        "0.0",
        "-0.125",
        "1.2300",
        "1e0",
        "-2E+17",
        "4e-09",
    ];
    for input in number_cases {
        assert_eq!(parse_json(input), Some(Json::Num(input.to_owned())));
    }
}

#[test]
fn numbers_keywords_and_full_consumption_are_strict() {
    let invalid_cases = [
        "",
        " ",
        "n",
        "nul",
        "True",
        "undefined",
        "01",
        "-01",
        "+1",
        "-",
        "--1",
        ".1",
        "1.",
        "1e",
        "1E+",
        "1e-",
        "1 2",
        "nullx",
        "truefalse",
        "NaN",
        "Infinity",
        "0x1",
        "1_0",
        "--0",
        "[",
        "{",
        "\"",
        "\"abc",
        "null]",
        "true,",
    ];
    for input in invalid_cases {
        assert_eq!(parse_json(input), None, "accepted invalid JSON {input:?}");
    }

    assert_eq!(parse_json(" \t\r\nnull\r"), Some(Json::Null));
    assert_eq!(parse_json("\u{feff}null"), None);
    assert_eq!(parse_json("\u{00a0}null"), None);
    assert_eq!(parse_json("\u{000b}null"), None);
}

#[test]
fn strings_decode_every_escape_and_validate_unicode() {
    let valid_cases: &[(&str, &str)] = &[
        ("\"\"", ""),
        (r#""aé😀""#, "aé😀"),
        (r#""\"""#, "\""),
        (r#""\\""#, "\\"),
        (r#""\/""#, "/"),
        (r#""\b""#, "\u{8}"),
        (r#""\f""#, "\u{c}"),
        (r#""\n""#, "\n"),
        (r#""\r""#, "\r"),
        (r#""\t""#, "\t"),
        (r#""\u0041\u00e9\uD83d\uDe00""#, "Aé😀"),
        (r#""\uD800\uDC00""#, "\u{10000}"),
        (r#""\uDBFF\uDFFF""#, "\u{10ffff}"),
        (r#""\u0000""#, "\0"),
    ];
    for (input, expected) in valid_cases {
        assert_eq!(parse_json(input), Some(Json::Str((*expected).to_owned())));
    }

    let invalid_cases = [
        r#""\x""#,
        r#""\u""#,
        r#""\u12""#,
        r#""\u12G4""#,
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uDC00\uD800""#,
        r#""\uD800\uDBFF""#,
        r#""\uD800x""#,
        r#""\uD800\\uDC00""#,
        "\"a\nb\"",
        "\"a\tb\"",
        "\"a\0b\"",
    ];
    for input in invalid_cases {
        assert_eq!(parse_json(input), None, "accepted invalid string {input:?}");
    }
}

#[test]
fn arrays_and_objects_require_json_separators_and_preserve_pair_order() {
    let valid_cases: &[(&str, Json)] = &[
        ("[]", Json::Arr(vec![])),
        ("{}", Json::Obj(vec![])),
        (
            "[null, true, 2]",
            Json::Arr(vec![
                Json::Null,
                Json::Bool(true),
                Json::Num("2".to_owned()),
            ]),
        ),
        (
            r#"{"a":1,"b":[false],"a":2}"#,
            Json::Obj(vec![
                ("a".to_owned(), Json::Num("1".to_owned())),
                ("b".to_owned(), Json::Arr(vec![Json::Bool(false)])),
                ("a".to_owned(), Json::Num("2".to_owned())),
            ]),
        ),
        (
            " { \"a\" : { \"b\" : [] } } ",
            Json::Obj(vec![(
                "a".to_owned(),
                Json::Obj(vec![("b".to_owned(), Json::Arr(vec![]))]),
            )]),
        ),
    ];
    for (input, expected) in valid_cases {
        assert_eq!(parse_json(input), Some(expected.clone()));
    }

    let invalid_cases = [
        "[,]",
        "[1,]",
        "[1 2]",
        "[1,,2]",
        "{,}",
        "{a:1}",
        r#"{"a" 1}"#,
        r#"{"a":}"#,
        r#"{"a":1,}"#,
        r#"{"a":1 "b":2}"#,
        "{",
        "}",
        "[1}",
        r#"{"a":1]"#,
    ];
    for input in invalid_cases {
        assert_eq!(
            parse_json(input),
            None,
            "accepted invalid container {input:?}"
        );
    }
}

#[test]
fn nesting_limit_accepts_twenty_containers_and_rejects_twenty_one() {
    let array_twenty = "[[[[[ [[[[[ [[[[[ [[[[[ 0 ]]]]] ]]]]] ]]]]] ]]]]]";
    let object_twenty = r#"{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":
0
}
}
}
}
}

}
}
}
}
}

}
}
}
}
}

}
}
}
}
}"#;
    let mixed_twenty = r#"[{"a":[{"a":[{"a":[{"a":[{"a": [{"a":[{"a":[{"a":[{"a":[{"a": 0}] }] }] }] }] }] }] }] }] }]"#;
    let array_twenty_one = "[[[[[ [[[[[ [[[[[ [[[[[ [ 0 ] ]]]]] ]]]]] ]]]]] ]]]]]";
    let object_twenty_one = r#"{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
{"a":
{"a":
{"a":
{"a":

{"a":
0
}
}
}
}
}

}
}
}
}
}

}
}
}
}
}

}
}
}
}
}"#;
    let mixed_twenty_one = r#"[{"a":[{"a":[{"a":[{"a":[{"a": [{"a":[{"a":[{"a":[{"a":[{"a": [{"a": 0}] }] }] }] }] }] }] }] }] }] }]"#;

    assert_eq!(
        parse_json(array_twenty).as_ref().and_then(|json| lookup(
            json,
            &Path(vec![
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
                Seg::Index(0),
            ])
        )),
        Some(&Json::Num("0".to_owned()))
    );
    assert_eq!(
        parse_json(object_twenty).as_ref().and_then(|json| lookup(
            json,
            &Path(vec![
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
                Seg::Key("a".to_owned()),
            ])
        )),
        Some(&Json::Num("0".to_owned()))
    );
    assert_eq!(
        parse_json(mixed_twenty).as_ref().and_then(|json| lookup(
            json,
            &Path(vec![
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
                Seg::Index(0),
                Seg::Key("a".to_owned()),
            ])
        )),
        Some(&Json::Num("0".to_owned()))
    );

    assert_eq!(parse_json(array_twenty_one), None);
    assert_eq!(parse_json(object_twenty_one), None);
    assert_eq!(parse_json(mixed_twenty_one), None);
}

#[test]
fn paths_accept_only_the_documented_ascii_keys_and_indices() {
    let valid_cases: &[(&str, Path)] = &[
        ("$", Path(vec![])),
        ("$.Az09_-", Path(vec![Seg::Key("Az09_-".to_owned())])),
        (
            "$.root[0].item-2[123456]",
            Path(vec![
                Seg::Key("root".to_owned()),
                Seg::Index(0),
                Seg::Key("item-2".to_owned()),
                Seg::Index(123456),
            ]),
        ),
        ("$[000001]", Path(vec![Seg::Index(1)])),
        (
            "$._.-",
            Path(vec![Seg::Key("_".to_owned()), Seg::Key("-".to_owned())]),
        ),
    ];
    for (input, expected) in valid_cases {
        assert_eq!(parse_path(input), Some(expected.clone()));
    }

    let invalid_cases = [
        "",
        ".",
        "a",
        "$.",
        "$..",
        "$[",
        "$[]",
        "$[-1]",
        "$[+1]",
        "$[1 ]",
        "$[ 1]",
        "$[1234567]",
        "$[١]",
        "$.é",
        "$[0]tail",
        "$[0][1",
        "$ [0]",
        "$\u{feff}",
        "$$",
    ];
    for input in invalid_cases {
        assert_eq!(parse_path(input), None, "accepted invalid path {input:?}");
    }
}

#[test]
fn lookup_is_borrowed_safe_and_selects_the_first_decoded_key() {
    let json = Json::Obj(vec![
        ("name".to_owned(), Json::Str("Ana".to_owned())),
        (
            "items".to_owned(),
            Json::Arr(vec![
                Json::Str("first".to_owned()),
                Json::Obj(vec![("value".to_owned(), Json::Bool(true))]),
                Json::Str("last".to_owned()),
            ]),
        ),
        ("dup".to_owned(), Json::Num("1".to_owned())),
        ("dup".to_owned(), Json::Num("2".to_owned())),
    ]);

    assert_eq!(lookup(&json, &Path(vec![])), Some(&json));
    assert_eq!(
        lookup(&json, &parse_path("$.name").unwrap_or(Path(vec![]))),
        Some(&Json::Str("Ana".to_owned()))
    );
    assert_eq!(
        lookup(&json, &parse_path("$.items[0]").unwrap_or(Path(vec![]))),
        Some(&Json::Str("first".to_owned()))
    );
    assert_eq!(
        lookup(&json, &parse_path("$.items[2]").unwrap_or(Path(vec![]))),
        Some(&Json::Str("last".to_owned()))
    );
    assert_eq!(
        lookup(
            &json,
            &parse_path("$.items[1].value").unwrap_or(Path(vec![]))
        ),
        Some(&Json::Bool(true))
    );
    assert_eq!(
        lookup(&json, &parse_path("$.dup").unwrap_or(Path(vec![]))),
        Some(&Json::Num("1".to_owned()))
    );

    let decoded_duplicate = parse_json(r#"{"a":1,"\u0061":2}"#);
    assert_eq!(
        decoded_duplicate
            .as_ref()
            .and_then(|value| lookup(value, &parse_path("$.a").unwrap_or(Path(vec![])))),
        Some(&Json::Num("1".to_owned()))
    );

    let array = Json::Arr(vec![Json::Null, Json::Bool(false)]);
    assert_eq!(
        lookup(&array, &Path(vec![Seg::Index(0)])),
        Some(&Json::Null)
    );
    assert_eq!(
        lookup(&array, &Path(vec![Seg::Index(1)])),
        Some(&Json::Bool(false))
    );
    assert_eq!(lookup(&array, &Path(vec![Seg::Index(2)])), None);
    assert_eq!(lookup(&array, &Path(vec![Seg::Index(usize::MAX)])), None);
    assert_eq!(lookup(&array, &Path(vec![Seg::Key("0".to_owned())])), None);
    assert_eq!(lookup(&json, &Path(vec![Seg::Index(0)])), None);
    assert_eq!(
        lookup(&Json::Bool(true), &Path(vec![Seg::Key("x".to_owned())])),
        None
    );
    assert_eq!(lookup(&Json::Bool(true), &Path(vec![Seg::Index(0)])), None);
    assert_eq!(
        lookup(&Json::Null, &Path(vec![Seg::Key("x".to_owned())])),
        None
    );
    assert_eq!(
        lookup(&json, &Path(vec![Seg::Key("missing".to_owned())])),
        None
    );
    assert_eq!(
        lookup(
            &json,
            &Path(vec![
                Seg::Key("name".to_owned()),
                Seg::Key("next".to_owned())
            ])
        ),
        None
    );
}
