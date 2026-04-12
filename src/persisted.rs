use crate::error::AthenaError;
use crate::feedback::{
    FeedbackEvent, FragmentFeedback, FragmentScores, TaskOutcome, apply_feedback, validate_feedback,
};
use crate::fragment::{Fragment, FragmentKind, load_fragments};
use crate::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use crate::packet::{PurposePacket, assemble_packet_with_scores};
use crate::purpose::{Purpose, PurposeStatus};
use crate::storage::DoltStorage;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurposeCommandResult {
    pub purpose: Purpose,
    pub packet: PurposePacket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewFragmentInput {
    pub kind: FragmentKind,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub full_text: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackApplyInput {
    pub fragment_feedback: Vec<FragmentFeedback>,
    #[serde(default)]
    pub new_fragments: Vec<NewFragmentInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackApplyResult {
    pub feedback: FeedbackEvent,
    pub created_fragments: Vec<Fragment>,
    pub fragment_scores: FragmentScores,
    pub next_packet: PurposePacket,
}

impl NewFragmentInput {
    fn resolve_text(self) -> (String, String) {
        let summary = self
            .summary
            .or_else(|| self.text.clone())
            .or_else(|| self.full_text.clone())
            .unwrap_or_default();
        let full_text = self
            .full_text
            .or(self.text)
            .unwrap_or_else(|| summary.clone());
        (summary, full_text)
    }
}

pub fn create_purpose(
    storage: &DoltStorage,
    fixture_path: impl AsRef<Path>,
    statement: &str,
    success_criteria: &str,
) -> Result<PurposeCommandResult, AthenaError> {
    let purpose = Purpose {
        purpose_id: PurposeId::new(unique_id("purpose")),
        statement: statement.to_owned(),
        success_criteria: success_criteria.to_owned(),
        status: PurposeStatus::Active,
    };

    let packet = assemble_persisted_packet(storage, fixture_path, &purpose)?;
    storage.insert_purpose(&purpose)?;
    storage.insert_packet(&packet)?;
    storage.commit_all(&format!("Create Athena purpose {}", purpose.purpose_id))?;

    Ok(PurposeCommandResult { purpose, packet })
}

pub fn update_purpose(
    storage: &DoltStorage,
    fixture_path: impl AsRef<Path>,
    purpose_id: &PurposeId,
    statement: &str,
    success_criteria: &str,
) -> Result<PurposeCommandResult, AthenaError> {
    let mut purpose = storage
        .get_purpose(purpose_id)?
        .ok_or_else(|| missing("purpose", &purpose_id.0))?;
    purpose.statement = statement.to_owned();
    purpose.success_criteria = success_criteria.to_owned();
    purpose.status = PurposeStatus::Active;

    let packet = assemble_persisted_packet(storage, fixture_path, &purpose)?;
    storage.insert_purpose(&purpose)?;
    storage.insert_packet(&packet)?;
    storage.commit_all(&format!("Update Athena purpose {}", purpose.purpose_id))?;

    Ok(PurposeCommandResult { purpose, packet })
}

pub fn apply_feedback_command(
    storage: &DoltStorage,
    fixture_path: impl AsRef<Path>,
    purpose_id: &PurposeId,
    packet_id: &PacketId,
    outcome: TaskOutcome,
    input: FeedbackApplyInput,
) -> Result<FeedbackApplyResult, AthenaError> {
    let purpose = storage
        .get_purpose(purpose_id)?
        .ok_or_else(|| missing("purpose", &purpose_id.0))?;
    let packet = storage
        .get_packet(packet_id)?
        .ok_or_else(|| missing("packet", &packet_id.0))?;

    let feedback = FeedbackEvent {
        feedback_id: FeedbackId::new(unique_id("feedback")),
        purpose_id: purpose.purpose_id.clone(),
        packet_id: packet.packet_id.clone(),
        outcome,
        fragment_feedback: input.fragment_feedback,
    };
    validate_feedback(&packet, &feedback)?;

    let created_fragments = input
        .new_fragments
        .into_iter()
        .map(|fragment| {
            let kind = fragment.kind.clone();
            let (summary, full_text) = fragment.resolve_text();
            let created = Fragment {
                fragment_id: FragmentId::new(unique_id("fragment")),
                kind,
                summary,
                full_text,
            };
            storage.insert_fragment_node(
                &created.fragment_id,
                &created.kind,
                &created.summary,
                &created.full_text,
            )?;
            Ok(created)
        })
        .collect::<Result<Vec<_>, AthenaError>>()?;

    let mut fragment_scores = FragmentScores::new();
    apply_feedback(&mut fragment_scores, &feedback);

    let next_packet =
        assemble_persisted_packet_with_scores(storage, fixture_path, &purpose, &fragment_scores)?;

    storage.insert_feedback(&feedback)?;
    storage.insert_packet(&next_packet)?;
    storage.commit_all(&format!("Apply Athena feedback {}", feedback.feedback_id))?;

    Ok(FeedbackApplyResult {
        feedback,
        created_fragments,
        fragment_scores,
        next_packet,
    })
}

fn assemble_persisted_packet(
    storage: &DoltStorage,
    fixture_path: impl AsRef<Path>,
    purpose: &Purpose,
) -> Result<PurposePacket, AthenaError> {
    assemble_persisted_packet_with_scores(storage, fixture_path, purpose, &FragmentScores::new())
}

fn assemble_persisted_packet_with_scores(
    storage: &DoltStorage,
    fixture_path: impl AsRef<Path>,
    purpose: &Purpose,
    fragment_scores: &FragmentScores,
) -> Result<PurposePacket, AthenaError> {
    let mut fragments = load_fragments(fixture_path)?;
    fragments.extend(storage.list_fragment_nodes()?);

    let mut packet = assemble_packet_with_scores(purpose, &fragments, fragment_scores)?;
    packet.packet_id = PacketId::new(unique_id("packet"));
    Ok(packet)
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn missing(kind: &str, id: &str) -> AthenaError {
    AthenaError::Io(std::io::Error::other(format!("missing {kind}: {id}")))
}
