use crate::ids::PurposeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurposeStatus {
    Active,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Purpose {
    pub purpose_id: PurposeId,
    pub statement: String,
    pub success_criteria: String,
    pub status: PurposeStatus,
}
