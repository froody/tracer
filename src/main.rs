extern crate clap;
extern crate json;
mod trace;

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
        (file.threads.len(), file.async_events.len())
    );
}
