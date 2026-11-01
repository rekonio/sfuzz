mod spec;

use std::error::Error;
use std::time::Duration;
use clap::{Command, arg};
use sfuzz::request::{RequestLine, Request};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("sfuzz")
        .about("Simple blackbox fuzzer")
        .arg(arg!(--spec <VALUE>).required(true))
        .get_matches();

    let spec = matches.get_one::<String>("spec").unwrap();
    let yaml = load_spec(spec)?;
    let doc = oas3::from_yaml(yaml).expect("cannot parse spec");


    let mut buf: Vec<u8> = Vec::new();
    spec::generate_fuzz_template(&doc, &mut buf, None);

    let content = String::from_utf8(buf)?;
    let requests: Vec<Request> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.parse::<RequestLine>().map(Request::from))
        .collect::<Result<_, _>>()?;

    for r in requests {
        for c in r.generate_candidates() {
            println!("{}", c)
        }
    }

    Ok(())
}

fn load_spec(source: &str) -> Result<String, Box<dyn Error>>  {
    match source {
        url if url.starts_with("http") => {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()?;
            Ok(client
                .get(url)
                .send()?
                .error_for_status()?
                .text()?)
        }
        path => {
            Ok(std::fs::read_to_string(path)?)
        }
    }
}
