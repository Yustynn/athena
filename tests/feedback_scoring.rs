use athena_v2::feedback::{
    FeedbackEvent, FragmentFeedback, FragmentScores, FragmentVerdict, TaskOutcome, apply_feedback,
};
use athena_v2::ids::{FeedbackId, FragmentId, PacketId, PurposeId};

fn feedback(verdicts: &[(&str, FragmentVerdict)]) -> FeedbackEvent {
    FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        outcome: TaskOutcome::Partial,
        fragment_feedback: verdicts
            .iter()
            .map(|(fragment_id, verdict)| FragmentFeedback {
                fragment_id: FragmentId::new(*fragment_id),
                verdict: verdict.clone(),
                reason: None,
            })
            .collect(),
    }
}

#[test]
fn scoring_accumulates_helped_and_wrong_feedback() {
    let mut scores = FragmentScores::new();

    apply_feedback(
        &mut scores,
        &feedback(&[
            ("f1", FragmentVerdict::Helped),
            ("f2", FragmentVerdict::Wrong),
        ]),
    );
    apply_feedback(
        &mut scores,
        &feedback(&[
            ("f1", FragmentVerdict::Helped),
            ("f2", FragmentVerdict::Neutral),
        ]),
    );

    assert_eq!(scores.get("f1"), Some(&2));
    assert_eq!(scores.get("f2"), Some(&-2));
}
