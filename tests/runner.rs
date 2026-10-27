use evalpin::check::Check;
use evalpin::provider::{Provider, Scripted};
use evalpin::replies::{Entry, Replies};
use evalpin::runner::{Attempt, CaseRun, run};
use evalpin::score::Score;
use evalpin::suite::Case;
use std::collections::BTreeMap;

fn case(id: &str, prompt: &str, checks: Vec<Check>) -> Case {
    Case {
        id: id.to_owned(),
        line: 1,
        prompt: prompt.to_owned(),
        checks,
    }
}

#[test]
fn runs_cases_and_cyclic_attempts_in_input_order() {
    let cases = [
        case("b", "prompt for b", vec![Check::Equals("x".to_owned())]),
        case("a", "prompt for a", vec![Check::Equals("x".to_owned())]),
    ];
    let replies = Replies(BTreeMap::from([
        (
            "b".to_owned(),
            vec![Entry::Text("x".to_owned()), Entry::Fail("boom".to_owned())],
        ),
        (
            "a".to_owned(),
            vec![Entry::Text("y".to_owned()), Entry::Text("x".to_owned())],
        ),
    ]));
    let mut provider = Scripted::new(replies);

    assert_eq!(
        run(&cases, &mut provider, 3),
        vec![
            CaseRun {
                id: "b".to_owned(),
                attempts: vec![
                    Attempt {
                        passed: true,
                        note: "".to_owned(),
                    },
                    Attempt {
                        passed: false,
                        note: "provider: boom".to_owned(),
                    },
                    Attempt {
                        passed: true,
                        note: "".to_owned(),
                    },
                ],
            },
            CaseRun {
                id: "a".to_owned(),
                attempts: vec![
                    Attempt {
                        passed: false,
                        note: "reply \"y\" is not \"x\"".to_owned(),
                    },
                    Attempt {
                        passed: true,
                        note: "".to_owned(),
                    },
                    Attempt {
                        passed: false,
                        note: "reply \"y\" is not \"x\"".to_owned(),
                    },
                ],
            },
        ]
    );
}

#[test]
fn provider_errors_first_failed_checks_and_empty_checks_have_exact_notes() {
    let cases = [
        case(
            "first",
            "p",
            vec![
                Check::Contains("Ana".to_owned()),
                Check::NotContains("error".to_owned()),
            ],
        ),
        case(
            "later",
            "p",
            vec![
                Check::Contains("Ana".to_owned()),
                Check::NotContains("error".to_owned()),
            ],
        ),
        case(
            "passing",
            "p",
            vec![
                Check::Contains("Ana".to_owned()),
                Check::NotContains("error".to_owned()),
            ],
        ),
        case("empty", "p", vec![]),
        case("missing", "p", vec![]),
        case("failed", "p", vec![Check::Equals("x".to_owned())]),
    ];
    let replies = Replies(BTreeMap::from([
        ("first".to_owned(), vec![Entry::Text("error".to_owned())]),
        (
            "later".to_owned(),
            vec![Entry::Text("Ana error".to_owned())],
        ),
        (
            "passing".to_owned(),
            vec![Entry::Text("Hello Ana!".to_owned())],
        ),
        ("empty".to_owned(), vec![Entry::Text("".to_owned())]),
        (
            "failed".to_owned(),
            vec![Entry::Fail("rate limited".to_owned())],
        ),
    ]));
    let mut provider = Scripted::new(replies);

    assert_eq!(
        run(&cases, &mut provider, 1),
        vec![
            CaseRun {
                id: "first".to_owned(),
                attempts: vec![Attempt {
                    passed: false,
                    note: "reply does not contain \"Ana\"".to_owned(),
                }],
            },
            CaseRun {
                id: "later".to_owned(),
                attempts: vec![Attempt {
                    passed: false,
                    note: "reply contains \"error\"".to_owned(),
                }],
            },
            CaseRun {
                id: "passing".to_owned(),
                attempts: vec![Attempt {
                    passed: true,
                    note: "".to_owned(),
                }],
            },
            CaseRun {
                id: "empty".to_owned(),
                attempts: vec![Attempt {
                    passed: true,
                    note: "".to_owned(),
                }],
            },
            CaseRun {
                id: "missing".to_owned(),
                attempts: vec![Attempt {
                    passed: false,
                    note: "provider: no reply for case missing".to_owned(),
                }],
            },
            CaseRun {
                id: "failed".to_owned(),
                attempts: vec![Attempt {
                    passed: false,
                    note: "provider: rate limited".to_owned(),
                }],
            },
        ]
    );
}

