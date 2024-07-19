use errors::ApplicationError;

use super::*;
use crate::group::Group;

pub struct User {
    id: String,
    credential_with_key: Option<CredentialWithKey>,
    signer: SignatureKeyPair,
    group: Option<Group>,
}

impl User {
    /// Builds a new `User`, taking in an id/username `String`.
    /// 
    /// # Errors
    /// 
    /// Returns any `ApplicationError`s returned by `User::generate_credential()`.
    pub fn build(id: String) -> Result<User, ApplicationError> {
        let (credential_with_key, signer) =
            Self::generate_credential(id.clone().into_bytes(), CredentialType::Basic)?;

        let mut user = User {
            id,
            credential_with_key: Some(credential_with_key),
            signer,
            group: None,
        };

        user.generate_group();
        let (credential_with_key, signer) =
            Self::generate_credential(user.id.clone().into_bytes(), CredentialType::Basic)?;
        user.credential_with_key = Some(credential_with_key);
        user.signer = signer;

        Ok(user)
    }

    /// Used as a helper for `User::build()`, or to update key material after it's used to encrypt a message. 
    /// Generates a `CredentialWithKey` and stores the intermediary `SignatureKeyPair` into the provider's key store.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::CryptoError` if `SignatureKeyPair::new()` fails, or an `ApplicationError::MlsKeyStoreError` 
    /// if `SignatureKeyPair::store()` fails.
    fn generate_credential(
        identity: Vec<u8>,
        credential_type: CredentialType,
    ) -> Result<(CredentialWithKey, SignatureKeyPair), ApplicationError> {
        let credential = Credential::new(identity, credential_type).expect("Hardcoded credential type should be supported.");
        let Ok(signature_keys) = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm()) else {
            return Err(ApplicationError::CryptoError);
        };

        match signature_keys.store((*PROVIDER).key_store()) {
            Ok(_) => (),
            Err(_) => return Err(ApplicationError::MlsKeyStoreError),
        }

        Ok((
            CredentialWithKey {
                credential: credential.into(),
                signature_key: signature_keys.public().into(),
            },
            signature_keys,
        ))
    }

    /// Returns an `Ok(MlsMessageOut, MlsMessageOut)`, with the first being a Commit to send to existing members of the group
    /// and the second being a Welcome for the new member. Takes in the `KeyPackageIn` corresponding to the new member.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::GroupDNE` if the `User`'s group is None, or an `ApplicationError::AddMembersError` if
    /// returned by `Group::add_member()`.
    pub fn add_member(&mut self, key_package: KeyPackageIn) -> Result<(MlsMessageOut, MlsMessageOut), ApplicationError> {
        if let Some(ref mut group) = self.group {
            Ok(group.add_member(&self.signer, key_package)?)
        } else { Err(ApplicationError::GroupDNE) }
    }

    /// Uses the user's key material to encrypt a plaintext message. Returns an `Ok(MlsMessageOut)` if successful.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::GroupDNE` on failure.
    /// 
    /// # TODO
    /// 
    /// Review error types, refactor to cover other error causes if needed.
    pub fn encrypt_message(&mut self, msg: &str) -> Result<MlsMessageOut, ApplicationError> {
        match &mut self.group {
            Some(g) =>
                match g.create_message(&self.signer, msg) {
                    Ok(result) => Ok(result),
                    Err(_) => Err(ApplicationError::GroupDNE),
                }
            None => Err(ApplicationError::GroupDNE),
        }
    }

    /// Generates and returns a user's `KeyPackage` from their `SignatureKeyPair` and `CredentialWithKey`.
    /// Takes ownership of the data within the user's `CredentialWithKey` and replaces it with None.
    /// 
    /// # Panics
    /// 
    /// Panics if the `KeyPackageBuilder::build()` returns an error, or if the user doesn't have a `CredentialWithKey`.
    /// 
    /// # TODO
    /// 
    /// Replace instances of `unwrap()` with more robust error handling.
    pub fn generate_key_package(
        &mut self,
    ) -> KeyPackage {
        KeyPackage::builder()
            .build(
                CryptoConfig::with_default_version(CIPHERSUITE),
                &(*PROVIDER),
                &self.signer,
                self.credential_with_key.take().unwrap(),
            ).unwrap()
    }

    /// Generates a new `MlsGroup` (with the user as the initiator).
    /// Takes ownership of the data within the user's `CredentialWithKey` and replaces it with None.
    /// 
    /// # Panics
    /// 
    /// Panics if the user doesn't currently have a `CredentialWithKey`, or if `MlsGroup::new()` fails.
    /// 
    /// # TODO
    /// 
    /// Replace instances of `unwrap()` with more robust error handling.
    pub fn generate_group(&mut self) {
        self.group = Some(
            Group::build_new(
                &self.signer,
                self.credential_with_key.to_owned().unwrap()
            )
        );
    }

    /// Returns true if the User's group is Some() or false if it's None.
    pub fn has_group(&self) -> bool {
        match &self.group {
            Some(_) => true,
            None => false
        }
    }

    /// Returns the User's ID string.
    pub fn get_id(&self) -> &String {
        &self.id
    }

    /// Sets the user's group to one created from a Welcome message.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::KeyPackageDNE` if no `KeyPackage` can be found.
    pub fn join_group(&mut self, welcome: Welcome) -> Result<(), ApplicationError> {
        if let Ok(group) = Group::build_join(welcome) {
            self.group = Some(group);
            Ok(())
        } else {
            Err(ApplicationError::KeyPackageDNE)
        }
    }

    /// Processes a `ProtocolMessage`. If it's an `ApplicationMessage`, returns an `Ok(Some(Vec<u8>))` with the decrypted message.
    /// Otherwise, returns an `Ok(None)` if successful.
    /// 
    /// # Errors
    /// 
    /// Returns a `ProcessMessageError(err)` or a `GroupDNE` error on failure.
    pub fn process_message(&mut self, msg: ProtocolMessage) -> Result<Option<Vec<u8>>, ApplicationError> {
        if let Some(ref mut group) = self.group {
            let processed_message = group.process_message(msg)?;
            match processed_message.into_content() {
                ProcessedMessageContent::ApplicationMessage(app_msg) => Ok(Some(app_msg.into_bytes())),
                ProcessedMessageContent::StagedCommitMessage(commit) => {
                    group.merge_commit(*commit);
                    Ok(None)
                }
                _ => Ok(None), // application isn't currently built to send the other remaining message content types in any scenario
            }
        } else { Err(ApplicationError::GroupDNE) }
    }

    /// Updates a `User`'s key material and returns an `Ok(MlsMessageOut)` with the resulting update message
    /// to be sent to other members of the group.
    /// 
    /// # Errors
    /// 
    /// Retuns an `ApplicationError::GroupDNE` if called on a `User` whose group is None.
    pub fn update_keys(&mut self) -> Result<MlsMessageOut, ApplicationError> {
        let (credential_with_key, signer) =
            Self::generate_credential(self.id.clone().into_bytes(), CredentialType::Basic)?;
        self.credential_with_key = Some(credential_with_key);
        self.signer = signer;

        if let Some(ref mut group) = self.group {
            Ok(group.update_keys(&self.signer)?)
        } else {
            Err(ApplicationError::GroupDNE)
        }
    } 
}