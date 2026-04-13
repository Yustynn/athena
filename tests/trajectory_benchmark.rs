use athena_v2::benchmark::{TrajectoryDataSource, run_trajectory_benchmark};
use athena_v2::storage::DoltStorage;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn benchmark_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trajectory/benchmark_spec.json")
}

fn jinja_blind_fragments_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benchmarks/trajectory/jinja/blind_fragments.json")
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
    assert_eq!(
        first.usage.as_ref().unwrap().source,
        TrajectoryDataSource::CodexEventLog
    );
    assert_eq!(first.tool_counts.len(), 2);
    assert!(
        first
            .tool_counts
            .iter()
            .any(|item| item.tool_name == "command_execution" && item.count == 1)
    );
    assert!(
        first
            .tool_counts
            .iter()
            .any(|item| item.tool_name == "file_change" && item.count == 1)
    );
    assert_eq!(first.observed_read_files.len(), 2);
    assert!(first.observed_read_files.iter().any(
        |item| item.path == "cachelib.py" && item.source == TrajectoryDataSource::CodexEventLog
    ));
    assert!(
        first
            .observed_read_files
            .iter()
            .any(|item| item.path == "tests/test_public.py"
                && item.source == TrajectoryDataSource::CodexEventLog)
    );
    assert_eq!(first.observed_edit_files.len(), 1);
    assert_eq!(first.observed_edit_files[0].path, "cachelib.py");
    assert_eq!(
        first.observed_edit_files[0].source,
        TrajectoryDataSource::CodexEventLog
    );
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
    assert!(
        third
            .tool_counts
            .iter()
            .any(|item| item.tool_name == "file_change" && item.count == 1)
    );
    assert!(
        third
            .observed_read_files
            .iter()
            .any(|item| item.path == "cachelib.py")
    );
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

#[test]
fn trajectory_benchmark_preseed_mode_seeds_benchmark_local_athena_storage() {
    let report = run_trajectory_benchmark(benchmark_spec_path(), "preseed", true).unwrap();
    let run_root = PathBuf::from(report.kept_run_root.as_ref().unwrap());
    let repo_dir = run_root.join("repo");
    let seed_root = run_root.join("athena-preseed");
    let db_path = seed_root.join("db");
    let env_path = repo_dir.join(".trajectory_runner_athena_env.json");

    let storage = DoltStorage::open(&db_path).unwrap();
    let fragments = storage.list_fragment_nodes().unwrap();
    let fragment_ids: Vec<&str> = fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();
    assert_eq!(fragment_ids, vec!["fixture_blind_001", "fixture_blind_002"]);

    let env_json: Value = serde_json::from_str(&fs::read_to_string(&env_path).unwrap()).unwrap();
    assert_eq!(
        env_json["athena_db_path"].as_str().unwrap(),
        db_path.to_str().unwrap()
    );
    assert_eq!(
        env_json["athena_dolt_home"].as_str().unwrap(),
        seed_root.join(".dolt-home-db").to_str().unwrap()
    );

    fs::remove_dir_all(run_root).unwrap();
}

#[test]
fn trajectory_benchmark_current_mode_stays_distinct_from_preseed_mode() {
    let report = run_trajectory_benchmark(benchmark_spec_path(), "current", true).unwrap();
    let run_root = PathBuf::from(report.kept_run_root.as_ref().unwrap());
    let repo_dir = run_root.join("repo");

    assert!(!run_root.join("athena-preseed").exists());
    assert!(!repo_dir.join(".trajectory_runner_athena_env.json").exists());
    assert!(repo_dir.join(".codex/hooks.json").exists());

    fs::remove_dir_all(run_root).unwrap();
}

#[test]
fn trajectory_benchmark_preseed_fragments_stay_blind_and_repo_sourced() {
    let spec: Value = serde_json::from_slice(&fs::read(benchmark_spec_path()).unwrap()).unwrap();
    let preseed = &spec["athena_preseed"];
    assert_eq!(preseed["source"]["kind"], "benchmark_clone_repo");

    let fixture_root = benchmark_spec_path().parent().unwrap().join("repo");
    for path in preseed["source"]["repo_paths"].as_array().unwrap() {
        assert!(fixture_root.join(path.as_str().unwrap()).exists());
    }

    let raw = fs::read_to_string(jinja_blind_fragments_path()).unwrap();
    for forbidden in [
        "zero_capacity",
        "zero-capacity",
        "step1",
        "step2",
        "step3",
        "peek(",
        "pop(",
        "test_hidden",
    ] {
        assert!(
            !raw.contains(forbidden),
            "blind fragments leaked {forbidden}"
        );
    }
}
