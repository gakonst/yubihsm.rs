//! Generate a new HMAC key within the `YubiHSM2`
//!
//! <https://developers.yubico.com/YubiHSM2/Commands/Generate_Hmac_Key.html>

use super::generate_key::GenerateKeyParams;
use super::{Command, Response};
use {
    Adapter, Capability, CommandType, Domain, HmacAlg, ObjectId, ObjectLabel, Session, SessionError,
};

/// Generate a new HMAC key within the `YubiHSM2`
pub fn generate_hmac_key<A: Adapter>(
    session: &mut Session<A>,
    key_id: ObjectId,
    label: ObjectLabel,
    domains: Domain,
    capabilities: Capability,
    algorithm: HmacAlg,
) -> Result<ObjectId, SessionError> {
    session
        .send_command(GenHMACKeyCommand(GenerateKeyParams {
            key_id,
            label,
            domains,
            capabilities,
            algorithm: algorithm.into(),
        })).map(|response| response.key_id)
}

/// Request parameters for `command::generate_hmac_key`
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenHMACKeyCommand(pub(crate) GenerateKeyParams);

impl Command for GenHMACKeyCommand {
    type ResponseType = GenHMACKeyResponse;
}

/// Response from `command::generate_hmac_key`
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenHMACKeyResponse {
    /// ID of the key
    pub key_id: ObjectId,
}

impl Response for GenHMACKeyResponse {
    const COMMAND_TYPE: CommandType = CommandType::GenerateHMACKey;
}