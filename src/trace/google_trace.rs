use super::trace_types::*;
use std::collections::HashMap;

fn make_hash_map(object: &json::JsonValue) -> HashMap<&str, &json::JsonValue> {
    let mut ret = HashMap::new();
    for (key, value) in object.entries() {
        ret.insert(key, value);
    }

    ret
}

fn make_trace_event(dict: &HashMap<&str, &json::JsonValue>) -> Result<TraceEvent, TraceError> {
    Ok(TraceEvent {
        ts: dict
            .get("ts")
            .ok_or("mising ts field")?
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
    })
}

pub fn load_file(filename: &str) -> Result<TraceFile, TraceError> {
    let buffer = std::fs::read_to_string(filename)?;
    let loaded = json::parse(buffer.as_str())?;
    println!("loaded {}", loaded.is_array());
    if !loaded.is_array() {
        return Err(TraceError::new("loaded isn't an array"));
    }
    let mut open_events: Vec<usize> = Vec::new();
    let mut fixups: Vec<TraceEvent> = Vec::new();
    let mut counter: usize = 0;
    let events: Result<Vec<TraceEvent>, TraceError> = loaded
        .members()
        .map(|event| -> Result<Option<TraceEvent>, TraceError> {
            if !event.is_object() {
                return Err(TraceError::new("non-object event"));
            }
            let event_map = make_hash_map(&event);
            let value = event_map.get("ph").ok_or(TraceError::new("missing ph"))?;
            let thing: &str = (*value)
                .as_str()
                .ok_or(TraceError::new("couldn't unwrap string"))?;
            match thing {
                "B" => open_events.push(counter),
                "E" => {fixups.push(make_trace_event(&event_map)?); return Ok(None);},
                "X" => (),
                _ => (),
            }
            counter += 1;
            Ok(Some(make_trace_event(&event_map)?))
        })
        .filter_map(|event : Result<Option<TraceEvent>, TraceError> | -> Option<Result<TraceEvent, TraceError>> {
            match event {
            Ok(None) => None,
            Ok(Some(e)) => Some(Ok(e)),
            Err(e) => Some(Err(e))
        }})
        .collect();

    Ok(TraceFile {
        threads: vec![TraceThread {
            name: String::from("taco"),
            events: events?,
        }],
    })
}
