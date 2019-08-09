extern crate clap;
extern crate json;
mod trace;
use std::sync::atomic::Ordering;
use trace::trace_types::{TraceThread, TraceEventType};

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

    let threads_result : Vec<&TraceThread> = file.threads.iter().filter(|x| x.events.len() == 1754).collect();
    let thread = (*threads_result[0]).clone();
    let min_ts = thread.events[0].ts;
    let max_ts = thread.events.iter().map(|e| e.ts + e.dur).max().unwrap();
    let max_depth = thread.events.iter().max_by_key(|x| x.depth)
                .unwrap()
                .depth;
    let len = (max_ts - min_ts) as f64;
    println!("min, max len {:?}", (min_ts, max_ts, len));


    trace::graphics::render_loop(move ||  {
        let rects : Vec<f32> = thread.events.iter().flat_map(|e| {
            let x: f32 = ((e.ts - min_ts) as f64 / len) as f32;
            let y: f32 = 0.25 + 0.01 * e.depth as f32;
            let len: f32 =  (e.dur as f64 / len) as f32; 
            vec![x - 0.5, y, len]
        }).collect();
        rects
    });
}
