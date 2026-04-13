use crate::ids::FragmentId;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Doctrine,
    Procedure,
    Pitfall,
    Preference,
    Context,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FragmentState {
    Scratch,
    #[default]
    Durable,
    Deferred,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fragment {
    pub fragment_id: FragmentId,
    pub kind: FragmentKind,
    pub summary: String,
    pub full_text: String,
    pub scope: Option<String>,
    pub trigger_conditions: Vec<String>,
    pub state: FragmentState,
    pub concept_key: Option<String>,
    pub usefulness_score: i32,
    pub correctness_confidence: i32,
    pub durability_score: i32,
    pub stale_after: Option<String>,
    pub supersedes: Vec<FragmentId>,
}

#[derive(Debug, Deserialize)]
struct FragmentWire {
    fragment_id: FragmentId,
    kind: FragmentKind,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    full_text: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    trigger_conditions: Vec<String>,
    #[serde(default)]
    state: FragmentState,
    #[serde(default)]
    concept_key: Option<String>,
    #[serde(default)]
    usefulness_score: i32,
    #[serde(default)]
    correctness_confidence: i32,
    #[serde(default)]
    durability_score: i32,
    #[serde(default)]
    stale_after: Option<String>,
    #[serde(default)]
    supersedes: Vec<FragmentId>,
}

impl<'de> Deserialize<'de> for Fragment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FragmentWire::deserialize(deserializer)?;
        let summary = wire
            .summary
            .or_else(|| wire.text.clone())
            .or_else(|| wire.full_text.clone())
            .unwrap_or_default();
        let full_text = wire
            .full_text
            .or(wire.text)
            .unwrap_or_else(|| summary.clone());

        Ok(Self {
            fragment_id: wire.fragment_id,
            kind: wire.kind,
            summary,
            full_text,
            scope: wire.scope,
            trigger_conditions: wire.trigger_conditions,
            state: wire.state,
            concept_key: wire.concept_key,
            usefulness_score: wire.usefulness_score,
            correctness_confidence: wire.correctness_confidence,
            durability_score: wire.durability_score,
            stale_after: wire.stale_after,
            supersedes: wire.supersedes,
        })
    }
}

impl Fragment {
    pub fn basic(
        fragment_id: impl Into<String>,
        kind: FragmentKind,
        summary: impl Into<String>,
        full_text: impl Into<String>,
    ) -> Self {
        Self {
            fragment_id: FragmentId::new(fragment_id),
            kind,
            summary: summary.into(),
            full_text: full_text.into(),
            scope: None,
            trigger_conditions: Vec::new(),
            state: FragmentState::Durable,
            concept_key: None,
            usefulness_score: 0,
            correctness_confidence: 0,
            durability_score: 0,
            stale_after: None,
            supersedes: Vec::new(),
        }
    }
}
