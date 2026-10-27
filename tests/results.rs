use evalpin::compare::{self, Vs};
use evalpin::results::{self, Row};
use evalpin::text::Failure;

#[test]
fn serialization_is_canonical_and_rows_round_trip() {
    assert_eq!(results::write(&[]), "evalpin-results 1\n");

    let rows = vec![
        Row {
            id: "greet".to_owned(),
            passed: 1,
            total: 2,
        },
        Row {
            id: "brief".to_owned(),
            passed: 0,
            total: 2,
        },
    ];
    let serialized = results::write(&rows);
    assert_eq!(serialized, "evalpin-results 1\ngreet 1 2\nbrief 0 2\n");

    let normalized: Vec<String> = evalpin::text::lines(&serialized)
        .iter()
        .map(|line| (*line).to_owned())
        .collect();
    assert_eq!(results::read(&normalized, "r"), Ok(rows.clone()));
    assert_eq!(rows, rows.clone());
}

#[test]
fn valid_result_files_preserve_order_and_accept_boundary_values() {
    assert_eq!(
        results::read(&[String::from("evalpin-results 1")], "r"),
        Ok(vec![])
    );
    assert_eq!(
        results::read(
            &[
                String::from("evalpin-results 1"),
                String::from("z 000001 000002"),
                String::from("a 0 999999"),
                String::from("a0_- 999999 999999"),
                String::from("abcdefghijklmnopqrstuvwxyz012345 1 2"),
            ],
            "r"
        ),
        Ok(vec![
            Row {
                id: "z".to_owned(),
                passed: 1,
                total: 2,
            },
            Row {
                id: "a".to_owned(),
                passed: 0,
                total: 999999,
            },
            Row {
                id: "a0_-".to_owned(),
                passed: 999999,
                total: 999999,
            },
            Row {
                id: "abcdefghijklmnopqrstuvwxyz012345".to_owned(),
                passed: 1,
                total: 2,
            },
        ])
    );
}

#[test]
fn result_header_and_row_structure_are_strict() {
    let cases: &[(&[&str], &str)] = &[
        (&[], "r:1: not an evalpin results file"),
        (&["evalpin-results 2"], "r:1: not an evalpin results file"),
        (&["evalpin-results 1 "], "r:1: not an evalpin results file"),
        (&[" evalpin-results 1"], "r:1: not an evalpin results file"),
        (
            &["evalpin-results 1", "a 1"],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", "a  1 2"],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", "a 1 2 "],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", " a 1 2"],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", ""],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", "a\t1\t2"],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", "# comment"],
            "r:2: line is not ID PASSED TOTAL",
        ),
        (
            &["evalpin-results 1", "A  x 0"],
            "r:2: line is not ID PASSED TOTAL",
        ),
    ];

    for (input, expected) in cases {
        let lines: Vec<String> = input.iter().map(|line| (*line).to_owned()).collect();
        assert_eq!(
            results::read(&lines, "r"),
            Err(Failure((*expected).to_owned()))
        );
    }
}

