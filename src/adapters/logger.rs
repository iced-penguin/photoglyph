use crate::ports::Logger;
use std::io::Write;

pub struct CliLogger<W> {
    writer: W,
}

impl<W> CliLogger<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> Logger for CliLogger<W> {
    fn warn(&mut self, message: &str) {
        writeln!(self.writer, "warning: {message}").expect("failed printing to stderr");
    }
}

#[cfg(test)]
mod tests {
    use super::CliLogger;
    use crate::ports::Logger;
    use std::io::{self, Write};

    #[test]
    fn writes_warnings_in_order() {
        let mut output = Vec::new();
        let mut logger = CliLogger::new(&mut output);
        logger.warn("first");
        logger.warn("second");
        assert_eq!(output, b"warning: first\nwarning: second\n");
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn failing_writer_flush_succeeds() {
        assert!(FailingWriter.flush().is_ok());
    }

    #[test]
    #[should_panic(expected = "failed printing to stderr")]
    fn panics_on_write_failure() {
        CliLogger::new(FailingWriter).warn("message");
    }
}
