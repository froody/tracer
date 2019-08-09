use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, PartialEq)]
pub struct TraceError {
    details: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TraceEventClass {
    Standalone,
    BeginEnd,
    Async,
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

#[derive(Debug, Clone)]
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

pub fn set_depth(mut events: Vec<TraceEvent>) -> Result<Vec<TraceEvent>, TraceError> {
    events.sort_by_key(|event| event.ts);
    let mut new_events: Vec<TraceEvent> = Vec::with_capacity(events.len());
    let mut open_events: Vec<usize> = Vec::new();
    let thing: Result<Vec<()>, TraceError> = events
        .into_iter()
        .map(|mut event: TraceEvent| -> Result<(), TraceError> {
            match (*event.event_type).class {
                TraceEventClass::Standalone => {
                    event.depth = open_events.len() as u8;
                    new_events.push(event);
                }
                TraceEventClass::BeginEnd => {
                    if event.finished == true {
                        if open_events.len() == 0 {
                            println!("")
                        }
                        let position = open_events.pop().ok_or("No matching open event")?;
                        let begin = &mut new_events[position];
                        begin.dur = event.ts - begin.ts;
                    } else {
                        event.depth = open_events.len() as u8;
                        open_events.push(new_events.len());
                        new_events.push(event);
                    }
                }
                TraceEventClass::Async => {}
            }
            Ok(())
        })
        .collect();
    thing?;
    Ok(new_events)
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
    pub fn get_thread(mut self) -> Result<TraceThread, TraceError> {
        self.thread.events = set_depth(self.events)?;

        println!(
            "self events len {}, {}",
            self.thread.events.len(),
            self.thread
                .events
                .iter()
                .max_by_key(|x| x.depth)
                .ok_or("no events")?
                .depth
        );
        Ok(self.thread)
    }
    pub fn add_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }
}

pub struct TraceLoader {
    pub threads: HashMap<u64, ThreadLoader>,
    pub async_thread_loader: ThreadLoader,
    pub event_types: HashMap<String, Rc<TraceEventType>>,
}

impl TraceLoader {
    pub fn new() -> TraceLoader {
        TraceLoader {
            threads: HashMap::new(),
            async_thread_loader: ThreadLoader::new(String::from("__async__")),
            event_types: HashMap::new(),
        }
    }
    pub fn get_event_type(
        &mut self,
        event_name: &str,
        event_class: TraceEventClass,
    ) -> Result<Rc<TraceEventType>, TraceError> {
        let event_type = self
            .event_types
            .entry(event_name.into())
            .or_insert_with(|| {
                Rc::new(TraceEventType::new(event_class.clone(), event_name.into()))
            });

        if (*event_type).class != event_class {
            return Err(TraceError {
                details: format!(
                    "Mismatched class for type {}, {:?} != {:?}",
                    event_name, event_class, event_type.class
                ),
            });
        }
        Ok(event_type.clone())
    }
    pub fn add_event(&mut self, tid: u64, event: TraceEvent) -> Result<(), TraceError> {
        let thread_loader: &mut ThreadLoader = self
            .threads
            .entry(tid)
            .or_insert_with(|| ThreadLoader::new(format!("{}", tid)));

        if (*event.event_type).class != TraceEventClass::BeginEnd || !event.finished {
            (*event.event_type).count.fetch_add(1, Ordering::SeqCst);
        }

        thread_loader.add_event(event);

        Ok(())
    }

    pub fn add_async_event(
        &mut self,
        tid: u64,
        event_class: TraceEventClass,
        event_name: &str,
        id: &str,
        event: TraceEvent,
    ) -> Result<(), TraceError> {
        Ok(())
    }
    pub fn trace_file(self) -> Result<TraceFile, TraceError> {
        Ok(TraceFile {
            threads: self
                .threads
                .into_iter()
                .map(|(_, value)| -> Result<TraceThread, TraceError> { value.get_thread() })
                .collect::<Result<Vec<TraceThread>, TraceError>>()?,
            async_events: Vec::new(),
            event_types: self
                .event_types
                .into_iter()
                .map(|(_, value)| value)
                .collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub ts: u64,
    pub dur: u64,
    pub tdur: u64,
    pub finished: bool,
    pub depth: u8,
    pub event_type: Rc<TraceEventType>,
}

#[derive(Debug)]
pub struct TraceEventType {
    pub name: String,
    pub class: TraceEventClass,
    pub count: AtomicUsize,
}

impl TraceEventType {
    pub fn new(class: TraceEventClass, name: String) -> TraceEventType {
        TraceEventType {
            class: class,
            name: name,
            count: AtomicUsize::new(0),
        }
    }
}

#[derive(Debug)]
pub struct TraceFile {
    pub threads: Vec<TraceThread>,
    pub async_events: Vec<TraceEvent>,
    pub event_types: Vec<Rc<TraceEventType>>,
}
