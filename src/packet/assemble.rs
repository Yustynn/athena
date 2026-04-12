use crate::error::AthenaError;
use crate::fragment::{Fragment, FragmentKind};
use crate::ids::PacketId;
use crate::packet::PurposePacket;
use crate::purpose::Purpose;
use std::collections::BTreeMap;

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
    let mut selected = Vec::new();
    let input = format!("{} {}", purpose.statement, purpose.success_criteria).to_lowercase();

    if let Some(fragment) = fragments.iter().find(|fragment| {
        fragment.kind == FragmentKind::Doctrine && score_for(fragment, fragment_scores) > -2
    }) {
        selected.push(fragment.clone());
    }

    if input.contains("feedback") {
        if let Some(fragment) = fragments
            .iter()
            .find(|fragment| {
                fragment.kind == FragmentKind::Pitfall && score_for(fragment, fragment_scores) > -2
            })
            .filter(|fragment| {
                !selected
                    .iter()
                    .any(|existing| existing.fragment_id == fragment.fragment_id)
            })
        {
            selected.push(fragment.clone());
        }
    }

    let mut ranked_indices: Vec<usize> = (0..fragments.len()).collect();
    ranked_indices.sort_by_key(|index| {
        (
            -score_for(&fragments[*index], fragment_scores),
            *index as i32,
        )
    });

    for index in ranked_indices {
        let fragment = &fragments[index];
        if selected.len() >= 3 {
            break;
        }

        if score_for(fragment, fragment_scores) <= -2 {
            continue;
        }

        if selected
            .iter()
            .any(|existing| existing.fragment_id == fragment.fragment_id)
        {
            continue;
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
