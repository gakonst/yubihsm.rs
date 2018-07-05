//! Commands supported by the `MockHSM`

use commands::*;
use connector::ConnectorError;
use responses::*;
use securechannel::{CommandMessage, CommandType, ResponseMessage};
use serializers::deserialize;
use {Algorithm, ObjectId, ObjectType};

use super::objects::Object;
use super::state::State;

/// Default auth key ID slot
const DEFAULT_AUTH_KEY_ID: ObjectId = 1;

/// Create a new HSM session
pub(crate) fn create_session(
    state: &mut State,
    cmd_message: &CommandMessage,
) -> Result<Vec<u8>, ConnectorError> {
    let cmd: CreateSessionCommand = deserialize(cmd_message.data.as_ref())
        .unwrap_or_else(|e| panic!("error parsing CreateSession command data: {:?}", e));

    assert_eq!(
        cmd.auth_key_id, DEFAULT_AUTH_KEY_ID,
        "unexpected auth key ID: {}",
        cmd.auth_key_id
    );

    let session = state.create_session(cmd.host_challenge);

    let mut response = CreateSessionResponse {
        card_challenge: *session.card_challenge(),
        card_cryptogram: session.card_cryptogram(),
    }.serialize();

    response.session_id = Some(session.id);
    Ok(response.into())
}

/// Authenticate an HSM session
pub(crate) fn authenticate_session(
    state: &mut State,
    command: &CommandMessage,
) -> Result<Vec<u8>, ConnectorError> {
    let session_id = command
        .session_id
        .unwrap_or_else(|| panic!("no session ID in command: {:?}", command.command_type));

    Ok(state
        .get_session(session_id)
        .channel
        .verify_authenticate_session(command)
        .unwrap()
        .into())
}

/// Encrypted session messages
pub(crate) fn session_message(
    state: &mut State,
    encrypted_command: CommandMessage,
) -> Result<Vec<u8>, ConnectorError> {
    let session_id = encrypted_command.session_id.unwrap_or_else(|| {
        panic!(
            "no session ID in command: {:?}",
            encrypted_command.command_type
        )
    });

    let command = state
        .get_session(session_id)
        .decrypt_command(encrypted_command);

    let response = match command.command_type {
        CommandType::Blink => BlinkResponse {}.serialize(),
        CommandType::DeleteObject => delete_object(state, &command.data),
        CommandType::Echo => echo(&command.data),
        CommandType::GenAsymmetricKey => gen_asymmetric_key(state, &command.data),
        CommandType::GetDeviceInfo => get_device_info(),
        CommandType::GetLogs => get_logs(),
        CommandType::GetObjectInfo => get_object_info(state, &command.data),
        CommandType::GetPubKey => get_pubkey(state, &command.data),
        CommandType::ListObjects => list_objects(state, &command.data),
        CommandType::SignDataEdDSA => sign_data_eddsa(state, &command.data),
        unsupported => panic!("unsupported command type: {:?}", unsupported),
    };

    Ok(state
        .get_session(session_id)
        .encrypt_response(response)
        .into())
}

/// Delete an object
fn delete_object(state: &mut State, cmd_data: &[u8]) -> ResponseMessage {
    let command: DeleteObjectCommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::DeleteObject: {:?}", e));

    match command.object_type {
        // TODO: support other asymmetric keys besides Ed25519 keys
        ObjectType::Asymmetric => match state.objects.ed25519_keys.remove(&command.object_id) {
            Some(_) => DeleteObjectResponse {}.serialize(),
            None => ResponseMessage::error(&format!("no such object ID: {:?}", command.object_id)),
        },
        _ => panic!("MockHSM only supports delete_object for ObjectType::Asymmetric"),
    }
}

/// Echo a message back to the host
fn echo(cmd_data: &[u8]) -> ResponseMessage {
    EchoResponse {
        message: cmd_data.into(),
    }.serialize()
}

/// Generate a new random asymmetric key
fn gen_asymmetric_key(state: &mut State, cmd_data: &[u8]) -> ResponseMessage {
    let command: GenAsymmetricKeyCommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::GetObjectInfo: {:?}", e));

    match command.algorithm {
        Algorithm::EC_ED25519 => {
            let key = Object::new(command.label, command.capabilities, command.domains);
            assert!(
                state
                    .objects
                    .ed25519_keys
                    .insert(command.key_id, key)
                    .is_none()
            );
        }
        other => panic!("unsupported algorithm: {:?}", other),
    }

    GenAsymmetricKeyResponse {
        key_id: command.key_id,
    }.serialize()
}

