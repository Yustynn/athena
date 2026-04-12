use crate::error::AthenaError;
use crate::fragment::Fragment;
use std::fs;
use std::path::Path;

pub fn load_fragments(path: impl AsRef<Path>) -> Result<Vec<Fragment>, AthenaError> {
    let raw = fs::read_to_string(path)?;
    let mut fragments: Vec<Fragment> = serde_json::from_str(&raw)?;
    fragments.sort_by(|left, right| left.fragment_id.cmp(&right.fragment_id));

    if fragments.is_empty() {
        return Err(AthenaError::EmptyFragmentFixture);
    }

    Ok(fragments)
}
