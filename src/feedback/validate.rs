use crate::error::AthenaError;
use crate::feedback::FeedbackEvent;
use crate::packet::PurposePacket;
use std::collections::BTreeSet;

pub fn validate_feedback(
    packet: &PurposePacket,
    feedback: &FeedbackEvent,
) -> Result<(), AthenaError> {
    let packet_ids: BTreeSet<String> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.clone())
        .collect();
    let feedback_ids: BTreeSet<String> = feedback
        .fragment_feedback
        .iter()
        .map(|fragment_feedback| fragment_feedback.fragment_id.0.clone())
        .collect();

    let missing: Vec<String> = packet_ids.difference(&feedback_ids).cloned().collect();
    if !missing.is_empty() {
        return Err(AthenaError::MissingFragmentFeedback(missing));
    }

    let extra: Vec<String> = feedback_ids.difference(&packet_ids).cloned().collect();
    if !extra.is_empty() {
        return Err(AthenaError::ExtraFragmentFeedback(extra));
    }

    Ok(())
}
