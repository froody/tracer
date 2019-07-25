use super::trace_types::*;
use std::collections::HashMap;

static MISSING_TS: &str = "Missing 'ts' field";
static MISSING_PH: &str = "Missing 'ph' field";
static MISSING_ID: &str = "Missing 'id' field";
static MISSING_TID: &str = "Missing 'tid' field";
static NOT_ARRAY: &str = "Value isn't 'array'";

macro_rules! to_some {
    ($x:expr) => {
        $x.ok_or_else(|| format!("expected Some, got None on {}", line!()))?
    };
}

fn make_hash_map(object: &json::JsonValue) -> HashMap<&str, &json::JsonValue> {
    let mut ret = HashMap::new();
    for (key, value) in object.entries() {
        ret.insert(key, value);
    }

    ret
}

fn make_trace_event(
    dict: &HashMap<&str, &json::JsonValue>,
    finished: bool,
) -> Result<TraceEvent, TraceError> {
    Ok(TraceEvent {
        ts: dict
            .get("ts")
            .ok_or(MISSING_TS)?
            .as_u64()
            .ok_or("couldn't decode u64")?,
        dur: match dict.get("dur") {
            None => 0,
            Some(json) => json.as_u64().ok_or("couldn't decode u64")?,
        },
        tdur: match dict.get("tdur") {
            None => 0,
            Some(json) => json.as_u64().ok_or("couldn't decode u64")?,
        },
        finished: finished,
    })
}

pub fn load_file(filename: &str) -> Result<TraceFile, TraceError> {
    let buffer = std::fs::read_to_string(filename)?;
    load_json(buffer.as_str())
}

pub fn load_json(json_str: &str) -> Result<TraceFile, TraceError> {
    let loaded = json::parse(json_str)?;
    println!("loaded {}", loaded.is_array());
    if !loaded.is_array() {
        return Err(TraceError::new(NOT_ARRAY));
    }
    let mut threads: HashMap<u64, ThreadLoader> = HashMap::new();
    let mut async_events: Vec<TraceEvent> = Vec::new();
    let mut open_async_events: HashMap<String, Vec<usize>> = HashMap::new();
    let mut async_counter: usize = 0;
    let finished = false;
    let result: Result<Vec<()>, TraceError> = loaded
        .members()
        .map(|event| -> Result<(), TraceError> {
            if !event.is_object() {
                return Err(TraceError::new("non-object event"));
            }
            let event_map = make_hash_map(&event);
            let tid = event_map
                .get("tid")
                .ok_or(MISSING_TID)?
                .as_u64()
                .ok_or("tid not u64")?;
            let thread_loader: &mut ThreadLoader = threads
                .entry(tid)
                .or_insert_with(|| ThreadLoader::new(format!("{}", tid)));
            let value = event_map.get("ph").ok_or(MISSING_PH)?;
            let thing: &str = to_some!((*value).as_str());
            match thing {
                "B" => {
                    thread_loader.open_events.push(thread_loader.events.len());
                    thread_loader
                        .events
                        .push(make_trace_event(&event_map, finished)?);
                }
                "E" => {
                    let previous = thread_loader
                        .open_events
                        .pop()
                        .ok_or("no matching open event")?;
                    let previous = &mut thread_loader.events[previous];
                    let current = make_trace_event(&event_map, finished)?;
                    previous.finished = true;
                    previous.dur = current.ts - previous.ts;
                }
                "S" => {
                    let id: String = event_map["id"].as_str().ok_or(MISSING_ID)?.into();
                    open_async_events
                        .entry(id)
                        .or_insert_with(|| Vec::new())
                        .push(async_counter);
                    async_counter += 1;
                    async_events.push(make_trace_event(&event_map, finished)?);
                }
                "F" => {
                    let id: String = event_map["id"].as_str().ok_or(MISSING_ID)?.into();
                    match open_async_events.get_mut(&id) {
                        Some(entry) => {
                            let previous = entry.pop().ok_or("no matching open async event")?;
                            async_events[previous].finished = true;
                        }
                        None => println!("unpaired async event {} @ {}", id, event_map["ts"]),
                    }
                }
                "X" => thread_loader
                    .events
                    .push(make_trace_event(&event_map, finished)?),
                _ => (),
            }
            return Ok(());
        })
        .collect();

    result?;

    Ok(TraceFile {
        threads: threads
            .into_iter()
            .map(|(_, value)| value.get_thread())
            .collect(),
        async_events: async_events,
    })
}

#[test]
fn test_malformed() {
    let json = "{}";
    assert_eq!(load_json(json).err().unwrap(), TraceError::new(NOT_ARRAY));
    let json = "[{}]";
    assert_eq!(load_json(json).err().unwrap(), TraceError::new(MISSING_TID));
}

#[test]
fn test_empty() {
    let json = "[]";
    let result = load_json(json);
    assert!(result.is_ok());
    let trace_file = result.ok().unwrap();
    assert_eq!(trace_file.async_events.len(), 0);
    assert_eq!(trace_file.threads.len(), 0);
}

#[test]
fn test_single_event() {
    let json = "[{\"tid\": 123, \"ph\": \"X\", \"ts\":456}]";
    let result = load_json(json);
    assert!(result.is_ok());
    let trace_file = result.ok().unwrap();
    assert_eq!(trace_file.async_events.len(), 0);
    assert_eq!(trace_file.threads.len(), 1);
    let thread = &trace_file.threads[0];
    assert_eq!(thread.name, "123");
    assert_eq!(thread.events.len(), 1);
    let event = &thread.events[0];
    assert_eq!(event.ts, 456);
    assert_eq!(event.dur, 0);
    assert_eq!(event.tdur, 0);
}

#[test]
fn test_event_pair() {
    let json =
        "[{\"tid\": 123, \"ph\": \"B\", \"ts\":456}, {\"tid\": 123, \"ph\": \"E\", \"ts\":656}]";
    let result = load_json(json);
    assert!(result.is_ok());
    let trace_file = result.ok().unwrap();
    assert_eq!(trace_file.async_events.len(), 0);
    assert_eq!(trace_file.threads.len(), 1);
    let thread = &trace_file.threads[0];
    assert_eq!(thread.name, "123");
    assert_eq!(thread.events.len(), 1);
    let event = &thread.events[0];
    assert_eq!(event.ts, 456);
    assert_eq!(event.dur, 200);
    assert_eq!(event.tdur, 0);
}
