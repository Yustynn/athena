use athena_v2::benchmark::run_trajectory_benchmark;
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
}
