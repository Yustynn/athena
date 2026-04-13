use athena_v2::benchmark::run_retrieval_benchmark;
use std::path::PathBuf;

fn benchmark_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/retrieval/benchmark_spec.json")
}

#[test]
fn retrieval_benchmark_runs_with_expected_shape() {
    let report = run_retrieval_benchmark(benchmark_spec_path()).unwrap();

    assert_eq!(report.overall.task_count, 16);
    assert_eq!(report.corpuses.len(), 4);
    assert_eq!(report.by_family["direct_trigger"].task_count, 4);
    assert_eq!(report.by_family["multi_constraint"].task_count, 4);
    assert_eq!(report.by_family["similar_choice"].task_count, 4);
    assert_eq!(report.by_family["frontier_replacement"].task_count, 4);
}

#[test]
fn retrieval_benchmark_filters_superseded_frontier_fragments() {
    let report = run_retrieval_benchmark(benchmark_spec_path()).unwrap();
    let task = report
        .task_results
        .iter()
        .find(|result| result.task_id == "fr_01")
        .unwrap();

    assert_eq!(task.ranked_fragment_ids[0], "gf_002");
    assert!(!task.ranked_fragment_ids.iter().any(|id| id == "gf_001"));
}

#[test]
fn retrieval_benchmark_disambiguates_similar_choices_when_metadata_matches() {
    let report = run_retrieval_benchmark(benchmark_spec_path()).unwrap();
    let task = report
        .task_results
        .iter()
        .find(|result| result.task_id == "sc_01")
        .unwrap();

    assert_eq!(task.ranked_fragment_ids[0], "as_001");
    assert_eq!(task.hit_at["1"], true);
}