#[test]
fn empty_and_zero_attempt_runs_preserve_their_required_shapes() {
    let no_cases: [Case; 0] = [];
    let mut empty_provider = Scripted::new(Replies(BTreeMap::new()));
    assert_eq!(
        run(&no_cases, &mut empty_provider, 3),
        Vec::<CaseRun>::new()
    );

    let cases = [case("a", "prompt a", vec![]), case("b", "prompt b", vec![])];
    let mut empty_provider = Scripted::new(Replies(BTreeMap::new()));
    assert_eq!(
        run(&cases, &mut empty_provider, 0),
        vec![
            CaseRun {
                id: "a".to_owned(),
                attempts: vec![],
            },
            CaseRun {
                id: "b".to_owned(),
                attempts: vec![],
            },
        ]
    );

    let empty_run = CaseRun {
        id: "a".to_owned(),
        attempts: vec![],
    };
    let empty_score = Score::of(&empty_run);
    assert_eq!(
        empty_score,
        Score {
            passed: 0,
            total: 0
        }
    );
    assert_eq!(empty_score.cell(), "0/0");
    assert!(empty_score.all());
    assert!(!empty_score.some());
    assert_eq!(empty_score.rate(), 0);

    let mixed_run = CaseRun {
        id: "mixed".to_owned(),
        attempts: vec![
            Attempt {
                passed: true,
                note: "".to_owned(),
            },
            Attempt {
                passed: false,
                note: "boom".to_owned(),
            },
            Attempt {
                passed: true,
                note: "".to_owned(),
            },
        ],
    };
    let mixed_score = Score::of(&mixed_run);
    assert_eq!(
        mixed_score,
        Score {
            passed: 2,
            total: 3
        }
    );
    assert_eq!(mixed_score.cell(), "2/3");
    assert!(!mixed_score.all());
    assert!(mixed_score.some());
    assert_eq!(mixed_score.rate(), 67);
}

#[test]
fn direct_scores_round_half_up_and_handle_public_field_boundaries() {
    let cases: &[(Score, usize, bool, bool, Option<&str>)] = &[
        (
            Score {
                passed: 0,
                total: 2,
            },
            0,
            false,
            false,
            Some("0/2"),
        ),
        (
            Score {
                passed: 2,
                total: 2,
            },
            100,
            true,
            true,
            Some("2/2"),
        ),
        (
            Score {
                passed: 1,
                total: 3,
            },
            33,
            false,
            true,
            Some("1/3"),
        ),
        (
            Score {
                passed: 1,
                total: 8,
            },
            13,
            false,
            true,
            Some("1/8"),
        ),
        (
            Score {
                passed: 1,
                total: 6,
            },
            17,
            false,
            true,
            Some("1/6"),
        ),
        (
            Score {
                passed: 3,
                total: 2,
            },
            150,
            false,
            true,
            Some("3/2"),
        ),
        (
            Score {
                passed: usize::MAX,
                total: usize::MAX,
            },
            100,
            true,
            true,
            None,
        ),
        (
            Score {
                passed: 1,
                total: usize::MAX,
            },
            0,
            false,
            true,
            None,
        ),
        (
            Score {
                passed: usize::MAX,
                total: 1,
            },
            usize::MAX,
            false,
            true,
            None,
        ),
        (
            Score {
                passed: usize::MAX,
                total: 0,
            },
            0,
            false,
            true,
            None,
        ),
    ];

    for (score, rate, all, some, cell) in cases {
        assert_eq!(score.rate(), *rate);
        assert_eq!(score.all(), *all);
        assert_eq!(score.some(), *some);
        if let Some(expected_cell) = cell {
            assert_eq!(score.cell(), *expected_cell);
        }
    }
}

#[test]
fn public_runner_and_score_types_derive_the_required_traits() {
    let attempt = Attempt {
        passed: true,
        note: "".to_owned(),
    };
    assert_eq!(attempt.clone(), attempt);
    assert!(format!("{attempt:?}").contains("Attempt"));

    let case_run = CaseRun {
        id: "public".to_owned(),
        attempts: vec![attempt],
    };
    assert_eq!(case_run.clone(), case_run);
    assert!(format!("{case_run:?}").contains("CaseRun"));

    let score = Score::of(&case_run);
    assert_eq!(score.clone(), score);
    assert!(format!("{score:?}").contains("Score"));

    let provider: &mut dyn Provider = &mut Scripted::new(Replies(BTreeMap::from([(
        "public".to_owned(),
        vec![Entry::Text("ok".to_owned())],
    )])));
    assert_eq!(
        run(
            &[case(
                "public",
                "exact prompt",
                vec![Check::Equals("ok".to_owned())]
            )],
            provider,
            1
        ),
        vec![CaseRun {
            id: "public".to_owned(),
            attempts: vec![Attempt {
                passed: true,
                note: "".to_owned(),
            }],
        }]
    );
}
