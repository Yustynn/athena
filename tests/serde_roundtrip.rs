use athena_v2::feedback::{FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome};
use athena_v2::fragment::{Fragment, FragmentKind, FragmentState};
use athena_v2::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use athena_v2::packet::PurposePacket;
use athena_v2::purpose::{Purpose, PurposeStatus};

#[test]
fn purpose_round_trips() {
    let value = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Build tracer".into(),
        success_criteria: "Tracer passes".into(),
        status: PurposeStatus::Active,
    };

    let json = serde_json::to_string(&value).unwrap();
    let round_trip: Purpose = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip, value);
}

#[test]
fn fragment_round_trips() {
    let mut value = Fragment::basic(
        "f1",
        FragmentKind::Doctrine,
        "Keep runtime deterministic.",
        "Keep runtime deterministic. Favor stable outputs from identical inputs.",
    );
    value.scope = Some("packet assembly".into());
    value.trigger_conditions = vec!["deterministic".into()];
    value.state = FragmentState::Durable;
    value.concept_key = Some("determinism".into());
    value.usefulness_score = 2;
    value.correctness_confidence = 1;
    value.durability_score = 3;
    value.stale_after = Some("2099-01-01".into());
    value.supersedes = vec![FragmentId::new("f0")];

    let json = serde_json::to_string(&value).unwrap();
    let round_trip: Fragment = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip, value);
}

#[test]
fn packet_round_trips() {
    let value = PurposePacket {
        packet_id: PacketId::new("packet-1"),
        purpose_id: PurposeId::new("purpose-1"),
        fragments: vec![Fragment::basic(
            "f1",
            FragmentKind::Doctrine,
            "Keep runtime deterministic.",
            "Keep runtime deterministic. Favor stable outputs from identical inputs.",
        )],
    };

    let json = serde_json::to_string(&value).unwrap();
    let round_trip: PurposePacket = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip, value);
}

#[test]
fn feedback_round_trips() {
    let value = FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        outcome: TaskOutcome::Success,
        fragment_feedback: vec![FragmentFeedback {
            fragment_id: FragmentId::new("f1"),
            verdict: FragmentVerdict::Helped,
            reason: Some("worked".into()),
        }],
    };

    let json = serde_json::to_string(&value).unwrap();
    let round_trip: FeedbackEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip, value);
}

#[test]
fn invalid_fragment_kind_fails() {
    let json = r#"{"fragment_id":"f1","kind":"bad_kind","summary":"nope","full_text":"nope"}"#;
    let parsed = serde_json::from_str::<Fragment>(json);
    assert!(parsed.is_err());
}

#[test]
fn legacy_text_fragment_still_loads() {
    let json = r#"{"fragment_id":"f1","kind":"doctrine","text":"legacy body"}"#;
    let parsed = serde_json::from_str::<Fragment>(json).unwrap();
    assert_eq!(parsed.summary, "legacy body");
    assert_eq!(parsed.full_text, "legacy body");
    assert_eq!(parsed.state, FragmentState::Durable);
    assert!(parsed.trigger_conditions.is_empty());
}
