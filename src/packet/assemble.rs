use crate::error::AthenaError;
use crate::fragment::{Fragment, FragmentKind};
use crate::ids::PacketId;
use crate::packet::PurposePacket;
use crate::purpose::Purpose;

pub fn assemble_packet(
    purpose: &Purpose,
    fragments: &[Fragment],
) -> Result<PurposePacket, AthenaError> {
    let mut selected = Vec::new();
    let input = format!("{} {}", purpose.statement, purpose.success_criteria).to_lowercase();

    if let Some(fragment) = fragments
        .iter()
        .find(|fragment| fragment.kind == FragmentKind::Doctrine)
    {
        selected.push(fragment.clone());
    }

    if input.contains("feedback") {
        if let Some(fragment) = fragments
            .iter()
            .find(|fragment| fragment.kind == FragmentKind::Pitfall)
            .filter(|fragment| {
                !selected
                    .iter()
                    .any(|existing| existing.fragment_id == fragment.fragment_id)
            })
        {
            selected.push(fragment.clone());
        }
    }

    for fragment in fragments {
        if selected.len() >= 3 {
            break;
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