/// Generate a mock device information report
fn get_device_info() -> ResponseMessage {
    GetDeviceInfoResponse {
        major_version: 2,
        minor_version: 0,
        build_version: 0,
        serial_number: 2_000_000,
        log_store_capacity: 62,
        log_store_used: 62,
        algorithms: vec![
            Algorithm::AES128_CCM_WRAP,
            Algorithm::AES192_CCM_WRAP,
            Algorithm::AES256_CCM_WRAP,
            Algorithm::EC_BP256,
            Algorithm::EC_BP384,
            Algorithm::EC_BP512,
            Algorithm::EC_ECDH,
            Algorithm::EC_ECDSA_SHA1,
            Algorithm::EC_ECDSA_SHA256,
            Algorithm::EC_ECDSA_SHA384,
            Algorithm::EC_ECDSA_SHA512,
            Algorithm::EC_ED25519,
            Algorithm::EC_K256,
            Algorithm::EC_P224,
            Algorithm::EC_P256,
            Algorithm::EC_P384,
            Algorithm::EC_P521,
            Algorithm::HMAC_SHA1,
            Algorithm::HMAC_SHA256,
            Algorithm::HMAC_SHA384,
            Algorithm::HMAC_SHA512,
            Algorithm::MGF1_SHA1,
            Algorithm::MGF1_SHA256,
            Algorithm::MGF1_SHA384,
            Algorithm::MGF1_SHA512,
            Algorithm::OPAQUE_DATA,
            Algorithm::OPAQUE_X509_CERT,
            Algorithm::RSA2048,
            Algorithm::RSA3072,
            Algorithm::RSA4096,
            Algorithm::RSA_OAEP_SHA1,
            Algorithm::RSA_OAEP_SHA256,
            Algorithm::RSA_OAEP_SHA384,
            Algorithm::RSA_OAEP_SHA512,
            Algorithm::RSA_PKCS1_SHA1,
            Algorithm::RSA_PKCS1_SHA256,
            Algorithm::RSA_PKCS1_SHA384,
            Algorithm::RSA_PKCS1_SHA512,
            Algorithm::RSA_PSS_SHA1,
            Algorithm::RSA_PSS_SHA256,
            Algorithm::RSA_PSS_SHA384,
            Algorithm::RSA_PSS_SHA512,
            Algorithm::TEMPL_SSH,
            Algorithm::YUBICO_AES_AUTH,
            Algorithm::YUBICO_OTP_AES128,
            Algorithm::YUBICO_OTP_AES192,
            Algorithm::YUBICO_OTP_AES256,
        ],
    }.serialize()
}

/// Get mock log information
fn get_logs() -> ResponseMessage {
    // TODO: mimic the YubiHSM's actual audit log
    GetLogsResponse {
        unlogged_boot_events: 0,
        unlogged_auth_events: 0,
        num_entries: 0,
        entries: vec![],
    }.serialize()
}

/// Get detailed info about a specific object
fn get_object_info(state: &State, cmd_data: &[u8]) -> ResponseMessage {
    let command: GetObjectInfoCommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::GetObjectInfo: {:?}", e));

    if command.object_type != ObjectType::Asymmetric {
        panic!("MockHSM only supports ObjectType::Asymmetric for now");
    }

    // TODO: support other asymmetric keys besides Ed25519 keys
    match state.objects.ed25519_keys.get(&command.object_id) {
        Some(key) => GetObjectInfoResponse {
            capabilities: key.capabilities,
            id: command.object_id,
            length: key.length,
            domains: key.domains,
            object_type: key.object_type,
            algorithm: key.algorithm,
            sequence: key.sequence,
            origin: key.origin,
            label: key.label.clone(),
            delegated_capabilities: key.delegated_capabilities,
        }.serialize(),
        None => ResponseMessage::error(&format!("no such object ID: {:?}", command.object_id)),
    }
}

/// Get the public key associated with a key in the HSM
fn get_pubkey(state: &State, cmd_data: &[u8]) -> ResponseMessage {
    let command: GetPubKeyCommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::GetPubKey: {:?}", e));

    // TODO: support other asymmetric keys besides Ed25519 keys
    match state.objects.ed25519_keys.get(&command.key_id) {
        Some(key) => GetPubKeyResponse {
            algorithm: Algorithm::EC_ED25519,
            data: key.value.public_key_bytes().into(),
        }.serialize(),
        None => ResponseMessage::error(&format!("no such object ID: {:?}", command.key_id)),
    }
}

/// List all objects presently accessible to a session
fn list_objects(state: &State, cmd_data: &[u8]) -> ResponseMessage {
    // TODO: filter support
    let _command: ListObjectsCommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::ListObjects: {:?}", e));

    // TODO: support other asymmetric keys besides Ed25519 keys
    let list_entries = state
        .objects
        .ed25519_keys
        .iter()
        .map(|(object_id, object)| ListObjectsEntry {
            id: *object_id,
            object_type: object.object_type,
            sequence: object.sequence,
        })
        .collect();

    ListObjectsResponse {
        objects: list_entries,
    }.serialize()
}

/// Sign a message using the Ed25519 signature algorithm
fn sign_data_eddsa(state: &State, cmd_data: &[u8]) -> ResponseMessage {
    let command: SignDataEdDSACommand = deserialize(cmd_data)
        .unwrap_or_else(|e| panic!("error parsing CommandType::SignDataEdDSA: {:?}", e));

    // TODO: support other asymmetric keys besides Ed25519 keys
    match state.objects.ed25519_keys.get(&command.key_id) {
        Some(key) => {
            let signature = key.value.sign(command.data.as_ref());
            SignDataEdDSAResponse {
                signature: signature.as_ref().into(),
            }.serialize()
        }
        None => ResponseMessage::error(&format!("no such object ID: {:?}", command.key_id)),
    }
}