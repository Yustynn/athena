use athena_v2::feedback::{FeedbackEvent, FragmentFeedback, FragmentVerdict, TaskOutcome};
use athena_v2::fragment::{Fragment, FragmentKind};
use athena_v2::ids::{FeedbackId, FragmentId, PacketId, PurposeId};
use athena_v2::packet::PurposePacket;
use athena_v2::purpose::{Purpose, PurposeStatus};
use athena_v2::storage::DoltStorage;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_purpose() -> Purpose {
    Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Build packet assembler tracer; don't regress".into(),
        success_criteria: "Feedback invariant persists".into(),
        status: PurposeStatus::Active,
    }
}

fn sample_fragments() -> Vec<Fragment> {
    vec![
        Fragment {
            fragment_id: FragmentId::new("f1"),
            kind: FragmentKind::Doctrine,
            summary: "Keep deterministic behavior".into(),
            full_text:
                "Keep deterministic behavior. Prefer stable packet assembly and feedback handling."
                    .into(),
        },
        Fragment {
            fragment_id: FragmentId::new("f2"),
            kind: FragmentKind::Pitfall,
            summary: "Do not skip per-fragment feedback".into(),
            full_text: "Do not skip per-fragment feedback\twith tabs\nor new lines".into(),
        },
    ]
}

fn sample_packet() -> PurposePacket {
    PurposePacket {
        packet_id: PacketId::new("packet-1"),
        purpose_id: PurposeId::new("purpose-1"),
        fragments: sample_fragments(),
    }
}

fn sample_feedback() -> FeedbackEvent {
    FeedbackEvent {
        feedback_id: FeedbackId::new("feedback-1"),
        purpose_id: PurposeId::new("purpose-1"),
        packet_id: PacketId::new("packet-1"),
        outcome: TaskOutcome::Success,
        fragment_feedback: vec![
            FragmentFeedback {
                fragment_id: FragmentId::new("f1"),
                verdict: FragmentVerdict::Helped,
                reason: Some("critical baseline".into()),
            },
            FragmentFeedback {
                fragment_id: FragmentId::new("f2"),
                verdict: FragmentVerdict::Wrong,
                reason: Some("was redundant in this run; don't repeat".into()),
            },
        ],
    }
}

fn unique_repo_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("athena-dolt-{nanos}"))
}

#[test]
fn dolt_crud_round_trip_for_core_objects() {
    let repo_path = unique_repo_path();
    let storage = DoltStorage::open(&repo_path).unwrap();

    let purpose = sample_purpose();
    storage.insert_purpose(&purpose).unwrap();
    assert_eq!(
        storage.get_purpose(&purpose.purpose_id).unwrap(),
        Some(purpose.clone())
    );

    let packet = sample_packet();
    storage.insert_packet(&packet).unwrap();
    assert_eq!(
        storage.get_packet(&packet.packet_id).unwrap(),
        Some(packet.clone())
    );

    let feedback = sample_feedback();
    storage.insert_feedback(&feedback).unwrap();
    assert_eq!(
        storage.get_feedback(&feedback.feedback_id).unwrap(),
        Some(feedback.clone())
    );
}

#[test]
fn dolt_fragment_node_is_immutable_and_edges_link_nodes() {
    let repo_path = unique_repo_path();
    let storage = DoltStorage::open(&repo_path).unwrap();

    storage
        .insert_fragment_node(
            &FragmentId::new("f_old"),
            &FragmentKind::Doctrine,
            "Old deterministic guidance",
            "Old deterministic guidance. Earlier version of paragraph-sized memory.",
        )
        .unwrap();

    storage
        .insert_fragment_node(
            &FragmentId::new("f_new"),
            &FragmentKind::Doctrine,
            "Updated deterministic guidance",
            "Updated deterministic guidance. New paragraph replaces earlier wording.",
        )
        .unwrap();

    storage
        .insert_fragment_edge(
            &FragmentId::new("f_old"),
            &FragmentId::new("f_new"),
            "rewrites",
        )
        .unwrap();

    // trying to mutate existing node in place must fail
    let in_place_update = storage.insert_fragment_node(
        &FragmentId::new("f_old"),
        &FragmentKind::Doctrine,
        "mutated text",
        "mutated text that should not overwrite immutable fragment body",
    );
    assert!(in_place_update.is_err());

    let edges = storage.outgoing_edges(&FragmentId::new("f_old")).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0], (FragmentId::new("f_new"), "rewrites".to_string()));
}
