use crate::error::AthenaError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static RUN_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryBenchmarkSpec {
    pub name: String,
    pub sequence_id: String,
    pub repo: TrajectoryRepoSpec,
    pub runner: TrajectoryRunnerSpec,
    pub steps: Vec<TrajectoryBenchmarkStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryRepoSpec {
    pub repo_id: String,
    pub source: TrajectoryRepoSource,
    #[serde(default)]
    pub setup_commands: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrajectoryRepoSource {
    Local { path: String },
    Git { clone_url: String, base_rev: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryRunnerSpec {
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryBenchmarkStep {
    pub step_id: String,
    pub prompt_path: String,
    pub verifier: TrajectoryVerifierSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryVerifierSpec {
    pub parser: TrajectoryParserKind,
    pub command: Vec<String>,
    pub test_patch_path: Option<String>,
    pub fail_to_pass: Vec<String>,
    pub pass_to_pass: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryParserKind {
    Pytest,
    Unittest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryDataSource {
    CodexEventLog,
    GitDiff,
    RunnerStdout,
    RunnerStderr,
    VerifierStdout,
    VerifierStderr,
    DerivedFromMultiple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub source: TrajectoryDataSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryToolCount {
    pub tool_name: String,
    pub count: u64,
    pub source: TrajectoryDataSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryFileObservation {
    pub path: String,
    pub count: u64,
    pub source: TrajectoryDataSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryFailureDescription {
    pub text: String,
    pub source: TrajectoryDataSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryBenchmarkReport {
    pub name: String,
    pub sequence_id: String,
    pub repo_id: String,
    pub athena_mode: String,
    pub kept_run_root: Option<String>,
    pub step_results: Vec<TrajectoryStepResult>,
    pub overall: TrajectoryBenchmarkAggregate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryStepResult {
    pub step_id: String,
    pub runner_exit_code: Option<i32>,
    pub runner_wall_time_ms: u128,
    pub runner_event_log_path: Option<String>,
    pub verifier_exit_code: Option<i32>,
    pub verifier_wall_time_ms: u128,
    pub test_patch_applied: bool,
    pub fail_to_pass_rate: f64,
    pub pass_to_pass_rate: f64,
    pub resolved: bool,
    pub usage: Option<TrajectoryUsage>,
    pub tool_counts: Vec<TrajectoryToolCount>,
    pub observed_read_files: Vec<TrajectoryFileObservation>,
    pub observed_edit_files: Vec<TrajectoryFileObservation>,
    pub changed_files: Vec<TrajectoryFileObservation>,
    pub failure_description: Option<TrajectoryFailureDescription>,
    pub tests_status: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryBenchmarkAggregate {
    pub step_count: usize,
    pub resolved_count: usize,
    pub resolution_rate: f64,
    pub total_runner_wall_time_ms: u128,
    pub total_verifier_wall_time_ms: u128,
    pub total_wall_time_ms: u128,
}

pub fn run_trajectory_benchmark(
    spec_path: impl AsRef<Path>,
    athena_mode: &str,
    keep_workdir: bool,
) -> Result<TrajectoryBenchmarkReport, AthenaError> {
    let spec_path = fs::canonicalize(spec_path.as_ref())?;
    let spec: TrajectoryBenchmarkSpec = read_json(&spec_path)?;
    let base_dir = spec_path
        .parent()
        .ok_or_else(|| std::io::Error::other("benchmark spec path missing parent"))?;
    let run_root = new_run_root()?;
    let repo_dir = run_root.join("repo");
    let run_started = Instant::now();

    materialize_repo(base_dir, &spec.repo, &repo_dir)?;
    if !repo_dir.join(".git").exists() {
        initialize_git_repo(&repo_dir)?;
    }
    for command in &spec.repo.setup_commands {
        run_command_checked(command, &repo_dir, None)?;
    }

    let mut step_results = Vec::new();
    for step in &spec.steps {
        let step_root = run_root.join("steps").join(&step.step_id);
        fs::create_dir_all(&step_root)?;

        let prompt_path = base_dir.join(&step.prompt_path);
        let message_file = step_root.join("last-message.txt");
        let runner_started = Instant::now();
        let runner_output = run_command(
            &resolve_command(base_dir, &spec.runner.command),
            &repo_dir,
            Some(benchmark_env(
                &spec.runner.env,
                athena_mode,
                &repo_dir,
                &step.step_id,
                &prompt_path,
                &message_file,
            )),
        )?;
        let runner_wall_time_ms = runner_started.elapsed().as_millis();
        let runner_stdout_path = step_root.join("runner.stdout.txt");
        let runner_stderr_path = step_root.join("runner.stderr.txt");
        fs::write(&runner_stdout_path, &runner_output.stdout)?;
        fs::write(&runner_stderr_path, &runner_output.stderr)?;
        let runner_telemetry = parse_runner_telemetry(&runner_output.stdout, &repo_dir);
        let changed_files = collect_changed_files(&repo_dir)?;
        let diff_stat_path = step_root.join("git.diff.stat.txt");
        let diff_stat = git_diff_stat(&repo_dir)?;
        fs::write(&diff_stat_path, &diff_stat)?;

        let verifier_dir = run_root.join("verifier").join(&step.step_id);
        copy_dir_all(&repo_dir, &verifier_dir)?;
        if !verifier_dir.join(".git").exists() {
            initialize_git_repo(&verifier_dir)?;
        }

        let mut test_patch_applied = false;
        if let Some(test_patch_path) = &step.verifier.test_patch_path {
            let patch_path = base_dir.join(test_patch_path);
            run_command_checked(
                &[
                    "git".to_string(),
                    "apply".to_string(),
                    "--whitespace=nowarn".to_string(),
                    patch_path.to_string_lossy().into_owned(),
                ],
                &verifier_dir,
                None,
            )?;
            test_patch_applied = true;
        }

        let verifier_started = Instant::now();
        let verifier_output = run_command(
            &resolve_command(base_dir, &step.verifier.command),
            &verifier_dir,
            Some(verifier_env(&repo_dir)),
        )?;
        let verifier_wall_time_ms = verifier_started.elapsed().as_millis();
        let verifier_stdout_path = step_root.join("verifier.stdout.txt");
        let verifier_stderr_path = step_root.join("verifier.stderr.txt");
        fs::write(&verifier_stdout_path, &verifier_output.stdout)?;
        fs::write(&verifier_stderr_path, &verifier_output.stderr)?;

        let tests_status = parse_test_statuses(
            &format!("{}\n{}", verifier_output.stdout, verifier_output.stderr),
            &step.verifier.parser,
        );
        let fail_to_pass_rate = success_rate(&step.verifier.fail_to_pass, &tests_status);
        let pass_to_pass_rate = success_rate(&step.verifier.pass_to_pass, &tests_status);
        let resolved = fail_to_pass_rate == 1.0 && pass_to_pass_rate == 1.0;
        let failure_description = if runner_output.exit_code != Some(0) {
            summarize_failure(
                &runner_output.stderr,
                &runner_output.stdout,
                TrajectoryDataSource::RunnerStderr,
                TrajectoryDataSource::RunnerStdout,
            )
        } else if verifier_output.exit_code != Some(0) {
            summarize_failure(
                &verifier_output.stderr,
                &verifier_output.stdout,
                TrajectoryDataSource::VerifierStderr,
                TrajectoryDataSource::VerifierStdout,
            )
        } else {
            None
        };

        step_results.push(TrajectoryStepResult {
            step_id: step.step_id.clone(),
            runner_exit_code: runner_output.exit_code,
            runner_wall_time_ms,
            runner_event_log_path: keep_workdir.then(|| runner_stdout_path.to_string_lossy().into_owned()),
            verifier_exit_code: verifier_output.exit_code,
            verifier_wall_time_ms,
            test_patch_applied,
            fail_to_pass_rate,
            pass_to_pass_rate,
            resolved,
            usage: runner_telemetry.usage,
            tool_counts: runner_telemetry.tool_counts,
            observed_read_files: runner_telemetry.observed_read_files,
            observed_edit_files: runner_telemetry.observed_edit_files,
            changed_files,
            failure_description,
            tests_status,
        });
    }

    let total_wall_time_ms = run_started.elapsed().as_millis();
    let resolved_count = step_results.iter().filter(|result| result.resolved).count();
    let total_runner_wall_time_ms = step_results
        .iter()
        .map(|result| result.runner_wall_time_ms)
        .sum();
    let total_verifier_wall_time_ms = step_results
        .iter()
        .map(|result| result.verifier_wall_time_ms)
        .sum();

    let report = TrajectoryBenchmarkReport {
        name: spec.name,
        sequence_id: spec.sequence_id,
        repo_id: spec.repo.repo_id,
        athena_mode: athena_mode.to_string(),
        kept_run_root: keep_workdir.then(|| run_root.to_string_lossy().into_owned()),
        overall: TrajectoryBenchmarkAggregate {
            step_count: step_results.len(),
            resolved_count,
            resolution_rate: safe_rate(resolved_count, step_results.len()),
            total_runner_wall_time_ms,
            total_verifier_wall_time_ms,
            total_wall_time_ms,
        },
        step_results,
    };

    if !keep_workdir {
        fs::remove_dir_all(&run_root)?;
    }

    Ok(report)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, AthenaError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn new_run_root() -> Result<PathBuf, AthenaError> {
    let parent = std::env::var_os("ATHENA_BENCH_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&parent)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();

    for _ in 0..1024 {
        let counter = RUN_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let run_root = parent.join(format!("trajectory-run-{nonce}-{pid}-{counter}"));
        match fs::create_dir(&run_root) {
            Ok(()) => return Ok(run_root),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Err(std::io::Error::other("failed to allocate unique trajectory run root").into())
}

fn materialize_repo(
    base_dir: &Path,
    repo: &TrajectoryRepoSpec,
    repo_dir: &Path,
) -> Result<(), AthenaError> {
    match &repo.source {
        TrajectoryRepoSource::Local { path } => {
            let source_path = base_dir.join(path);
            copy_dir_all(&source_path, repo_dir)?;
        }
        TrajectoryRepoSource::Git {
            clone_url,
            base_rev,
        } => {
            run_command_checked(
                &[
                    "git".to_string(),
                    "clone".to_string(),
                    "--quiet".to_string(),
                    clone_url.clone(),
                    repo_dir.to_string_lossy().into_owned(),
                ],
                base_dir,
                None,
            )?;
            run_command_checked(
                &[
                    "git".to_string(),
                    "checkout".to_string(),
                    "--quiet".to_string(),
                    base_rev.clone(),
                ],
                repo_dir,
                None,
            )?;
        }
    }
    Ok(())
}

fn initialize_git_repo(repo_dir: &Path) -> Result<(), AthenaError> {
    run_command_checked(
        &["git".into(), "init".into(), "--quiet".into()],
        repo_dir,
        None,
    )?;
    run_command_checked(
        &[
            "git".into(),
            "config".into(),
            "user.email".into(),
            "athena-bench@example.com".into(),
        ],
        repo_dir,
        None,
    )?;
    run_command_checked(
        &[
            "git".into(),
            "config".into(),
            "user.name".into(),
            "Athena Bench".into(),
        ],
        repo_dir,
        None,
    )?;
    run_command_checked(&["git".into(), "add".into(), "-A".into()], repo_dir, None)?;
    run_command_checked(
        &[
            "git".into(),
            "commit".into(),
            "--quiet".into(),
            "--allow-empty".into(),
            "-m".into(),
            "benchmark base".into(),
        ],
        repo_dir,
        None,
    )?;
    Ok(())
}

fn benchmark_env(
    extra_env: &BTreeMap<String, String>,
    athena_mode: &str,
    repo_dir: &Path,
    step_id: &str,
    prompt_path: &Path,
    message_file: &Path,
) -> BTreeMap<String, String> {
    let mut env = extra_env.clone();
    env.insert(
        "ATHENA_TRAJECTORY_REPO_DIR".into(),
        repo_dir.to_string_lossy().into_owned(),
    );
    env.insert("ATHENA_TRAJECTORY_STEP_ID".into(), step_id.to_string());
    env.insert(
        "ATHENA_TRAJECTORY_STEP_PROMPT_FILE".into(),
        prompt_path.to_string_lossy().into_owned(),
    );
    env.insert(
        "ATHENA_TRAJECTORY_MESSAGE_FILE".into(),
        message_file.to_string_lossy().into_owned(),
    );
    env.insert(
        "ATHENA_TRAJECTORY_ATHENA_MODE".into(),
        athena_mode.to_string(),
    );
    env
}

fn verifier_env(repo_dir: &Path) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    let venv_dir = repo_dir.join(".venv");
    let venv_bin = venv_dir.join("bin");
    if venv_bin.exists() {
        let existing_path = std::env::var("PATH").unwrap_or_default();
        let mut path = venv_bin.to_string_lossy().into_owned();
        if !existing_path.is_empty() {
            path.push(':');
            path.push_str(&existing_path);
        }
        env.insert("PATH".into(), path);
        env.insert(
            "VIRTUAL_ENV".into(),
            venv_dir.to_string_lossy().into_owned(),
        );
    }
    env
}

struct CommandOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Default)]
struct ParsedRunnerTelemetry {
    usage: Option<TrajectoryUsage>,
    tool_counts: Vec<TrajectoryToolCount>,
    observed_read_files: Vec<TrajectoryFileObservation>,
    observed_edit_files: Vec<TrajectoryFileObservation>,
}

fn resolve_command(base_dir: &Path, command: &[String]) -> Vec<String> {
    let Some((program, args)) = command.split_first() else {
        return Vec::new();
    };

    let mut resolved = Vec::with_capacity(command.len());
    if should_resolve_path(program) {
        resolved.push(base_dir.join(program).to_string_lossy().into_owned());
    } else {
        resolved.push(program.clone());
    }

    if is_shell_program(program) {
        if let Some((script, rest)) = args.split_first() {
            if should_resolve_path(script) {
                resolved.push(base_dir.join(script).to_string_lossy().into_owned());
            } else {
                resolved.push(script.clone());
            }
            resolved.extend(rest.iter().cloned());
            return resolved;
        }
    }

    resolved.extend(args.iter().cloned());
    resolved
}

fn should_resolve_path(value: &str) -> bool {
    value.contains('/') && !Path::new(value).is_absolute()
}

fn is_shell_program(program: &str) -> bool {
    matches!(program, "bash" | "sh" | "/bin/bash" | "/bin/sh")
}

fn run_command(
    command: &[String],
    cwd: &Path,
    extra_env: Option<BTreeMap<String, String>>,
) -> Result<CommandOutput, AthenaError> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| std::io::Error::other("command cannot be empty"))?;
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.current_dir(cwd);
    if let Some(extra_env) = extra_env {
        cmd.envs(extra_env);
    }
    let output = cmd.output()?;
    Ok(CommandOutput {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn run_command_checked(
    command: &[String],
    cwd: &Path,
    extra_env: Option<BTreeMap<String, String>>,
) -> Result<CommandOutput, AthenaError> {
    let output = run_command(command, cwd, extra_env)?;
    if output.exit_code == Some(0) {
        return Ok(output);
    }

    Err(std::io::Error::other(format!(
        "command failed in {}: {:?}\nstdout:\n{}\nstderr:\n{}",
        cwd.display(),
        command,
        output.stdout,
        output.stderr
    ))
    .into())
}

fn parse_runner_telemetry(output: &str, repo_dir: &Path) -> ParsedRunnerTelemetry {
    let mut usage = None;
    let mut tool_counts = BTreeMap::<String, u64>::new();
    let mut observed_read_files = BTreeMap::<String, u64>::new();
    let mut observed_edit_files = BTreeMap::<String, u64>::new();
    let canonical_repo_dir = fs::canonicalize(repo_dir).unwrap_or_else(|_| repo_dir.to_path_buf());

    for line in output.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        match event.get("type").and_then(Value::as_str) {
            Some("turn.completed") => {
                if let Some(raw_usage) = event.get("usage") {
                    let input_tokens = raw_usage.get("input_tokens").and_then(Value::as_u64);
                    let cached_input_tokens =
                        raw_usage.get("cached_input_tokens").and_then(Value::as_u64);
                    let output_tokens = raw_usage.get("output_tokens").and_then(Value::as_u64);
                    usage = Some(TrajectoryUsage {
                        input_tokens,
                        cached_input_tokens,
                        output_tokens,
                        total_tokens: combine_tokens(input_tokens, output_tokens),
                        source: TrajectoryDataSource::CodexEventLog,
                    });
                }
            }
            Some("item.completed") => {
                let Some(item) = event.get("item") else {
                    continue;
                };
                let Some(item_type) = item.get("type").and_then(Value::as_str) else {
                    continue;
                };
                if item_type == "agent_message" {
                    continue;
                }
                *tool_counts.entry(item_type.to_string()).or_insert(0) += 1;

                if item_type == "command_execution" {
                    if let Some(command) = item.get("command").and_then(Value::as_str) {
                        for path in observed_read_paths(command, &canonical_repo_dir) {
                            *observed_read_files.entry(path).or_insert(0) += 1;
                        }
                    }
                } else if item_type == "file_change" {
                    if let Some(changes) = item.get("changes").and_then(Value::as_array) {
                        for change in changes {
                            let Some(path) = change.get("path").and_then(Value::as_str) else {
                                continue;
                            };
                            if let Some(normalized) =
                                normalize_repo_file(Path::new(path), &canonical_repo_dir)
                            {
                                *observed_edit_files.entry(normalized).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ParsedRunnerTelemetry {
        usage,
        tool_counts: tool_counts
            .into_iter()
            .map(|(tool_name, count)| TrajectoryToolCount {
                tool_name,
                count,
                source: TrajectoryDataSource::CodexEventLog,
            })
            .collect(),
        observed_read_files: observed_read_files
            .into_iter()
            .map(|(path, count)| TrajectoryFileObservation {
                path,
                count,
                source: TrajectoryDataSource::CodexEventLog,
            })
            .collect(),
        observed_edit_files: observed_edit_files
            .into_iter()
            .map(|(path, count)| TrajectoryFileObservation {
                path,
                count,
                source: TrajectoryDataSource::CodexEventLog,
            })
            .collect(),
    }
}

fn combine_tokens(input_tokens: Option<u64>, output_tokens: Option<u64>) -> Option<u64> {
    match (input_tokens, output_tokens) {
        (Some(input_tokens), Some(output_tokens)) => Some(input_tokens + output_tokens),
        _ => None,
    }
}

fn observed_read_paths(command: &str, repo_dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    for token in command.split(is_command_separator) {
        let trimmed = token.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        let candidate = Path::new(trimmed);
        let Some(normalized) = normalize_repo_file(candidate, repo_dir) else {
            continue;
        };
        if !paths.contains(&normalized) {
            paths.push(normalized);
        }
    }
    paths
}

fn is_command_separator(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ';' | '|' | '&' | '<' | '>'
                | '=' | ','
        )
}

fn normalize_repo_file(path: &Path, repo_dir: &Path) -> Option<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_dir.join(path)
    };
    if !absolute.is_file() {
        return None;
    }
    let canonical = fs::canonicalize(&absolute).ok()?;
    let relative = canonical.strip_prefix(repo_dir).ok()?;
    Some(relative.to_string_lossy().into_owned())
}

fn collect_changed_files(repo_dir: &Path) -> Result<Vec<TrajectoryFileObservation>, AthenaError> {
    let output = run_command(
        &[
            "git".into(),
            "diff".into(),
            "--name-only".into(),
            "--relative".into(),
        ],
        repo_dir,
        None,
    )?;
    let mut changed_files = Vec::new();
    for path in output.stdout.lines().map(str::trim).filter(|line| !line.is_empty()) {
        changed_files.push(TrajectoryFileObservation {
            path: path.to_string(),
            count: 1,
            source: TrajectoryDataSource::GitDiff,
        });
    }
    Ok(changed_files)
}

fn git_diff_stat(repo_dir: &Path) -> Result<String, AthenaError> {
    Ok(run_command(
        &[
            "git".into(),
            "diff".into(),
            "--stat".into(),
            "--relative".into(),
        ],
        repo_dir,
        None,
    )?
    .stdout)
}

fn summarize_failure(
    primary_output: &str,
    fallback_output: &str,
    primary_source: TrajectoryDataSource,
    fallback_source: TrajectoryDataSource,
) -> Option<TrajectoryFailureDescription> {
    summarize_failure_text(primary_output).map(|text| TrajectoryFailureDescription {
        text,
        source: primary_source,
    })
    .or_else(|| {
        summarize_failure_text(fallback_output).map(|text| TrajectoryFailureDescription {
            text,
            source: fallback_source,
        })
    })
}

fn summarize_failure_text(output: &str) -> Option<String> {
    for line in output.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.contains("ERROR")
            || line.contains("Error")
            || line.contains("FAILED")
            || line.contains("failed")
            || line.contains("fatal")
            || line.contains("Traceback")
        {
            return Some(truncate_text(line, 240));
        }
    }

    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| truncate_text(line, 240))
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for ch in text.chars().take(max_chars) {
        truncated.push(ch);
    }
    truncated
}

fn parse_test_statuses(output: &str, parser: &TrajectoryParserKind) -> BTreeMap<String, String> {
    match parser {
        TrajectoryParserKind::Pytest => parse_pytest_statuses(output),
        TrajectoryParserKind::Unittest => parse_unittest_statuses(output),
    }
}

fn parse_pytest_statuses(output: &str) -> BTreeMap<String, String> {
    let mut statuses = BTreeMap::new();
    for line in output.lines() {
        let trimmed = line.trim();
        for status in ["PASSED", "FAILED", "SKIPPED", "XFAIL", "ERROR"] {
            let marker = format!(" {status}");
            if let Some(idx) = trimmed.rfind(&marker) {
                let name = trimmed[..idx].trim();
                if name.contains("::") {
                    statuses.insert(name.to_string(), status.to_string());
                }
            }
        }
    }
    statuses
}

fn parse_unittest_statuses(output: &str) -> BTreeMap<String, String> {
    let mut statuses = BTreeMap::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((name, status)) = trimmed.split_once(" ... ") {
            let normalized = match status {
                "ok" => Some("PASSED"),
                "FAIL" => Some("FAILED"),
                "ERROR" => Some("ERROR"),
                _ if status.starts_with("skipped") => Some("SKIPPED"),
                _ => None,
            };
            if let Some(normalized) = normalized {
                statuses.insert(name.to_string(), normalized.to_string());
            }
        }
    }
    statuses
}

fn success_rate(expected: &[String], statuses: &BTreeMap<String, String>) -> f64 {
    if expected.is_empty() {
        return 1.0;
    }
    let success = expected
        .iter()
        .filter(|test_case| {
            matches!(
                statuses.get(*test_case).map(String::as_str),
                Some("PASSED") | Some("XFAIL")
            )
        })
        .count();
    safe_rate(success, expected.len())
}

fn safe_rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), AthenaError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
