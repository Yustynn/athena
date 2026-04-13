use athena_v2::benchmark::{
    run_creation_benchmark, run_retrieval_benchmark, run_trajectory_benchmark,
};
use std::env;
use std::io;
use std::path::PathBuf;

fn default_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/retrieval/benchmark_spec.json")
}

fn default_creation_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/creation/benchmark_spec.json")
}

fn default_trajectory_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/trajectory/jinja_tracer_bullet.json")
}

fn usage() -> &'static str {
    "usage:
  athena-bench retrieval [--spec path]
  athena-bench creation [--spec path] --proposals path
  athena-bench trajectory [--spec path] [--athena-mode off|current] [--keep-workdir]"
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

    match args[0].as_str() {
        "retrieval" => {
            let mut spec_path = default_spec_path();
            let mut index = 1;
            while index < args.len() {
                match args[index].as_str() {
                    "--spec" => {
                        spec_path = PathBuf::from(require_flag(&args, &mut index, "--spec")?);
                    }
                    "--help" | "-h" => return Err(io::Error::other(usage()).into()),
                    other => {
                        return Err(io::Error::other(format!("unknown argument: {other}")).into());
                    }
                }
                index += 1;
            }
            let report = run_retrieval_benchmark(spec_path)?;
            serde_json::to_writer_pretty(io::stdout(), &report)?;
            println!();
        }
        "creation" => {
            let mut spec_path = default_creation_spec_path();
            let mut proposals_path = None;
            let mut index = 1;
            while index < args.len() {
                match args[index].as_str() {
                    "--spec" => {
                        spec_path = PathBuf::from(require_flag(&args, &mut index, "--spec")?);
                    }
                    "--proposals" => {
                        proposals_path = Some(PathBuf::from(require_flag(
                            &args,
                            &mut index,
                            "--proposals",
                        )?));
                    }
                    "--help" | "-h" => return Err(io::Error::other(usage()).into()),
                    other => {
                        return Err(io::Error::other(format!("unknown argument: {other}")).into());
                    }
                }
                index += 1;
            }

            let proposals_path =
                proposals_path.ok_or_else(|| io::Error::other("--proposals path is required"))?;
            let report = run_creation_benchmark(spec_path, proposals_path)?;
            serde_json::to_writer_pretty(io::stdout(), &report)?;
            println!();
        }
        "trajectory" => {
            let mut spec_path = default_trajectory_spec_path();
            let mut athena_mode = String::from("off");
            let mut keep_workdir = false;
            let mut index = 1;
            while index < args.len() {
                match args[index].as_str() {
                    "--spec" => {
                        spec_path = PathBuf::from(require_flag(&args, &mut index, "--spec")?);
                    }
                    "--athena-mode" => {
                        athena_mode = require_flag(&args, &mut index, "--athena-mode")?;
                    }
                    "--keep-workdir" => {
                        keep_workdir = true;
                    }
                    "--help" | "-h" => return Err(io::Error::other(usage()).into()),
                    other => {
                        return Err(io::Error::other(format!("unknown argument: {other}")).into());
                    }
                }
                index += 1;
            }
            if athena_mode != "off" && athena_mode != "current" {
                return Err(io::Error::other("--athena-mode must be off or current").into());
            }
            let report = run_trajectory_benchmark(spec_path, &athena_mode, keep_workdir)?;
            serde_json::to_writer_pretty(io::stdout(), &report)?;
            println!();
        }
        _ => return Err(io::Error::other(usage()).into()),
    }
    Ok(())
}
