use athena_v2::benchmark::run_creation_benchmark;
use std::path::PathBuf;

fn benchmark_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/creation/benchmark_spec.json")
}

fn baseline_proposals_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/creation/proposals/baseline.json")
}

#[test]
fn creation_benchmark_runs_with_expected_shape() {
    let report = run_creation_benchmark(benchmark_spec_path(), baseline_proposals_path()).unwrap();

    assert_eq!(report.overall.case_count, 3);
    assert_eq!(report.by_family["core"].case_count, 3);
    assert_eq!(report.by_difficulty["easy"].case_count, 2);
    assert_eq!(report.by_difficulty["medium"].case_count, 1);
}

#[test]
fn creation_benchmark_rewards_good_single_fragment_capture() {
    let report = run_creation_benchmark(benchmark_spec_path(), baseline_proposals_path()).unwrap();
    let case = report
        .case_results
        .iter()
        .find(|result| result.case_id == "cr_01")
        .unwrap();

    assert!(case.decision_correct);
    assert!(case.count_ok);
    assert_eq!(case.kind_match, Some(true));
    assert_eq!(case.required_recall, 1.0);
    assert!(case.forbidden_clean);
    assert_eq!(case.score, 1.0);
}

#[test]
fn creation_benchmark_penalizes_wrong_kind_and_missing_specificity() {
    let report = run_creation_benchmark(benchmark_spec_path(), baseline_proposals_path()).unwrap();
    let case = report
        .case_results
        .iter()
        .find(|result| result.case_id == "cr_03")
        .unwrap();

    assert_eq!(case.kind_match, Some(false));
    assert_eq!(case.required_recall, 0.5);
    assert_eq!(case.hit_forbidden_concepts, vec!["generic-canonical-docs"]);
    assert!(case.score < 0.7);
}
