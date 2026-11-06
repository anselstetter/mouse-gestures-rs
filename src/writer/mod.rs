#[cfg_attr(test, mockall::automock)]
pub trait Writer {
    fn write(&self, s: &str);
}

pub struct StdoutWriter;

impl StdoutWriter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Writer for StdoutWriter {
    fn write(&self, s: &str) {
        println!("{}", s);
    }
}
