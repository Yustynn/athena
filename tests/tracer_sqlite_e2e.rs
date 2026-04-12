use athena_v2::storage::SqliteStorage;
use athena_v2::tracer::run_tracer_persisted;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

fn unique_db_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("athena-{nanos}.sqlite"))
}

#[test]
fn tracer_persists_purpose_packet_and_feedback_in_sqlite() {
    let db_path = unique_db_path();

    let result = run_tracer_persisted(
        "Assemble packet and collect feedback",
        "Packet feedback invariant holds",
        fixture_path(),
        &db_path,
    )
    .unwrap();

    let storage = SqliteStorage::open(&db_path).unwrap();

    assert_eq!(
        storage.get_purpose(&result.purpose.purpose_id).unwrap(),
        Some(result.purpose.clone())
    );
    assert_eq!(
        storage.get_packet(&result.packet.packet_id).unwrap(),
        Some(result.packet.clone())
    );
    assert_eq!(
        storage.get_feedback(&result.feedback.feedback_id).unwrap(),
        Some(result.feedback.clone())
    );
}
