use std::env;
use std::io::{self, BufReader, BufWriter, Write};
use std::process::ExitCode;

use weyriva_luau_host::config::HostConfig;
use weyriva_luau_host::protocol::{fatal_event, serve};
use weyriva_luau_host::runtime::Host;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let event = fatal_event(&error);
            let mut output = BufWriter::new(io::stdout().lock());
            if serde_json::to_writer(&mut output, &event).is_ok() {
                let _ = output.write_all(b"\n");
                let _ = output.flush();
            }
            eprintln!("weyriva-luau-host: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> weyriva_luau_host::HostResult<()> {
    let config = HostConfig::from_args(env::args_os().skip(1))?;
    let host = Host::new(&config)?;
    let mut input = BufReader::new(io::stdin().lock());
    let mut output = BufWriter::new(io::stdout().lock());
    serve(&host, &mut input, &mut output)
        .map_err(|error| weyriva_luau_host::HostError::new("io_error", error.to_string()))
}
