use athena_v2::purpose::PurposeStatus;
use athena_v2::tracer::run_tracer;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

#[test]
fn tracer_happy_path_runs_end_to_end() {
    let result = run_tracer(
        "Assemble packet and collect feedback",
        "Packet feedback invariant holds",
        fixture_path(),
    )
    .unwrap();

    assert_eq!(result.purpose.purpose_id, result.packet.purpose_id);
    assert_eq!(result.purpose.purpose_id, result.feedback.purpose_id);
    assert_eq!(result.packet.packet_id, result.feedback.packet_id);
    assert_eq!(result.purpose.status, PurposeStatus::Completed);
    assert_eq!(result.packet.fragments.len(), 3);
    assert_eq!(result.feedback.fragment_feedback.len(), 3);
}
