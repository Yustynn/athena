use crate::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentVerdict {
    Helped,
    Neutral,
    Wrong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentFeedback {
    pub fragment_id: FragmentId,
    pub verdict: FragmentVerdict,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub feedback_id: FeedbackId,
    pub purpose_id: PurposeId,
    pub packet_id: PacketId,
    pub outcome: TaskOutcome,
    pub fragment_feedback: Vec<FragmentFeedback>,
}
