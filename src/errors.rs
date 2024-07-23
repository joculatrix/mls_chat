use openmls::{group::AddMembersError, prelude::KeyPackageVerifyError};
use openmls_rust_crypto::MemoryKeyStore;

/// A type to encapsulate error types necessary to the program, for the convenience
/// of being able to pass ApplicationErrors between calling functions with '?' when
/// appropriate.
/// 
/// # TODO
/// 
/// Consider using these types as containers to hold lower-level error messages.
/// 
/// Also, reconsider which error types are appropriate, which need consolidation, and which need
/// to be more specific.
#[derive(Debug)]
pub enum ApplicationError {
    AddMemberError(AddMembersError<<MemoryKeyStore as openmls::prelude::OpenMlsKeyStore>::Error>),
    ConnectionFailed,
    CryptoError,
    GroupDNE, // if an operation is attempted on a nonexistent MlsGroup
    InvalidMessage,
    IOError,
    JoinError,
    KeyPackageDNE, // if the User has no key package
    KeyPackageVerify(KeyPackageVerifyError),
    KeyUpdateError,
    MlsKeyStoreError,
    ProcessMessageError(openmls::group::ProcessMessageError),
    TerminalError,
    TlsSerializeError,
}