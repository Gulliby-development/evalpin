use evalpin::cli;
use evalpin::io::Io;

struct FakeIo {
    old: &'static str,
    new: &'static str,
    outputs: Vec<String>,
    errors: Vec<String>,
}

impl Io for FakeIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        match path {
            "old" => Some(self.old.as_bytes().to_vec()),
            "new" => Some(self.new.as_bytes().to_vec()),
            _ => None,
        }
    }

    fn write(&mut self, _path: &str, _text: &str) -> bool {
        true
    }

    fn out(&mut self, text: &str) {
        self.outputs.push(text.to_owned());
    }

    fn err(&mut self, text: &str) {
        self.errors.push(text.to_owned());
    }
}

const OLD: &str = "evalpin-results 1\na 1 2\nb 1 1\nc 0 1\nz 1 1\n";
const NEW: &str = "evalpin-results 1\na 2 2\nb 1 1\nc 0 1\nd 1 1\n";

fn diff(extra: &[&str]) -> (i32, FakeIo) {
    let mut io = FakeIo {
        old: OLD,
        new: NEW,
        outputs: Vec::new(),
        errors: Vec::new(),
    };
    let mut args = vec!["diff".to_owned(), "old".to_owned(), "new".to_owned()];
    args.extend(extra.iter().map(|part| (*part).to_owned()));
    let status = cli::run(&args, &mut io);
    (status, io)
}

#[test]
fn default_and_all_print_every_row() {
    let expected = "CASE  OLD  NEW  VS\na     1/2  2/2  better\nb     1/1  1/1  same\nc     0/1  0/1  same\nd     -    1/1  new\nz     1/1  -    gone\nevalpin: 5 cases, 1 better, 2 same, 0 worse, 1 new, 1 gone\n";
    let (status, io) = diff(&[]);
    assert_eq!(status, 0);
    assert_eq!(io.outputs, vec![expected.to_owned()]);
    let (status, io) = diff(&["--show", "all"]);
    assert_eq!(status, 0);
    assert_eq!(io.outputs, vec![expected.to_owned()]);
}

#[test]
fn changes_leaves_out_the_same_rows() {
    let expected = "CASE  OLD  NEW  VS\na     1/2  2/2  better\nd     -    1/1  new\nz     1/1  -    gone\nevalpin: 5 cases, 1 better, 2 same, 0 worse, 1 new, 1 gone\n";
    let (status, io) = diff(&["--show", "changes"]);
    assert_eq!(status, 0);
    assert_eq!(io.outputs, vec![expected.to_owned()]);
    assert!(io.errors.is_empty());
}

#[test]
fn the_option_may_come_first() {
    let mut io = FakeIo {
        old: "evalpin-results 1\na 1 1\n",
        new: "evalpin-results 1\na 1 1\n",
        outputs: Vec::new(),
        errors: Vec::new(),
    };
    let args = ["diff", "--show", "changes", "old", "new"].map(str::to_owned);
    assert_eq!(cli::run(&args, &mut io), 0);
    assert_eq!(
        io.outputs,
        vec![
            "CASE  OLD  NEW  VS\nevalpin: 1 cases, 0 better, 1 same, 0 worse, 0 new, 0 gone\n"
                .to_owned()
        ]
    );
}

#[test]
fn a_regression_still_exits_1_and_shows_its_row() {
    let mut io = FakeIo {
        old: "evalpin-results 1\na 1 1\nb 1 1\n",
        new: "evalpin-results 1\na 0 1\nb 1 1\n",
        outputs: Vec::new(),
        errors: Vec::new(),
    };
    let args = ["diff", "old", "new", "--show", "changes"].map(str::to_owned);
    assert_eq!(cli::run(&args, &mut io), 1);
    assert_eq!(
        io.outputs,
        vec!["CASE  OLD  NEW  VS\na     1/1  0/1  worse\nevalpin: 2 cases, 0 better, 1 same, 1 worse, 0 new, 0 gone\n".to_owned()]
    );
}

#[test]
fn another_value_is_an_error() {
    let (status, io) = diff(&["--show", "some"]);
    assert_eq!(status, 2);
    assert!(io.outputs.is_empty());
    assert_eq!(
        io.errors,
        vec!["evalpin: --show needs all or changes, got some\n".to_owned()]
    );
    let (status, io) = diff(&["--show"]);
    assert_eq!(status, 2);
    assert_eq!(
        io.errors,
        vec!["evalpin: --show needs a value\n".to_owned()]
    );
    let (status, io) = diff(&["--show", "all", "--show", "changes"]);
    assert_eq!(status, 2);
    assert_eq!(io.errors, vec!["evalpin: --show given twice\n".to_owned()]);
}
