use crate::ids::{PacketId, PurposeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrientationResponse {
    pub purpose_id: PurposeId,
    pub packet_id: PacketId,
    pub best_path: String,
    pub addressed_constraints: Vec<String>,
    pub unresolved_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionPacket {
    pub purpose_id: PurposeId,
    pub packet_id: PacketId,
    pub missing_constraints: Vec<String>,
    pub notes: Vec<String>,
}
