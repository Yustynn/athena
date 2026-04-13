use athena_v2::benchmark::run_retrieval_benchmark;
use std::env;
use std::io;
use std::path::PathBuf;

fn default_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/retrieval/benchmark_spec.json")
}

fn usage() -> &'static str {
    "usage:
  athena-bench retrieval [--spec path]"
}

fn require_flag(args: &[String], index: &mut usize, flag: &str) -> Result<String, io::Error> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| io::Error::other(format!("{flag} requires value")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err(io::Error::other(usage()).into());
    }

    if args[0] != "retrieval" {
        return Err(io::Error::other(usage()).into());
    }

    let mut spec_path = default_spec_path();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--spec" => {
                spec_path = PathBuf::from(require_flag(&args, &mut index, "--spec")?);
            }
            "--help" | "-h" => return Err(io::Error::other(usage()).into()),
            other => return Err(io::Error::other(format!("unknown argument: {other}")).into()),
        }
        index += 1;
    }

    let report = run_retrieval_benchmark(spec_path)?;
    serde_json::to_writer_pretty(io::stdout(), &report)?;
    println!();
    Ok(())
}
