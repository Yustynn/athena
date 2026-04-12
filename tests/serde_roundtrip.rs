use athena_v2::feedback::{FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome};
use athena_v2::fragment::{Fragment, FragmentKind};
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
    let value = Fragment {
        fragment_id: FragmentId::new("f1"),
        kind: FragmentKind::Doctrine,
        text: "Keep runtime deterministic.".into(),
    };

    let json = serde_json::to_string(&value).unwrap();
    let round_trip: Fragment = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip, value);
}

#[test]
fn packet_round_trips() {
    let value = PurposePacket {
        packet_id: PacketId::new("packet-1"),
        purpose_id: PurposeId::new("purpose-1"),
        fragments: vec![Fragment {
            fragment_id: FragmentId::new("f1"),
            kind: FragmentKind::Doctrine,
            text: "Keep runtime deterministic.".into(),
        }],
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
    let json = r#"{"fragment_id":"f1","kind":"bad_kind","text":"nope"}"#;
    let parsed = serde_json::from_str::<Fragment>(json);
    assert!(parsed.is_err());
}