#[test]
fn ids_counts_and_validation_order_are_strict() {
    let cases: &[(&[&str], &str)] = &[
        (&["evalpin-results 1", "A x 0"], "r:2: case A is not valid"),
        (
            &["evalpin-results 1", "_a 1 2"],
            "r:2: case _a is not valid",
        ),
        (
            &["evalpin-results 1", "-a 1 2"],
            "r:2: case -a is not valid",
        ),
        (&["evalpin-results 1", "é 1 2"], "r:2: case é is not valid"),
        (
            &["evalpin-results 1", "abcdefghijklmnopqrstuvwxyz0123456 1 2"],
            "r:2: case abcdefghijklmnopqrstuvwxyz0123456 is not valid",
        ),
        (
            &["evalpin-results 1", "a 3 2"],
            "r:2: counts 3 of 2 are not valid",
        ),
        (
            &["evalpin-results 1", "a 0 0"],
            "r:2: counts 0 of 0 are not valid",
        ),
        (
            &["evalpin-results 1", "a 1000000 999999"],
            "r:2: counts 1000000 of 999999 are not valid",
        ),
        (
            &["evalpin-results 1", "a 1 1000000"],
            "r:2: counts 1 of 1000000 are not valid",
        ),
        (
            &["evalpin-results 1", "a -1 2"],
            "r:2: counts -1 of 2 are not valid",
        ),
        (
            &["evalpin-results 1", "a +1 2"],
            "r:2: counts +1 of 2 are not valid",
        ),
        (
            &["evalpin-results 1", "a ١ 2"],
            "r:2: counts ١ of 2 are not valid",
        ),
        (
            &["evalpin-results 1", "a 1 x"],
            "r:2: counts 1 of x are not valid",
        ),
        (
            &["evalpin-results 1", "a 1 2", "a 2 2"],
            "r:3: case a given twice",
        ),
        (
            &["evalpin-results 1", "a 1 2", "a 3 2"],
            "r:3: counts 3 of 2 are not valid",
        ),
        (
            &["evalpin-results 1", "a 3 2", "A 1 2"],
            "r:2: counts 3 of 2 are not valid",
        ),
    ];

    for (input, expected) in cases {
        let lines: Vec<String> = input.iter().map(|line| (*line).to_owned()).collect();
        assert_eq!(
            results::read(&lines, "r"),
            Err(Failure((*expected).to_owned()))
        );
    }
}

#[test]
fn diagnostics_sanitize_file_ids_and_count_values() {
    assert_eq!(
        results::read(&[], ""),
        Err(Failure("(empty):1: not an evalpin results file".to_owned()))
    );
    assert_eq!(
        results::read(
            &[String::from("evalpin-results 1"), String::from("A\t 1 2"),],
            "r\t"
        ),
        Err(Failure("r?:2: case A? is not valid".to_owned()))
    );
    assert_eq!(
        results::read(
            &[String::from("evalpin-results 1"), String::from("a x\t 2"),],
            "r"
        ),
        Err(Failure("r:2: counts x? of 2 are not valid".to_owned()))
    );
    assert_eq!(
        results::read(
            &[
                String::from("evalpin-results 1"),
                String::from("a 1 x\u{7f}"),
            ],
            "r"
        ),
        Err(Failure("r:2: counts 1 of x? are not valid".to_owned()))
    );
}

#[test]
fn comparisons_apply_presence_precedence_and_exact_cross_products() {
    let cases = [
        (None, None, Vs::New),
        (Some((1, 2)), None, Vs::New),
        (None, Some((1, 2)), Vs::Gone),
        (Some((2, 3)), Some((1, 2)), Vs::Better),
        (Some((1, 2)), Some((2, 3)), Vs::Worse),
        (Some((1, 2)), Some((2, 4)), Vs::Same),
        (Some((999999, 999999)), Some((999998, 999999)), Vs::Better),
        (
            Some((usize::MAX, usize::MAX)),
            Some((usize::MAX, usize::MAX)),
            Vs::Same,
        ),
        (
            Some((usize::MAX, usize::MAX)),
            Some((usize::MAX - 1, usize::MAX)),
            Vs::Better,
        ),
        (
            Some((usize::MAX - 1, usize::MAX)),
            Some((usize::MAX, usize::MAX)),
            Vs::Worse,
        ),
        (Some((1, 0)), Some((1, 1)), Vs::Better),
        (Some((0, 0)), Some((1, 1)), Vs::Same),
    ];

    for (now, before, expected) in cases {
        assert_eq!(compare::vs(now, before), expected.clone());
        assert_eq!(expected, expected.clone());
    }

    let words = [
        (Vs::Better, "better"),
        (Vs::Same, "same"),
        (Vs::Worse, "worse"),
        (Vs::New, "new"),
        (Vs::Gone, "gone"),
    ];
    for (value, word) in words {
        assert_eq!(value.clone().word(), word);
    }
}
