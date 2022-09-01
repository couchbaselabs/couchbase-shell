use nu_protocol::ShellError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct TestError {
    message: String,
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message.clone())
    }
}

impl Error for TestError {}

impl From<ShellError> for TestError {
    fn from(e: ShellError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl From<serde_json::Error> for TestError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl From<String> for TestError {
    fn from(message: String) -> Self {
        TestError { message }
    }
}
