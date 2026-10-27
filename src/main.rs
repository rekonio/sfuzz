mod request;
mod spec;
mod tokens;

use clap::{Command, arg};
use oas3::spec::{ObjectOrReference, Schema};

fn main() {
    let matches = Command::new("sfuzz")
        .about("Simple blackbox fuzzer")
        .arg(arg!(--spec <VALUE>).required(true))
        .get_matches();

    let file = matches.get_one::<String>("spec").unwrap();
    let yaml = std::fs::read_to_string(file).unwrap();
    let doc = oas3::from_yaml(yaml).expect("cannot parse spec");

    spec::list_operations(&doc)
}
