use athena_v2::ids::{PacketId, PurposeId};
use athena_v2::orientation::{OrientationResponse, check_orientation};
use athena_v2::packet::assemble_packet;
use athena_v2::purpose::{Purpose, PurposeStatus};
use athena_v2::tracer::run_orientation_loop;
use athena_v2::{fragment::load_fragments, packet::PurposePacket};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

fn purpose_with(success_criteria: &str) -> Purpose {
    Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Build tracer".into(),
        success_criteria: success_criteria.into(),
        status: PurposeStatus::Active,
    }
}

fn packet_for(purpose: &Purpose) -> PurposePacket {
    let fragments = load_fragments(fixture_path()).unwrap();
    assemble_packet(purpose, &fragments).unwrap()
}

#[test]
fn aligned_response_yields_no_correction() {
    let purpose = purpose_with("collect feedback and preserve invariants");
    let packet = packet_for(&purpose);
    let response = OrientationResponse {
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        best_path: "Collect feedback first, then preserve invariants before finish".into(),
        addressed_constraints: vec!["collect feedback".into(), "preserve invariants".into()],
        unresolved_questions: vec![],
    };

    assert!(check_orientation(&purpose, &packet, &response).is_none());
}

#[test]
fn missing_constraint_yields_correction_packet() {
    let purpose = purpose_with("collect feedback and preserve invariants");
    let packet = packet_for(&purpose);
    let response = OrientationResponse {
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        best_path: "Collect feedback first".into(),
        addressed_constraints: vec!["collect feedback".into()],
        unresolved_questions: vec!["Did we check invariants?".into()],
    };

    let correction = check_orientation(&purpose, &packet, &response).unwrap();
    assert_eq!(correction.missing_constraints, vec!["preserve invariants"]);
}

#[test]
fn correction_loop_improves_second_response() {
    let result = run_orientation_loop(
        "Assemble packet and check orientation",
        "collect feedback and preserve invariants",
        fixture_path(),
    )
    .unwrap();

    assert!(result.correction_applied);
    assert!(
        result
            .second_response
            .best_path
            .contains("collect feedback")
    );
    assert!(
        result
            .second_response
            .best_path
            .contains("preserve invariants")
    );
}
