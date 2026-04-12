use athena_v2::tracer::{run_feedback_loop, run_tracer_persisted};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

fn db_path() -> PathBuf {
    std::env::temp_dir().join("athena-dogfood.sqlite")
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
    println!("dogfood db: {}", db.display());
    println!("first packet fragments: {:?}", first_ids);
    println!("second packet fragments: {:?}", second_ids);
}
