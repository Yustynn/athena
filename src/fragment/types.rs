use crate::ids::FragmentId;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Doctrine,
    Procedure,
    Pitfall,
    Context,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fragment {
    pub fragment_id: FragmentId,
    pub kind: FragmentKind,
    pub summary: String,
    pub full_text: String,
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
        })
    }
}
