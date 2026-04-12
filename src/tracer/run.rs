use crate::error::AthenaError;
use crate::feedback::{
    FeedbackEvent, FragmentFeedback, FragmentScores, FragmentVerdict, TaskOutcome, apply_feedback,
    validate_feedback,
};
use crate::fragment::load_fragments;
use crate::ids::{FeedbackId, PurposeId};
use crate::orientation::{OrientationResponse, check_orientation};
use crate::packet::{PurposePacket, assemble_packet, assemble_packet_with_scores};
use crate::purpose::{Purpose, PurposeStatus};
use crate::storage::SqliteStorage;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracerResult {
    pub purpose: Purpose,
    pub packet: PurposePacket,
    pub feedback: FeedbackEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationLoopResult {
    pub first_response: OrientationResponse,
    pub correction_applied: bool,
    pub second_response: OrientationResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackLoopResult {
    pub first_packet: PurposePacket,
    pub feedback: FeedbackEvent,
    pub fragment_scores: FragmentScores,
    pub second_packet: PurposePacket,
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

pub fn run_tracer_persisted(
    prompt: &str,
    success_criteria: &str,
    fixture_path: impl AsRef<Path>,
    db_path: impl AsRef<Path>,
) -> Result<TracerResult, AthenaError> {
    let result = run_tracer(prompt, success_criteria, fixture_path)?;
    let storage = SqliteStorage::open(db_path)?;
    storage.insert_purpose(&result.purpose)?;
    storage.insert_packet(&result.packet)?;
    storage.insert_feedback(&result.feedback)?;
    Ok(result)
}

pub fn run_feedback_loop(
    prompt: &str,
    success_criteria: &str,
    fixture_path: impl AsRef<Path>,
) -> Result<FeedbackLoopResult, AthenaError> {
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: prompt.to_owned(),
        success_criteria: success_criteria.to_owned(),
        status: PurposeStatus::Active,
    };

    let fragments = load_fragments(fixture_path)?;
    let first_packet = assemble_packet(&purpose, &fragments)?;

    let feedback = FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-loop-1"),
        purpose_id: purpose.purpose_id.clone(),
        packet_id: first_packet.packet_id.clone(),
        outcome: TaskOutcome::Partial,
        fragment_feedback: first_packet
            .fragments
            .iter()
            .enumerate()
            .map(|(idx, fragment)| FragmentFeedback {
                fragment_id: fragment.fragment_id.clone(),
                verdict: if idx == 1 {
                    FragmentVerdict::Wrong
                } else {
                    FragmentVerdict::Helped
                },
                reason: Some("dogfood feedback loop".to_owned()),
            })
            .collect(),
    };

    validate_feedback(&first_packet, &feedback)?;

    let mut fragment_scores = FragmentScores::new();
    apply_feedback(&mut fragment_scores, &feedback);

    let second_packet = assemble_packet_with_scores(&purpose, &fragments, &fragment_scores)?;

    Ok(FeedbackLoopResult {
        first_packet,
        feedback,
        fragment_scores,
        second_packet,
    })
}

pub fn run_orientation_loop(
    prompt: &str,
    success_criteria: &str,
    fixture_path: impl AsRef<Path>,
) -> Result<OrientationLoopResult, AthenaError> {
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: prompt.to_owned(),
        success_criteria: success_criteria.to_owned(),
        status: PurposeStatus::Active,
    };

    let fragments = load_fragments(fixture_path)?;
    let packet = assemble_packet(&purpose, &fragments)?;

    let first_response = OrientationResponse {
        purpose_id: purpose.purpose_id.clone(),
        packet_id: packet.packet_id.clone(),
        best_path: "Draft plan quickly".to_owned(),
        addressed_constraints: vec![],
        unresolved_questions: vec!["Did we satisfy all constraints?".to_owned()],
    };

    let correction = check_orientation(&purpose, &packet, &first_response);

    let correction_applied = correction.is_some();

    let second_response = if let Some(correction) = correction {
        let coverage = correction
            .missing_constraints
            .iter()
            .map(|constraint| format!("include {constraint}"))
            .collect::<Vec<_>>()
            .join(", ");
        OrientationResponse {
            purpose_id: purpose.purpose_id.clone(),
            packet_id: packet.packet_id.clone(),
            best_path: format!("Draft plan, then {coverage}"),
            addressed_constraints: correction.missing_constraints.clone(),
            unresolved_questions: Vec::new(),
        }
    } else {
        first_response.clone()
    };

    Ok(OrientationLoopResult {
        first_response,
        correction_applied,
        second_response,
    })
}
