mod spec;

use std::error::Error;
use std::io;
use std::time::Duration;
use clap::{Command, arg};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("sfuzz")
        .about("Simple blackbox fuzzer")
        .arg(arg!(--spec <VALUE>).required(true))
        .get_matches();

    let spec = matches.get_one::<String>("spec").unwrap();
    let yaml = load_spec(spec)?;
    let doc = oas3::from_yaml(yaml).expect("cannot parse spec");

    spec::generate_fuzz_template(&doc, &mut io::stderr(), None);

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
