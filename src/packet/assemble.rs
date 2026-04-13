use crate::error::AthenaError;
use crate::fragment::{Fragment, FragmentState};
use crate::ids::PacketId;
use crate::packet::PurposePacket;
use crate::purpose::Purpose;
use std::collections::{BTreeMap, BTreeSet};

pub fn assemble_packet(
    purpose: &Purpose,
    fragments: &[Fragment],
) -> Result<PurposePacket, AthenaError> {
    assemble_packet_with_scores(purpose, fragments, &BTreeMap::new())
}

pub fn assemble_packet_with_scores(
    purpose: &Purpose,
    fragments: &[Fragment],
    fragment_scores: &BTreeMap<String, i32>,
) -> Result<PurposePacket, AthenaError> {
    let input = format!("{} {}", purpose.statement, purpose.success_criteria).to_lowercase();
    let superseded_ids = active_superseded_ids(fragments);

    let mut ranked = fragments
        .iter()
        .filter(|fragment| should_consider(fragment, &input, &superseded_ids))
        .map(|fragment| {
            (
                -rank_score(fragment, &input, fragment_scores),
                fragment.fragment_id.clone(),
                fragment,
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut selected = Vec::new();
    let mut concept_keys = BTreeSet::new();

    for (_, _, fragment) in ranked {
        if selected.len() >= 3 {
            break;
        }

        if let Some(key) = fragment.concept_key.as_ref() {
            if !concept_keys.insert(key.to_lowercase()) {
                continue;
            }
        }

        selected.push(fragment.clone());
    }

    if selected.is_empty() {
        return Err(AthenaError::EmptyPacket);
    }

    Ok(PurposePacket {
        packet_id: PacketId::new("packet-1"),
        purpose_id: purpose.purpose_id.clone(),
        fragments: selected,
    })
}

fn score_for(fragment: &Fragment, fragment_scores: &BTreeMap<String, i32>) -> i32 {
    *fragment_scores.get(&fragment.fragment_id.0).unwrap_or(&0)
}

fn should_consider(
    fragment: &Fragment,
    input: &str,
    superseded_ids: &BTreeSet<String>,
) -> bool {
    if matches!(fragment.state, FragmentState::Stale | FragmentState::Superseded) {
        return false;
    }

    if superseded_ids.contains(&fragment.fragment_id.0) {
        return false;
    }

    if fragment
        .scope
        .as_ref()
        .is_some_and(|scope| !input.contains(&scope.to_lowercase()))
    {
        return false;
    }

    fragment.trigger_conditions.is_empty()
        || fragment
            .trigger_conditions
            .iter()
            .any(|trigger| input.contains(&trigger.to_lowercase()))
}

fn active_superseded_ids(fragments: &[Fragment]) -> BTreeSet<String> {
    fragments
        .iter()
        .filter(|fragment| !matches!(fragment.state, FragmentState::Stale | FragmentState::Superseded))
        .flat_map(|fragment| fragment.supersedes.iter().map(|fragment_id| fragment_id.0.clone()))
        .collect()
}

fn rank_score(fragment: &Fragment, input: &str, fragment_scores: &BTreeMap<String, i32>) -> i32 {
    let trigger_bonus = if fragment.trigger_conditions.is_empty() {
        0
    } else if fragment
        .trigger_conditions
        .iter()
        .any(|trigger| input.contains(&trigger.to_lowercase()))
    {
        2
    } else {
        0
    };

    score_for(fragment, fragment_scores)
        + fragment.usefulness_score
        + fragment.correctness_confidence
        + fragment.durability_score
        + trigger_bonus
}
