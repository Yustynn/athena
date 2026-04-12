use crate::error::AthenaError;
use crate::feedback::{
    FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome, validate_feedback,
};
use crate::fragment::load_fragments;
use crate::ids::{FeedbackId, PurposeId};
use crate::packet::{PurposePacket, assemble_packet};
use crate::purpose::{Purpose, PurposeStatus};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracerResult {
    pub purpose: Purpose,
    pub packet: PurposePacket,
    pub feedback: FeedbackEvent,
}

pub fn run_tracer(
    prompt: &str,
    success_criteria: &str,
    fixture_path: impl AsRef<Path>,
) -> Result<TracerResult, AthenaError> {
    let mut purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: prompt.to_owned(),
        success_criteria: success_criteria.to_owned(),
        status: PurposeStatus::Active,
    };

    let fragments = load_fragments(fixture_path)?;
    let packet = assemble_packet(&purpose, &fragments)?;

    let feedback = FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: purpose.purpose_id.clone(),
        packet_id: packet.packet_id.clone(),
        outcome: TaskOutcome::Success,
        fragment_feedback: packet
            .fragments
            .iter()
            .map(|fragment| FragmentFeedback {
                fragment_id: fragment.fragment_id.clone(),
                verdict: FragmentVerdict::Helped,
                reason: Some("tracer happy path".to_owned()),
            })
            .collect(),
    };

    validate_feedback(&packet, &feedback)?;
    purpose.status = PurposeStatus::Completed;

    Ok(TracerResult {
        purpose,
        packet,
        feedback,
    })
}
