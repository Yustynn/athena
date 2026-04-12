use athena_v2::tracer::run_feedback_loop;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

#[test]
fn wrong_feedback_penalizes_fragment_and_changes_next_packet() {
    let result = run_feedback_loop(
        "Assemble packet and collect feedback",
        "Packet feedback invariant holds",
        fixture_path(),
    )
    .unwrap();

    let first_ids: Vec<&str> = result
        .first_packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();
    let second_ids: Vec<&str> = result
        .second_packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(first_ids, vec!["f1", "f2", "f3"]);
    assert_eq!(result.fragment_scores.get("f2"), Some(&-2));
    assert_eq!(second_ids, vec!["f1", "f3", "f4"]);
}
