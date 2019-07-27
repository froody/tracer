extern crate clap;
extern crate json;
mod trace;
use std::sync::atomic::Ordering;
use trace::trace_types::TraceEventType;

use clap::{App, Arg};

fn main() {
    let matches = App::new("Tracer")
        .version("1.0")
        .arg(
            Arg::with_name("chrome_trace")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let file = trace::google_trace::load_file(
        matches
            .value_of("chrome_trace")
            .expect("Must specify chrome_trace"),
    )
    .unwrap();

    println!(
        "Hello, world! {:?}",
        (
            file.threads.len(),
            file.async_events.len(),
            file.event_types.len()
        )
    );
    for event_type in file.event_types {
        let event_type: &TraceEventType = &event_type;
        println!(
            "event {}: {}",
            event_type.name,
            event_type.count.load(Ordering::SeqCst)
        );
    }
}
