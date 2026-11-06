pub trait Logger {
    fn warn(&mut self, message: &str);
}
