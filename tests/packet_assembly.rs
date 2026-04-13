use athena_v2::fragment::{Fragment, FragmentKind, FragmentState, load_fragments};
use athena_v2::ids::{FragmentId, PurposeId};
use athena_v2::packet::assemble_packet;
use athena_v2::purpose::{Purpose, PurposeStatus};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fragments.json")
}

#[test]
fn packet_assembly_is_deterministic() {
    let fragments = load_fragments(fixture_path()).unwrap();
    let purpose = Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: "Need feedback-safe tracer".into(),
        success_criteria: "Feedback loop finishes".into(),
        status: PurposeStatus::Active,
    };

    let packet = assemble_packet(&purpose, &fragments).unwrap();
    let fragment_ids: Vec<&str> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(packet.purpose_id, purpose.purpose_id);
    assert_eq!(fragment_ids, vec!["f1", "f2", "f3"]);
}

fn purpose(statement: &str, success_criteria: &str) -> Purpose {
    Purpose {
        purpose_id: PurposeId::new("purpose-1"),
        statement: statement.into(),
        success_criteria: success_criteria.into(),
        status: PurposeStatus::Active,
    }
}

#[test]
fn packet_assembly_filters_stale_superseded_and_duplicate_fragments() {
    let mut stale = Fragment::basic("f_stale", FragmentKind::Context, "stale", "stale");
    stale.state = FragmentState::Stale;

    let mut original = Fragment::basic("f_old", FragmentKind::Doctrine, "old", "old");
    original.concept_key = Some("determinism".into());

    let mut replacement = Fragment::basic("f_new", FragmentKind::Doctrine, "new", "new");
    replacement.concept_key = Some("determinism".into());
    replacement.usefulness_score = 3;
    replacement.supersedes = vec![FragmentId::new("f_old")];

    let keep = Fragment::basic("f_keep", FragmentKind::Procedure, "keep", "keep");
    let context = Fragment::basic("f_ctx", FragmentKind::Context, "context", "context");

    let packet = assemble_packet(
        &purpose("Need deterministic packet selection", "Exclude stale and duplicate guidance"),
        &[stale, original, replacement, keep, context],
    )
    .unwrap();

    let fragment_ids: Vec<&str> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(fragment_ids, vec!["f_new", "f_ctx", "f_keep"]);
}

#[test]
fn packet_assembly_honors_trigger_conditions() {
    let generic = Fragment::basic("f_generic", FragmentKind::Context, "generic", "generic");

    let mut triggered = Fragment::basic("f_triggered", FragmentKind::Pitfall, "feedback", "feedback");
    triggered.trigger_conditions = vec!["feedback".into()];
    triggered.usefulness_score = 1;

    let mut hidden = Fragment::basic("f_hidden", FragmentKind::Preference, "benchmark", "benchmark");
    hidden.trigger_conditions = vec!["benchmark".into()];
    hidden.usefulness_score = 10;

    let packet = assemble_packet(
        &purpose("Collect feedback carefully", "Preserve coverage"),
        &[generic, triggered, hidden],
    )
    .unwrap();

    let fragment_ids: Vec<&str> = packet
        .fragments
        .iter()
        .map(|fragment| fragment.fragment_id.0.as_str())
        .collect();

    assert_eq!(fragment_ids, vec!["f_triggered", "f_generic"]);
}
