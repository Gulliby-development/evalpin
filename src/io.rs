use crate::text::{Failure, lines, name, normalize};

pub trait Io {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn write(&mut self, path: &str, text: &str) -> bool;
    fn out(&mut self, text: &str);
    fn err(&mut self, text: &str);
}

pub fn read_lines(io: &dyn Io, path: &str) -> Result<Vec<String>, Failure> {
    let bytes = io
        .read(path)
        .ok_or_else(|| Failure(format!("{}: cannot be read", name(path))))?;
    let text =
        normalize(&bytes).ok_or_else(|| Failure(format!("{}: not valid UTF-8", name(path))))?;
    Ok(lines(&text).into_iter().map(str::to_owned).collect())
}
