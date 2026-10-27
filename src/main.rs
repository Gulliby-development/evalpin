use evalpin::cli::run;
use evalpin::osio::OsIo;

fn main() {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    std::process::exit(run(&args, &mut OsIo));
}
