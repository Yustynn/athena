use athena_v2::protocol::{AthenaRequest, handle_request};
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

fn fixture_path_from_args() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixture" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--fixture requires path".to_owned())?;
                fixture_path = PathBuf::from(value);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: athena-stdio [--fixture path] < request.json > response.json"
                        .to_owned(),
                );
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    Ok(fixture_path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture_path = fixture_path_from_args().map_err(io::Error::other)?;

    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;

    if raw.trim().is_empty() {
        return Err(io::Error::other("stdin request body was empty").into());
    }

    let request: AthenaRequest = serde_json::from_str(&raw)?;
    let response = handle_request(request, &fixture_path)?;
    serde_json::to_writer_pretty(io::stdout(), &response)?;
    println!();
    Ok(())
}
