use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct TraceError {
    details: String,
}
impl TraceError {
    pub fn new(msg: &str) -> TraceError {
        TraceError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for TraceError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<io::Error> for TraceError {
    fn from(err: io::Error) -> Self {
        TraceError::new(err.description())
    }
}
impl From<json::Error> for TraceError {
    fn from(err: json::Error) -> Self {
        TraceError::new(err.description())
    }
}

impl From<String> for TraceError {
    fn from(err: String) -> Self {
        TraceError { details: err }
    }
}

impl From<&str> for TraceError {
    fn from(err: &str) -> Self {
        TraceError::new(err)
    }
}

pub struct TraceThread {
    pub name: String,
    pub events: Vec<TraceEvent>,
}

pub struct TraceEvent {
    pub ts: u64,
    pub dur: u64,
    pub tdur: u64,
}

pub struct TraceFile {
    pub threads: Vec<TraceThread>,
}
