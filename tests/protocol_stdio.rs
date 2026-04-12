use athena_v2::feedback::{FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome};
use athena_v2::ids::{FeedbackId, PacketId, PurposeId};
use athena_v2::orientation::OrientationResponse;
use athena_v2::protocol::{AthenaRequest, AthenaResponse, handle_request};
use athena_v2::purpose::{Purpose, PurposeStatus};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

#[test]
fn assemble_packet_request_returns_expected_fragments() {
    let response = handle_request(
        AthenaRequest::AssemblePacket {
            prompt: "Assemble packet and collect feedback".into(),
            success_criteria: "Packet feedback invariant holds".into(),
        },
        fixture_path(),
    )
    .unwrap();

    let AthenaResponse::PacketAssembly { purpose, packet } = response else {
        panic!("expected packet assembly response");
    };

    let fragment_ids: Vec<&str> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(purpose.statement, "Assemble packet and collect feedback");
    assert_eq!(fragment_ids, vec!["f1", "f2", "f3"]);
}

#[test]
fn apply_feedback_request_returns_changed_next_packet() {
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Assemble packet and collect feedback".into(),
        success_criteria: "Packet feedback invariant holds".into(),
        status: PurposeStatus::Active,
    };

    let packet = match handle_request(
        AthenaRequest::AssemblePacket {
            prompt: purpose.statement.clone(),
            success_criteria: purpose.success_criteria.clone(),
        },
        fixture_path(),
    )
    .unwrap()
    {
        AthenaResponse::PacketAssembly { packet, .. } => packet,
        other => panic!("unexpected response: {other:?}"),
    };

    let feedback = FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: purpose.purpose_id.clone(),
        packet_id: PacketId::new("packet-1"),
        outcome: TaskOutcome::Partial,
        fragment_feedback: vec![
            FragmentFeedback {
                fragment_id: "f1".into(),
                verdict: FragmentVerdict::Helped,
                reason: Some("still useful".into()),
            },
            FragmentFeedback {
                fragment_id: "f2".into(),
                verdict: FragmentVerdict::Wrong,
                reason: Some("bad fit".into()),
            },
            FragmentFeedback {
                fragment_id: "f3".into(),
                verdict: FragmentVerdict::Helped,
                reason: Some("keep".into()),
            },
        ],
    };

    let response = handle_request(
        AthenaRequest::ApplyFeedback {
            purpose,
            packet,
            feedback,
        },
        fixture_path(),
    )
    .unwrap();

    let AthenaResponse::FeedbackApplication {
        fragment_scores,
        next_packet,
        ..
    } = response
    else {
        panic!("expected feedback application response");
    };

    let fragment_ids: Vec<&str> = next_packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(fragment_scores.get("f2"), Some(&-2));
    assert_eq!(fragment_ids, vec!["f1", "f3", "f4"]);
}

#[test]
fn orientation_request_returns_correction_when_constraint_missing() {
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Build tracer".into(),
        success_criteria: "collect feedback and preserve invariants".into(),
        status: PurposeStatus::Active,
    };

    let packet = match handle_request(
        AthenaRequest::AssemblePacket {
            prompt: purpose.statement.clone(),
            success_criteria: purpose.success_criteria.clone(),
        },
        fixture_path(),
    )
    .unwrap()
    {
        AthenaResponse::PacketAssembly { packet, .. } => packet,
        other => panic!("unexpected response: {other:?}"),
    };

    let response = handle_request(
        AthenaRequest::CheckOrientation {
            purpose,
            packet,
            response: OrientationResponse {
                purpose_id: PurposeId::new("purpose-1"),
                packet_id: PacketId::new("packet-1"),
                best_path: "Collect feedback first".into(),
                addressed_constraints: vec!["collect feedback".into()],
                unresolved_questions: vec!["Did we check invariants?".into()],
            },
        },
        fixture_path(),
    )
    .unwrap();

    let AthenaResponse::OrientationCheck { correction } = response else {
        panic!("expected orientation check response");
    };

    assert_eq!(
        correction.unwrap().missing_constraints,
        vec!["preserve invariants"]
    );
}
