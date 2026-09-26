use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::wire::PROTOCOL_VERSION;

pub(super) const HANDSHAKE_BYTES: usize = 48;
const HANDSHAKE_MAGIC: [u8; 8] = *b"AMEJH3\0\0";
pub(super) const CLIENT_PROOF_BYTES: usize = 48;
const CLIENT_PROOF_MAGIC: [u8; 8] = *b"AMEJCP3\0";
pub(super) const SERVER_ACCEPT_BYTES: usize = 48;
const SERVER_ACCEPT_MAGIC: [u8; 8] = *b"AMEJOK3\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ServerConnectionIdentity {
    pub(super) connection_id: [u8; 16],
    pub(super) connection_generation: u64,
    pub(super) connection_nonce: u64,
}

impl ServerConnectionIdentity {
    fn validate(self) -> Result<Self, ConnectionIdentityError> {
        if self.connection_id == [0; 16]
            || self.connection_generation == 0
            || self.connection_nonce == 0
        {
            return Err(ConnectionIdentityError::Invalid);
        }
        Ok(self)
    }
}

pub(super) trait ConnectionIdentitySource: Send + Sync {
    fn next_identity(&self) -> Result<ServerConnectionIdentity, ConnectionIdentityError>;
}

pub(super) struct ProcessConnectionIdentitySource {
    generation: AtomicU64,
    seed: [u8; 32],
}

impl ProcessConnectionIdentitySource {
    pub(super) fn new() -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"cedarflake-ame-journal-broker-connection-v1\0");
        hasher.update(&std::process::id().to_le_bytes());
        hasher.update(
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_le_bytes(),
        );
        Self {
            generation: AtomicU64::new(1),
            seed: *hasher.finalize().as_bytes(),
        }
    }
}

impl ConnectionIdentitySource for ProcessConnectionIdentitySource {
    fn next_identity(&self) -> Result<ServerConnectionIdentity, ConnectionIdentityError> {
        let generation = self
            .generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ConnectionIdentityError::Exhausted)?;
        let mut hasher = blake3::Hasher::new_keyed(&self.seed);
        hasher.update(&generation.to_le_bytes());
        let digest = hasher.finalize();
        let bytes = digest.as_bytes();
        let mut connection_id = [0_u8; 16];
        connection_id.copy_from_slice(&bytes[..16]);
        let mut nonce_bytes = [0_u8; 8];
        nonce_bytes.copy_from_slice(&bytes[16..24]);
        ServerConnectionIdentity {
            connection_id,
            connection_generation: generation,
            connection_nonce: u64::from_le_bytes(nonce_bytes),
        }
        .validate()
    }
}

pub(super) fn encode_handshake(identity: ServerConnectionIdentity) -> [u8; HANDSHAKE_BYTES] {
    encode_identity_message(identity, HANDSHAKE_MAGIC)
}

pub(super) fn encode_client_proof(identity: ServerConnectionIdentity) -> [u8; CLIENT_PROOF_BYTES] {
    encode_identity_message(identity, CLIENT_PROOF_MAGIC)
}

pub(super) fn encode_server_accept(
    identity: ServerConnectionIdentity,
) -> [u8; SERVER_ACCEPT_BYTES] {
    encode_identity_message(identity, SERVER_ACCEPT_MAGIC)
}

fn encode_identity_message(
    identity: ServerConnectionIdentity,
    magic: [u8; 8],
) -> [u8; HANDSHAKE_BYTES] {
    let mut bytes = [0_u8; HANDSHAKE_BYTES];
    bytes[..8].copy_from_slice(&magic);
    bytes[8..10].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    bytes[16..32].copy_from_slice(&identity.connection_id);
    bytes[32..40].copy_from_slice(&identity.connection_generation.to_le_bytes());
    bytes[40..48].copy_from_slice(&identity.connection_nonce.to_le_bytes());
    bytes
}

#[allow(
    dead_code,
    reason = "the decoder is consumed by the R2c-O production adapter seam wired during R2c-P"
)]
pub(super) fn decode_handshake(
    bytes: &[u8],
) -> Result<ServerConnectionIdentity, ConnectionIdentityError> {
    decode_identity_message(bytes, HANDSHAKE_MAGIC)
}

