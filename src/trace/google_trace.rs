use super::trace_types::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;

static NOT_ARRAY: &str = "Value isn't 'array'";

type EventMap<'a> = HashMap<&'a str, &'a json::JsonValue>;

fn make_hash_map(object: &json::JsonValue) -> EventMap {
    let mut ret = HashMap::new();
    for (key, value) in object.entries() {
        ret.insert(key, value);
    }

    ret
}

pub fn get_str<'a>(event_map: &'a EventMap, name: &str) -> Result<&'a str, TraceError> {
    let result = event_map
        .get(name)
        .ok_or_else(|| format!("Missing '{}' field", name))?
        .as_str()
        .ok_or("Expected Some, got None")?;
    Ok(result)
}

pub fn get_u64<'a>(event_map: &'a EventMap, name: &str) -> Result<u64, TraceError> {
    let result = event_map
        .get(name)
        .ok_or_else(|| format!("Missing '{}' field", name))?
        .as_u64()
        .ok_or("Expected Some, got None")?;
    Ok(result)
}

fn make_trace_event(
    dict: &EventMap,
    event_type: Rc<TraceEventType>,
    finished: bool,
) -> Result<TraceEvent, TraceError> {
    Ok(TraceEvent {
        ts: get_u64(dict, "ts")?,
        dur: match dict.get("dur") {
            None => 0,
            Some(json) => json.as_u64().ok_or("couldn't decode u64")?,
        },
        tdur: match dict.get("tdur") {
            None => 0,
            Some(json) => json.as_u64().ok_or("couldn't decode u64")?,
        },
        finished: finished,
        depth: 0,
        event_type: event_type,
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
    let finished = false;
    let mut loader = TraceLoader::new();

    let result: Result<Vec<()>, TraceError> = loaded
        .members()
        .map(|event| -> Result<(), TraceError> {
            if !event.is_object() {
                return Err(TraceError::new("non-object event"));
            }
            let event_map = make_hash_map(&event);
            let tid = get_u64(&event_map, "tid")?;
            let phase = get_str(&event_map, "ph")?;
            let name: &str = get_str(&event_map, "name")?;

            match phase {
                "B" => {
                    let event_type = loader.get_event_type(name, TraceEventClass::BeginEnd)?;
                    loader.add_event(tid, make_trace_event(&event_map, event_type, false)?)?;
                }
                "E" => {
                    let event_type = loader.get_event_type(name, TraceEventClass::BeginEnd)?;
                    loader.add_event(tid, make_trace_event(&event_map, event_type, true)?)?;
                }
                "S" => {
                    let id: &str = get_str(&event_map, "id")?;
                    let event_type = loader.get_event_type(name, TraceEventClass::Async)?;
                    loader.add_async_event(
                        tid,
                        TraceEventClass::Async,
                        name,
                        id,
                        make_trace_event(&event_map, event_type, false)?,
                    )?;
                }
                "F" => {
                    let id: &str = get_str(&event_map, "id")?;
                    let event_type = loader.get_event_type(name, TraceEventClass::Async)?;
                    loader.add_async_event(
                        tid,
                        TraceEventClass::Async,
                        name,
                        id,
                        make_trace_event(&event_map, event_type, true)?,
                    )?;
                }
                "X" => {
                    let event_type = loader.get_event_type(name, TraceEventClass::Standalone)?;
                    loader.add_event(tid, make_trace_event(&event_map, event_type, true)?)?;
                }
                _ => (),
            }
            return Ok(());
        })
        .collect();

    result?;

    loader.trace_file()
}

#[test]
fn test_malformed() {
    let json = "{}";
    assert_eq!(load_json(json).err().unwrap(), TraceError::new(NOT_ARRAY));
    let json = "[{}]";
    assert_eq!(
        load_json(json).err().unwrap(),
        TraceError::new("Missing 'tid' field")
    );
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
    let json = "[{\"tid\": 123, \"ph\": \"X\", \"ts\":456, \"name\": \"eventA\"}]";
    let result = load_json(json);
    if result.is_err() {
        println!("result {:?}", result);
    }
    assert!(result.is_ok());
    let trace_file = result.ok().unwrap();
    assert_eq!(trace_file.async_events.len(), 0);
    assert_eq!(trace_file.threads.len(), 1);
    assert_eq!(trace_file.event_types.len(), 1);
    let thread = &trace_file.threads[0];
    assert_eq!(thread.name, "123");
    assert_eq!(thread.events.len(), 1);
    let event = &thread.events[0];
    assert_eq!(event.ts, 456);
    assert_eq!(event.dur, 0);
    assert_eq!(event.tdur, 0);
    assert_eq!(event.depth, 0);
    let event_type = &trace_file.event_types[0];
    assert_eq!(event_type.name, "eventA");
    assert_eq!(event_type.count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_event_pair() {
    let json =
        "[{\"tid\": 123, \"ph\": \"B\", \"ts\":456, \"name\": \"eventA\"}, {\"tid\": 123, \"ph\": \"E\", \"ts\":656, \"name\": \"eventA\"}]";
    let result = load_json(json);
    if result.is_err() {
        println!("result {:?}", result);
    }
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
    assert_eq!(event.depth, 0);
    assert_eq!(trace_file.event_types.len(), 1);
    let event_type = &trace_file.event_types[0];
    assert_eq!(event_type.name, "eventA");
    assert_eq!(event_type.count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_nested_event_pair() {
    let json = "[\
                {\"tid\": 123, \"ph\": \"B\", \"ts\":456, \"name\": \"eventA\"}, \
                {\"tid\": 123, \"ph\": \"B\", \"ts\":556, \"name\": \"eventB\"}, \
                {\"tid\": 123, \"ph\": \"E\", \"ts\":600, \"name\": \"eventB\"}, \
                {\"tid\": 123, \"ph\": \"E\", \"ts\":656, \"name\": \"eventA\"}, \
                {\"tid\": 123, \"ph\": \"X\", \"ts\":580, \"name\": \"eventC\", \"dur\": 10} \
                ]";
    let result = load_json(json);
    if result.is_err() {
        println!("result {:?}", result);
    }
    assert!(result.is_ok());
    let trace_file = result.ok().unwrap();
    assert_eq!(trace_file.async_events.len(), 0);
    assert_eq!(trace_file.threads.len(), 1);
    let thread = &trace_file.threads[0];
    assert_eq!(thread.name, "123");
    assert_eq!(thread.events.len(), 3);
    let event_a = &thread.events[0];
    assert_eq!(event_a.ts, 456);
    assert_eq!(event_a.dur, 200);
    assert_eq!(event_a.tdur, 0);
    assert_eq!(event_a.depth, 0);
    let event_b = &thread.events[1];
    assert_eq!(event_b.ts, 556);
    assert_eq!(event_b.dur, 44);
    assert_eq!(event_b.tdur, 0);
    assert_eq!(event_b.depth, 1);
    let event_c = &thread.events[2];
    assert_eq!(event_c.ts, 580);
    assert_eq!(event_c.dur, 10);
    assert_eq!(event_c.tdur, 0);
    assert_eq!(event_c.depth, 2);
    assert_eq!(trace_file.event_types.len(), 3);
    let event_type_a = trace_file
        .event_types
        .iter()
        .find(|&x| x.name == "eventA")
        .unwrap();
    assert_eq!(event_type_a.name, "eventA");
    assert_eq!(event_type_a.count.load(Ordering::SeqCst), 1);
    let event_type_b = trace_file
        .event_types
        .iter()
        .find(|&x| x.name == "eventB")
        .unwrap();
    assert_eq!(event_type_b.name, "eventB");
    assert_eq!(event_type_b.count.load(Ordering::SeqCst), 1);
    let event_type_c = trace_file
        .event_types
        .iter()
        .find(|&x| x.name == "eventC")
        .unwrap();
    assert_eq!(event_type_c.name, "eventC");
    assert_eq!(event_type_c.count.load(Ordering::SeqCst), 1);
}
