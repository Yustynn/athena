use crate::fragment::Fragment;
use crate::ids::{PacketId, PurposeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurposePacket {
    pub packet_id: PacketId,
    pub purpose_id: PurposeId,
    pub fragments: Vec<Fragment>,
}
