use athena_v2::feedback::FeedbackEvent;
use athena_v2::fragment::{Fragment, FragmentKind};
use athena_v2::ids::FragmentId;
use athena_v2::packet::PurposePacket;
use athena_v2::purpose::{Purpose, PurposeStatus};
use athena_v2::storage::DoltStorage;
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

fn unique_repo_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("athena-cli-{nanos}"))
}

fn run_athena(args: &[&str], stdin: Option<&str>) -> String {
    let bin = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("athena");
    let mut child = Command::new(bin)
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(stdin_body) = stdin {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin_body.as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn purpose_create_persists_purpose_and_first_packet() {
    let repo_path = unique_repo_path();
    let output = run_athena(
        &[
            "--db",
            repo_path.to_str().unwrap(),
            "--fixture",
            fixture_path().to_str().unwrap(),
            "purpose",
            "create",
            "--statement",
            "Replace sqlite storage with dolt",
            "--success-criteria",
            "Athena writes persisted packets",
        ],
        None,
    );

    let response: Value = serde_json::from_str(&output).unwrap();
    let purpose: Purpose = serde_json::from_value(response["purpose"].clone()).unwrap();
    let packet: PurposePacket = serde_json::from_value(response["packet"].clone()).unwrap();

    assert_eq!(purpose.status, PurposeStatus::Active);
    assert!(purpose.purpose_id.0.starts_with("purpose-"));
    assert_eq!(packet.purpose_id, purpose.purpose_id);
    assert!(packet.packet_id.0.starts_with("packet-"));
    assert_eq!(packet.fragments.len(), 3);

    let storage = DoltStorage::open(&repo_path).unwrap();
    assert_eq!(
        storage.get_purpose(&purpose.purpose_id).unwrap(),
        Some(purpose)
    );
    assert_eq!(storage.get_packet(&packet.packet_id).unwrap(), Some(packet));
}

#[test]
fn purpose_update_persists_updated_purpose_and_new_packet() {
    let repo_path = unique_repo_path();
    let create_output = run_athena(
        &[
            "--db",
            repo_path.to_str().unwrap(),
            "--fixture",
            fixture_path().to_str().unwrap(),
            "purpose",
            "create",
            "--statement",
            "Replace sqlite storage with dolt",
            "--success-criteria",
            "Athena writes persisted packets",
        ],
        None,
    );
    let create_response: Value = serde_json::from_str(&create_output).unwrap();
    let first_packet: PurposePacket =
        serde_json::from_value(create_response["packet"].clone()).unwrap();
    let purpose: Purpose = serde_json::from_value(create_response["purpose"].clone()).unwrap();

    let output = run_athena(
        &[
            "--db",
            repo_path.to_str().unwrap(),
            "--fixture",
            fixture_path().to_str().unwrap(),
            "purpose",
            "update",
            "--purpose-id",
            &purpose.purpose_id.0,
            "--statement",
            "Replace sqlite storage with tracked dolt memory",
            "--success-criteria",
            "Athena updates purpose and packet",
        ],
        None,
    );

    let response: Value = serde_json::from_str(&output).unwrap();
    let updated_purpose: Purpose = serde_json::from_value(response["purpose"].clone()).unwrap();
    let new_packet: PurposePacket = serde_json::from_value(response["packet"].clone()).unwrap();

    assert_eq!(updated_purpose.purpose_id, purpose.purpose_id);
    assert_eq!(
        updated_purpose.statement,
        "Replace sqlite storage with tracked dolt memory"
    );
    assert_eq!(
        updated_purpose.success_criteria,
        "Athena updates purpose and packet"
    );
    assert_eq!(new_packet.purpose_id, purpose.purpose_id);
    assert_ne!(new_packet.packet_id, first_packet.packet_id);

    let storage = DoltStorage::open(&repo_path).unwrap();
    assert_eq!(
        storage.get_purpose(&updated_purpose.purpose_id).unwrap(),
        Some(updated_purpose)
    );
    assert_eq!(
        storage.get_packet(&new_packet.packet_id).unwrap(),
        Some(new_packet)
    );
}

#[test]
fn feedback_apply_persists_feedback_new_fragments_and_next_packet() {
    let repo_path = unique_repo_path();
    let create_output = run_athena(
        &[
            "--db",
            repo_path.to_str().unwrap(),
            "--fixture",
            fixture_path().to_str().unwrap(),
            "purpose",
            "create",
            "--statement",
            "Replace sqlite storage with dolt",
            "--success-criteria",
            "Athena writes persisted feedback",
        ],
        None,
    );
    let create_response: Value = serde_json::from_str(&create_output).unwrap();
    let purpose: Purpose = serde_json::from_value(create_response["purpose"].clone()).unwrap();
    let packet: PurposePacket = serde_json::from_value(create_response["packet"].clone()).unwrap();

    let feedback_input = json!({
        "fragment_feedback": [
            {
                "fragment_id": packet.fragments[0].fragment_id.0,
                "verdict": "helped",
                "reason": "keep"
            },
            {
                "fragment_id": packet.fragments[1].fragment_id.0,
                "verdict": "wrong",
                "reason": "bad fit"
            },
            {
                "fragment_id": packet.fragments[2].fragment_id.0,
                "verdict": "helped",
                "reason": "still useful"
            }
        ],
        "new_fragments": [
            {
                "kind": "doctrine",
                "text": "Use tracked Dolt repo for Athena memory."
            }
        ]
    })
    .to_string();

    let output = run_athena(
        &[
            "--db",
            repo_path.to_str().unwrap(),
            "--fixture",
            fixture_path().to_str().unwrap(),
            "feedback",
            "apply",
            "--purpose-id",
            &purpose.purpose_id.0,
            "--packet-id",
            &packet.packet_id.0,
            "--outcome",
            "partial",
        ],
        Some(&feedback_input),
    );

    let response: Value = serde_json::from_str(&output).unwrap();
    let feedback: FeedbackEvent = serde_json::from_value(response["feedback"].clone()).unwrap();
    let next_packet: PurposePacket =
        serde_json::from_value(response["next_packet"].clone()).unwrap();
    let created_fragments = response["created_fragments"].as_array().unwrap();
    let created_fragment: Fragment =
        serde_json::from_value(response["created_fragments"][0].clone()).unwrap();

    assert_eq!(feedback.purpose_id, purpose.purpose_id);
    assert_eq!(feedback.packet_id, packet.packet_id);
    assert!(feedback.feedback_id.0.starts_with("feedback-"));
    assert_eq!(created_fragments.len(), 1);
    assert_eq!(created_fragment.kind, FragmentKind::Doctrine);
    assert_eq!(
        created_fragment.text,
        "Use tracked Dolt repo for Athena memory."
    );
    assert!(created_fragment.fragment_id.0.starts_with("fragment-"));
    assert_ne!(next_packet.packet_id, packet.packet_id);

    let storage = DoltStorage::open(&repo_path).unwrap();
    assert_eq!(
        storage.get_feedback(&feedback.feedback_id).unwrap(),
        Some(feedback)
    );
    assert_eq!(
        storage.get_packet(&next_packet.packet_id).unwrap(),
        Some(next_packet)
    );
    assert_eq!(
        storage
            .get_fragment_node(&FragmentId::new(created_fragment.fragment_id.0.clone()))
            .unwrap(),
        Some(created_fragment)
    );
}
