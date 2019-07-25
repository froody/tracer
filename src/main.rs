use std::env;
use std::error::Error;
use std::fmt;
use std::io;

extern crate clap;
extern crate json;
mod trace;
use trace::google_trace;

use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("Tracer")
        .version("1.0")
        .arg(
            Arg::with_name("chrome_trace")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let file = google_trace::load_file(
        matches
            .value_of("chrome_trace")
            .expect("Must specify chrome_trace"),
    )
    .unwrap();

    println!("Hello, world! {}", file.threads[0].events.len());
}
