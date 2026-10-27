use crate::io::Io;
use std::fs;
use std::io::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OsIo;

impl Io for OsIo {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(path).ok()
    }

    fn write(&mut self, path: &str, text: &str) -> bool {
        fs::write(path, text.as_bytes()).is_ok()
    }

    fn out(&mut self, text: &str) {
        let stdout = std::io::stdout();
        let mut stream = stdout.lock();
        let _ = stream.write_all(text.as_bytes());
        let _ = stream.flush();
    }

    fn err(&mut self, text: &str) {
        let stderr = std::io::stderr();
        let mut stream = stderr.lock();
        let _ = stream.write_all(text.as_bytes());
        let _ = stream.flush();
    }
}
