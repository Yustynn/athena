use crate::ids::FragmentId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Doctrine,
    Procedure,
    Pitfall,
    Context,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fragment {
    pub fragment_id: FragmentId,
    pub kind: FragmentKind,
    pub text: String,
}
