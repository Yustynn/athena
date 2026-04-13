use athena_v2::benchmark::{TrajectoryDataSource, run_trajectory_benchmark};
use std::fs;
use std::path::PathBuf;

fn benchmark_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trajectory/benchmark_spec.json")
}

#[test]
fn trajectory_benchmark_runs_end_to_end_with_hidden_oracle() {
    let report = run_trajectory_benchmark(benchmark_spec_path(), "off", false).unwrap();

    assert_eq!(report.overall.step_count, 3);
    assert_eq!(report.overall.resolved_count, 3);
    assert_eq!(report.step_results[0].step_id, "step1_zero_capacity");
    assert_eq!(report.step_results[1].step_id, "step2_peek");
    assert_eq!(report.step_results[2].step_id, "step3_pop");
    assert!(report.step_results.iter().all(|step| step.resolved));

    let first = &report.step_results[0];
    assert_eq!(first.usage.as_ref().unwrap().input_tokens, Some(101));
    assert_eq!(first.usage.as_ref().unwrap().cached_input_tokens, Some(41));
    assert_eq!(first.usage.as_ref().unwrap().output_tokens, Some(11));
    assert_eq!(first.usage.as_ref().unwrap().total_tokens, Some(112));
    assert_eq!(first.usage.as_ref().unwrap().source, TrajectoryDataSource::CodexEventLog);
    assert_eq!(first.tool_counts.len(), 2);
    assert!(first
        .tool_counts
        .iter()
        .any(|item| item.tool_name == "command_execution" && item.count == 1));
    assert!(first
        .tool_counts
        .iter()
        .any(|item| item.tool_name == "file_change" && item.count == 1));
    assert_eq!(first.observed_read_files.len(), 2);
    assert!(first
        .observed_read_files
        .iter()
        .any(|item| item.path == "cachelib.py" && item.source == TrajectoryDataSource::CodexEventLog));
    assert!(first
        .observed_read_files
        .iter()
        .any(|item| item.path == "tests/test_public.py" && item.source == TrajectoryDataSource::CodexEventLog));
    assert_eq!(first.observed_edit_files.len(), 1);
    assert_eq!(first.observed_edit_files[0].path, "cachelib.py");
    assert_eq!(first.observed_edit_files[0].source, TrajectoryDataSource::CodexEventLog);
    assert_eq!(first.changed_files.len(), 1);
    assert_eq!(first.changed_files[0].path, "cachelib.py");
    assert_eq!(first.changed_files[0].source, TrajectoryDataSource::GitDiff);
    assert!(first.failure_description.is_none());
}

#[test]
fn trajectory_benchmark_preserves_pass_to_pass_checks_between_steps() {
    let report = run_trajectory_benchmark(benchmark_spec_path(), "current", false).unwrap();
    let third = &report.step_results[2];

    assert_eq!(third.fail_to_pass_rate, 1.0);
    assert_eq!(third.pass_to_pass_rate, 1.0);
    assert!(third.tests_status.contains_key(
        "test_items_promote_recency (tests.test_public.PublicTests.test_items_promote_recency)"
    ));
    assert_eq!(third.usage.as_ref().unwrap().input_tokens, Some(103));
    assert!(third
        .tool_counts
        .iter()
        .any(|item| item.tool_name == "file_change" && item.count == 1));
    assert!(third
        .observed_read_files
        .iter()
        .any(|item| item.path == "cachelib.py"));
}

#[test]
fn trajectory_benchmark_current_mode_writes_athena_session_start_hook() {
    let report = run_trajectory_benchmark(benchmark_spec_path(), "current", true).unwrap();
    let run_root = PathBuf::from(report.kept_run_root.as_ref().unwrap());
    let repo_dir = run_root.join("repo");
    let hooks_json_path = repo_dir.join(".codex/hooks.json");
    let hook_script_path = repo_dir.join(".codex/hooks/session_start_athena_prime.sh");

    let hooks_json = fs::read_to_string(&hooks_json_path).unwrap();
    assert!(hooks_json.contains("\"SessionStart\""));
    assert!(hooks_json.contains("session_start_athena_prime.sh"));

    let hook_script = fs::read_to_string(&hook_script_path).unwrap();
    assert!(hook_script.contains("hookEventName\":\"SessionStart"));
    assert!(hook_script.contains(&format!(
        "source_repo_root='{}'",
        env!("CARGO_MANIFEST_DIR")
    )));
    assert!(hook_script.contains("\"$source_repo_root/scripts/athena\" prime"));

    fs::remove_dir_all(run_root).unwrap();
}
