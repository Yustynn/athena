use crate::error::AthenaError;
use crate::feedback::{FeedbackEvent, FragmentScores, apply_feedback, validate_feedback};
use crate::fragment::load_fragments;
use crate::orientation::{CorrectionPacket, OrientationResponse, check_orientation};
use crate::packet::{PurposePacket, assemble_packet_with_scores};
use crate::purpose::{Purpose, PurposeStatus};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AthenaRequest {
    AssemblePacket {
        prompt: String,
        success_criteria: String,
    },
    CheckOrientation {
        purpose: Purpose,
        packet: PurposePacket,
        response: OrientationResponse,
    },
    ApplyFeedback {
        purpose: Purpose,
        packet: PurposePacket,
        feedback: FeedbackEvent,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AthenaResponse {
    PacketAssembly {
        purpose: Purpose,
        packet: PurposePacket,
    },
    OrientationCheck {
        correction: Option<CorrectionPacket>,
    },
    FeedbackApplication {
        feedback: FeedbackEvent,
        fragment_scores: FragmentScores,
        next_packet: PurposePacket,
    },
}

pub fn handle_request(
    request: AthenaRequest,
    fixture_path: impl AsRef<Path>,
) -> Result<AthenaResponse, AthenaError> {
    match request {
        AthenaRequest::AssemblePacket {
            prompt,
            success_criteria,
        } => {
            let purpose = Purpose {
                purpose_id: "purpose-1".into(),
                statement: prompt,
                success_criteria,
                status: PurposeStatus::Active,
            };
            let fragments = load_fragments(fixture_path)?;
            let packet = assemble_packet_with_scores(&purpose, &fragments, &FragmentScores::new())?;
            Ok(AthenaResponse::PacketAssembly { purpose, packet })
        }
        AthenaRequest::CheckOrientation {
            purpose,
            packet,
            response,
        } => Ok(AthenaResponse::OrientationCheck {
            correction: check_orientation(&purpose, &packet, &response),
        }),
        AthenaRequest::ApplyFeedback {
            purpose,
            packet,
            feedback,
        } => {
            validate_feedback(&packet, &feedback)?;
            let fragments = load_fragments(fixture_path)?;
            let mut fragment_scores = FragmentScores::new();
            apply_feedback(&mut fragment_scores, &feedback);
            let next_packet = assemble_packet_with_scores(&purpose, &fragments, &fragment_scores)?;
            Ok(AthenaResponse::FeedbackApplication {
                feedback,
                fragment_scores,
                next_packet,
            })
        }
    }
}
