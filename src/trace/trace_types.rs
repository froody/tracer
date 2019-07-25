use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug, PartialEq)]
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

//impl From<core::option::NoneError> for TraceError {
//    fn from(err: core::option::NoneError) -> Self {
//        TraceError::new("none");
//    }
//}

#[derive(Debug)]
pub struct TraceThread {
    pub name: String,
    pub events: Vec<TraceEvent>,
}
impl TraceThread {
    pub fn new(name: String) -> TraceThread {
        TraceThread {
            name: name,
            events: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ThreadLoader {
    pub open_events: Vec<usize>,
    pub events: Vec<TraceEvent>,
    pub thread: TraceThread,
}
impl ThreadLoader {
    pub fn new(name: String) -> ThreadLoader {
        ThreadLoader {
            open_events: Vec::new(),
            events: Vec::new(),
            thread: TraceThread::new(name),
        }
    }
    pub fn get_thread(mut self) -> TraceThread {
        self.thread.events = self.events;
        println!("self events len {}", self.thread.events.len());
        self.thread
    }
}

#[derive(Debug)]
pub struct TraceEvent {
    pub ts: u64,
    pub dur: u64,
    pub tdur: u64,
    pub finished: bool,
}

#[derive(Debug)]
pub struct TraceFile {
    pub threads: Vec<TraceThread>,
    pub async_events: Vec<TraceEvent>,
}
