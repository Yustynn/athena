use athena_v2::fragment::load_fragments;
use athena_v2::ids::PurposeId;
use athena_v2::packet::assemble_packet;
use athena_v2::purpose::{Purpose, PurposeStatus};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

#[test]
fn packet_assembly_is_deterministic() {
    let fragments = load_fragments(fixture_path()).unwrap();
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Need feedback-safe tracer".into(),
        success_criteria: "Feedback loop finishes".into(),
        status: PurposeStatus::Active,
    };

    let packet = assemble_packet(&purpose, &fragments).unwrap();
    let fragment_ids: Vec<&str> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(packet.purpose_id, purpose.purpose_id);
    assert_eq!(fragment_ids, vec!["f1", "f2", "f3"]);
}
