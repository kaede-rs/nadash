use std::fmt;

#[derive(Debug, Clone)]
pub struct NadeError {
    pub msg: String,
    pub line: usize,
}

impl NadeError {
    pub fn new(msg: impl Into<String>, line: usize) -> Self {
        NadeError { msg: msg.into(), line }
    }
}

impl fmt::Display for NadeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.line, self.msg)
    }
}

impl std::error::Error for NadeError {}

pub type Result<T> = std::result::Result<T, NadeError>;
