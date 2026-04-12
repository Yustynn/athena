use athena_v2::tracer::{run_feedback_loop, run_tracer_persisted};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

fn tracked_db_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".athena/db")
}

fn db_path_from_env(override_path: Option<&str>) -> PathBuf {
    if let Some(path) = override_path {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }

    tracked_db_path()
}

fn db_path() -> PathBuf {
    let override_path = std::env::var("ATHENA_DOGFOOD_DB_DIR").ok();
    db_path_from_env(override_path.as_deref())
}

fn main() {
    let fixture = fixture_path();
    let db = db_path();

    let persisted = run_tracer_persisted(
        "Dogfood packet assembly and feedback loop",
        "Second packet should improve after feedback",
        &fixture,
        &db,
    )
    .expect("persisted tracer run should succeed");

    let loop_result = run_feedback_loop(
        "Dogfood packet assembly and feedback loop",
        "Second packet should improve after feedback",
        &fixture,
    )
    .expect("feedback loop run should succeed");

    let first_ids: Vec<&str> = loop_result
        .first_packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();
    let second_ids: Vec<&str> = loop_result
        .second_packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    println!("persisted packet id: {}", persisted.packet.packet_id);
    println!("dogfood repo: {}", db.display());
    println!("first packet fragments: {:?}", first_ids);
    println!("second packet fragments: {:?}", second_ids);
}

#[cfg(test)]
mod tests {
    use super::{db_path_from_env, tracked_db_path};

    #[test]
    fn db_path_uses_override_when_non_empty() {
        let result = db_path_from_env(Some("/tmp/custom-dogfood-repo"));
        assert_eq!(
            result.to_string_lossy().to_string(),
            "/tmp/custom-dogfood-repo"
        );
    }

    #[test]
    fn db_path_ignores_empty_or_whitespace_override() {
        let expected = tracked_db_path();
        assert_eq!(db_path_from_env(Some("")).as_path(), expected.as_path());
        assert_eq!(db_path_from_env(Some("   ")).as_path(), expected.as_path());
    }

    #[test]
    fn db_path_defaults_when_override_missing() {
        let expected = tracked_db_path();
        assert_eq!(db_path_from_env(None).as_path(), expected.as_path());
    }
}
