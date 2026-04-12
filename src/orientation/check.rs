use crate::orientation::{CorrectionPacket, OrientationResponse};
use crate::packet::PurposePacket;
use crate::purpose::Purpose;

pub fn check_orientation(
    purpose: &Purpose,
    packet: &PurposePacket,
    response: &OrientationResponse,
) -> Option<CorrectionPacket> {
    let missing_constraints = required_constraints(&purpose.success_criteria)
        .into_iter()
        .filter(|constraint| !contains_ignore_case(&response.best_path, constraint))
        .collect::<Vec<_>>();

    if missing_constraints.is_empty() {
        return None;
    }

    let notes = missing_constraints
        .iter()
        .map(|constraint| format!("Include explicit plan step for: {constraint}"))
        .collect();

    Some(CorrectionPacket {
        purpose_id: purpose.purpose_id.clone(),
        packet_id: packet.packet_id.clone(),
        missing_constraints,
        notes,
    })
}

fn required_constraints(success_criteria: &str) -> Vec<String> {
    success_criteria
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .flat_map(|part| part.split(" and "))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_lowercase())
        .collect()
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}
