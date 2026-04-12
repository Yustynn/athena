use thiserror::Error;

#[derive(Debug, Error)]
pub enum AthenaError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("fixture had no fragments")]
    EmptyFragmentFixture,

    #[error("packet had no selected fragments")]
    EmptyPacket,

    #[error("missing fragment feedback for: {0:?}")]
    MissingFragmentFeedback(Vec<String>),

    #[error("feedback referenced packet-external fragments: {0:?}")]
    ExtraFragmentFeedback(Vec<String>),
}