pub(super) fn validate_client_proof(
    expected: ServerConnectionIdentity,
    bytes: &[u8],
) -> Result<(), ConnectionIdentityError> {
    let actual = decode_identity_message(bytes, CLIENT_PROOF_MAGIC)?;
    if actual != expected {
        return Err(ConnectionIdentityError::ProtocolMismatch);
    }
    Ok(())
}

pub(super) fn validate_server_accept(
    expected: ServerConnectionIdentity,
    bytes: &[u8],
) -> Result<(), ConnectionIdentityError> {
    let actual = decode_identity_message(bytes, SERVER_ACCEPT_MAGIC)?;
    if actual != expected {
        return Err(ConnectionIdentityError::ProtocolMismatch);
    }
    Ok(())
}

fn decode_identity_message(
    bytes: &[u8],
    magic: [u8; 8],
) -> Result<ServerConnectionIdentity, ConnectionIdentityError> {
    if bytes.len() != HANDSHAKE_BYTES || bytes[..8] != magic {
        return Err(ConnectionIdentityError::ProtocolMismatch);
    }
    let version = u16::from_le_bytes(
        bytes[8..10]
            .try_into()
            .map_err(|_| ConnectionIdentityError::ProtocolMismatch)?,
    );
    if version != PROTOCOL_VERSION || bytes[10..16] != [0; 6] {
        return Err(ConnectionIdentityError::ProtocolMismatch);
    }
    ServerConnectionIdentity {
        connection_id: bytes[16..32]
            .try_into()
            .map_err(|_| ConnectionIdentityError::ProtocolMismatch)?,
        connection_generation: u64::from_le_bytes(
            bytes[32..40]
                .try_into()
                .map_err(|_| ConnectionIdentityError::ProtocolMismatch)?,
        ),
        connection_nonce: u64::from_le_bytes(
            bytes[40..48]
                .try_into()
                .map_err(|_| ConnectionIdentityError::ProtocolMismatch)?,
        ),
    }
    .validate()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConnectionIdentityError {
    #[allow(
        dead_code,
        reason = "constructed by the production handshake decoder wired during R2c-P"
    )]
    ProtocolMismatch,
    Invalid,
    Exhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_source_owns_unique_identity_generation_and_nonce() {
        let source = ProcessConnectionIdentitySource::new();
        let first = source.next_identity().expect("first identity");
        let second = source.next_identity().expect("second identity");

        assert_ne!(first.connection_id, second.connection_id);
        assert_ne!(first.connection_generation, second.connection_generation);
        assert_ne!(first.connection_nonce, second.connection_nonce);
        assert_eq!(decode_handshake(&encode_handshake(first)), Ok(first));
    }

    #[test]
    fn handshake_rejects_protocol_and_reserved_field_changes() {
        let source = ProcessConnectionIdentitySource::new();
        let identity = source.next_identity().expect("identity");
        let mut wrong_version = encode_handshake(identity);
        wrong_version[8..10].copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
        assert_eq!(
            decode_handshake(&wrong_version),
            Err(ConnectionIdentityError::ProtocolMismatch)
        );

        let mut reserved = encode_handshake(identity);
        reserved[10] = 1;
        assert_eq!(
            decode_handshake(&reserved),
            Err(ConnectionIdentityError::ProtocolMismatch)
        );
    }

    #[test]
    fn client_proof_is_versioned_and_rejects_replayed_connection_identity() {
        let source = ProcessConnectionIdentitySource::new();
        let first = source.next_identity().expect("first identity");
        let second = source.next_identity().expect("second identity");
        let proof = encode_client_proof(first);

        assert_eq!(validate_client_proof(first, &proof), Ok(()));
        assert_eq!(
            validate_client_proof(second, &proof),
            Err(ConnectionIdentityError::ProtocolMismatch)
        );
        assert_eq!(
            validate_client_proof(first, &encode_handshake(first)),
            Err(ConnectionIdentityError::ProtocolMismatch)
        );
        assert_eq!(
            validate_server_accept(first, &encode_server_accept(first)),
            Ok(())
        );
        assert_eq!(
            validate_server_accept(second, &encode_server_accept(first)),
            Err(ConnectionIdentityError::ProtocolMismatch)
        );
    }
}
