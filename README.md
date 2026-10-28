<p align="center"><img src="docs/logo.svg" width="96" alt="" /></p>

# evalpin

evalpin is an offline evaluation runner for people building with language models. Write a suite of prompts and checks, then run each case against deterministic scripted replies. Save results to compare later, or make a baseline regression fail in CI. This version has only the `Scripted` provider: it makes no model calls and needs no API key, account, network access, or model download.

[![CI](https://github.com/Gulliby-development/evalpin/actions/workflows/ci.yml/badge.svg)](https://github.com/Gulliby-development/evalpin/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Install

Install Rust stable with Cargo (the project uses Rust edition 2024), then use a checkout of this repository. There are no third-party Rust dependencies. In a POSIX shell, including Git Bash on Windows, set Cargo's offline mode before running the commands in this README:

```sh
export CARGO_NET_OFFLINE=true
```

From the repository root, install the local binary with Cargo:

```sh
cargo install --path . --offline
evalpin --help
```

Cargo must be on your `PATH`; after installation, Cargo's binary directory (usually `~/.cargo/bin`) must also be on your `PATH`. Alternatively, invoke the project from the checkout with `cargo run -q --` as in the Usage example. The shell transcripts below use POSIX `printf`; Windows users can run them in Git Bash.

## Usage

The command forms are:

```text
evalpin run SUITE --replies FILE [-k N] [--only ID]... [--out FILE] [--baseline FILE]
evalpin diff OLD NEW [--show all|changes]
evalpin --help
```

Options take a separate following argument; for example, write `--out results.txt`, not `--out=results.txt`. `run` requires `--replies FILE`. `-k N` runs each case N times, defaults to 1, and accepts integers from 1 through 20. Repeated `--only ID` options select only those suite cases; duplicate IDs are ignored and selected cases remain in suite order. An ID not in the suite is an error.

The run output reports each case's passed attempts, rounded percentage rate, and whether all attempts passed. Percentages are rounded to the nearest whole number, with halves rounded up. `pass^k` counts cases that passed all k attempts; `pass@k` counts cases that passed at least one. Failed attempts include the first failure note for each affected case. A reply can be a scripted failure as well as text.

Before running it, set `CARGO_NET_OFFLINE=true` as shown in Install. The first run exits 1 because one attempt fails; it saves `first.txt`. The improved run uses that file as its baseline, exits 0, and saves `second.txt`.

```sh
$ printf 'case greet\nprompt: Greet Ana.\nexpect contains: Ana\n' > suite.txt
$ printf 'reply greet\n| Hello Ana!\nreply greet\n| Hi there.\n' > replies.txt
$ cargo run -q -- run suite.txt --replies replies.txt -k 2 --out first.txt
CASE   PASSED  RATE  ALL
greet  1/2     50%   no

greet: attempt 2: reply does not contain "Ana"
evalpin: 1 cases x 2 attempts, pass^2 0/1, pass@2 1/1
$ printf 'reply greet\n| Hello Ana!\nreply greet\n| Hello again, Ana.\n' > replies.txt
$ cargo run -q -- run suite.txt --replies replies.txt -k 2 --out second.txt --baseline first.txt
CASE   PASSED  RATE  ALL  VS
greet  2/2     100%  yes  better
evalpin: 1 cases x 2 attempts, pass^2 1/1, pass@2 1/1; vs baseline: 1 better, 0 same, 0 worse, 0 new, 0 gone
```

Compare those two saved result files with this runnable command (it prints the diff report):

```sh
cargo run -q -- diff first.txt second.txt
```

Exit codes are 0 for a successful run or a diff with no worse case, 1 when a run fails or a diff contains a regression, and 2 for command, input, parsing, or output-write errors. Without a baseline, a run fails if any case is not fully passing. With `--baseline FILE`, a case fails only if it is worse than its baseline, or is new and not fully passing. Better and same cases never fail due to incomplete passes, and gone cases never fail. A baseline case omitted by `--only` is filtered, not gone; a baseline ID that is absent from the full suite is gone.

`diff OLD NEW` compares two results files and exits 1 if any case is worse, otherwise 0. It lists IDs in the new file's order followed by IDs only in the old file, sorted by code point. A run writes `--out FILE` after the run has completed and before printing its report. If the write fails, it exits 2 and prints no report. Inputs are read only from the paths named in the command.

`diff OLD NEW --show changes` leaves out the rows whose VS is `same`; the totals line still counts every case and the exit code does not change. `--show all` is the default and prints every row.

## Checks

Each suite case has one prompt and one or more `expect KIND: VALUE` lines. Before every check, the reply has leading and trailing ASCII space, TAB, CR and LF removed. Checks run in their listed order, and a failing check supplies that attempt's note.

| Kind | Value syntax | Meaning |
| --- | --- | --- |
| `equals` | Any non-empty text | The trimmed reply must exactly equal the value. |
| `contains` | Any non-empty text | The trimmed reply must contain the value as a substring. |
| `not-contains` | Any non-empty text | The trimmed reply must not contain the value as a substring. |
| `one-of` | Non-empty choices separated by `|` | The reply must equal one of the choices; surrounding spaces and tabs on each choice are removed. |
| `max-chars` | 1–6 ASCII digits | The reply may contain at most this many Unicode code points. |
| `min-chars` | 1–6 ASCII digits | The reply must contain at least this many Unicode code points. |
| `json` | `PATH = SCALAR` | Parse a JSON reply, look up PATH, and compare the result to one JSON scalar. |

Character limits count Unicode code points, not bytes; both limits include the stated boundary. Numeric check values may have leading zeros and may be zero. For JSON, paths start with `$` and may continue with `.name` object-key steps and `[N]` zero-based array-index steps. A JSON check expects a scalar (string, number, boolean, or null), not an array or object. Strings compare by decoded content; numbers compare by their exact JSON spelling, so `1` and `1.0` differ. A reply wrapped in a fenced block is checked using the lines inside the fence when it has at least two lines, starts with three backticks (any following text on that line is ignored), and ends with a line of exactly three backticks.

## Files

All three input formats are UTF-8 text. Invalid UTF-8 and NUL bytes are rejected. One leading UTF-8 BOM is ignored, and CRLF line endings are normalized to LF. In suite and replies files, only ASCII space and TAB count as surrounding blanks. File errors identify the path and line where applicable.

### Suite

A suite uses `case ID`, one `prompt: TEXT`, and one or more `expect KIND: VALUE` lines per case. Blank lines and lines beginning with `#` are ignored. Case IDs use lowercase ASCII letters, digits, `_` and `-`, and cannot start with `_` or `-`. This command creates a small suite:

```sh
printf 'case greet\nprompt: Greet Ana.\nexpect contains: Ana\nexpect max-chars: 24\n' > suite.txt
```

Prompts and check values may contain spaces. For `one-of`, separate choices with `|`; for `json`, use a path, equals sign, and JSON scalar, such as `expect json: $.user.name = "Ana"`.

### Replies

A `reply ID` opens a scripted reply. Each following line beginning with `|` in column one adds text; one space immediately after the pipe is removed, and text lines are joined with LF. A non-text line closes that reply. Repeated reply entries for an ID form an ordered cycle: attempts 1, 2, and so on use entries in order, wrapping around when necessary. A `fail ID MESSAGE` entry instead produces a failed attempt with that message. Blank lines and lines beginning with `#` are ignored, but a text line after either is outside a reply.

Create replies for the suite above with:

```sh
printf 'reply greet\n| Hello Ana!\nreply greet\n| Hi there.\nfail other rate limited\n' > replies.txt
```

Entries for IDs that are not in the suite are ignored. The example includes a failure entry to illustrate its syntax; the suite above does not use `other`.

### Results

A results file starts with the exact header `evalpin-results 1`, then has one row per case in the form `ID PASSED TOTAL`. Counts are decimal digits, TOTAL must be at least 1, and PASSED cannot exceed TOTAL. The `--out` option writes these rows in run order. For example, this command creates a valid previous result with one pass in two attempts:

```sh
printf 'evalpin-results 1\ngreet 1 2\n' > first.txt
```

Results are strict: fields are separated by single spaces, and blank rows, extra spaces, invalid counts, and duplicate IDs are errors. Use `--baseline FILE` to compare a run to a results file, or `diff OLD NEW` to compare two files.

## Development

The project targets Rust edition 2024 and uses only the standard library. CI runs on Linux, macOS, and Windows. The workflow provisions stable Rust with rustfmt and Clippy, then runs Cargo checks offline; toolchain provisioning may need network access, but the Cargo formatting, Clippy, and test steps use no network. The CI commands are:

```sh
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
```

Run them from the repository root. The repository is distributed under the MIT License; see [LICENSE](LICENSE).

---

Built with BAKED
