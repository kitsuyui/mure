use std::error;
use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum Error {
    Message(String),
}

impl Error {
    pub fn from_str(message: &str) -> Error {
        Error::Message(message.to_string())
    }
    pub fn message(&self) -> String {
        match self {
            Error::Message(message) => message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::from_str(&e.to_string())
    }
}