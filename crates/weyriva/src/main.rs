fn main() -> std::process::ExitCode {
    match weyriva::cli::run() {
        Ok(code) => std::process::ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("weyriva: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
