use athena_v2::error::AthenaError;
use athena_v2::feedback::{
    FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome, validate_feedback,
};
use athena_v2::fragment::{Fragment, FragmentKind};
use athena_v2::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use athena_v2::packet::PurposePacket;

fn packet() -> PurposePacket {
    PurposePacket {
        packet_id: PacketId::new("packet-1"),
        purpose_id: PurposeId::new("purpose-1"),
        fragments: vec![
            Fragment {
                fragment_id: FragmentId::new("f1"),
                kind: FragmentKind::Doctrine,
                text: "Keep runtime deterministic.".into(),
            },
            Fragment {
                fragment_id: FragmentId::new("f2"),
                kind: FragmentKind::Pitfall,
                text: "Do not skip fragment feedback.".into(),
            },
        ],
    }
}

fn feedback(fragment_ids: &[&str]) -> FeedbackEvent {
    FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        outcome: TaskOutcome::Success,
        fragment_feedback: fragment_ids
            .iter()
            .map(|fragment_id| FragmentFeedback {
                fragment_id: FragmentId::new(*fragment_id),
                verdict: FragmentVerdict::Helped,
                reason: None,
            })
            .collect(),
    }
}

#[test]
fn exhaustive_feedback_passes() {
    let result = validate_feedback(&packet(), &feedback(&["f1", "f2"]));
    assert!(result.is_ok());
}

#[test]
fn missing_fragment_feedback_fails() {
    let result = validate_feedback(&packet(), &feedback(&["f1"]));
    match result {
        Err(AthenaError::MissingFragmentFeedback(missing)) => {
            assert_eq!(missing, vec!["f2".to_string()]);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn extra_fragment_feedback_fails() {
    let result = validate_feedback(&packet(), &feedback(&["f1", "f2", "f3"]));
    match result {
        Err(AthenaError::ExtraFragmentFeedback(extra)) => {
            assert_eq!(extra, vec!["f3".to_string()]);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}
