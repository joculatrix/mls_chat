use errors::ApplicationError;
use openmls_rust_crypto::RustCrypto;
use super::*;

use openmls::{
    credentials::CredentialWithKey,
    group::{MlsGroup, MlsGroupConfig},
};
use openmls_traits::signatures::Signer;

pub struct Group {
    group: MlsGroup,
}

impl Group {
    /// Generates a new `MlsGroup` with the initiator's credentials.
    /// 
    /// # Panics
    /// 
    /// Panics if `MlsGroup::new()` fails.
    /// 
    /// # TODO
    /// 
    /// Replace `unwrap()` with more robust error handling.
    pub fn build_new(
        signer: &impl Signer,
        credential: CredentialWithKey
    )-> Group {
        let mls_group_config = MlsGroupConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

        Group {
            group: MlsGroup::new(
                &(*PROVIDER),
                signer,
                &mls_group_config,
                credential,
            ).unwrap(),
        }
    }

    /// Generates a new `MlsGroup` based on a `Welcome` message.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::KeyPackageDNE` if no `KeyPackage` can be found.
    pub fn build_join(welcome: Welcome) -> Result<Group, ApplicationError> {
        let config = MlsGroupConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

        if let Ok(group) = MlsGroup::new_from_welcome(
            &(*PROVIDER),
            &config,
            welcome,
            None) {
                Ok(Group { group })
        } else {
            Err(ApplicationError::KeyPackageDNE)
        }
    }

    /// Creates the necessary messages for adding a new member to the group. Returns a tuple
    /// `(MlsMessageOut, MlsMessageOut)` where the first is a Commit to be merged by the other members
    /// of the group, and the Welcome contains the information needed by the new member to calculate
    /// the necessary tree information on their machine.
    /// 
    /// Takes in the calling `User`'s `SignatureKeyPair` and the new member's `KeyPackageIn`.
    /// 
    /// # Errors
    /// 
    /// Returns an `AddMembersError` if `MlsGroup::add_members()` fails, or if `KeyPackageIn::validate()`
    /// returns that the key package can't be validated.
    pub fn add_member(
        &mut self,
        signer: &impl Signer,
        key_package: KeyPackageIn
    ) -> Result<(MlsMessageOut, MlsMessageOut), ApplicationError> {
        let Ok(key_package) = key_package.validate(&RustCrypto::default(), ProtocolVersion::default())
            else { return Err(ApplicationError::AddMemberError) };

        if let Ok((commit, welcome, _)) = self.group
            .add_members(&(*PROVIDER), signer, &[key_package]) {
                Ok((commit, welcome))
        } else { Err(ApplicationError::AddMemberError) }
    }

    /// Uses a `User`'s provided signature keys to encrypt a message. Returns an `MlsMessageOut`.
    /// 
    /// # Errors
    /// 
    /// Returns an Mls `CreateMessageError` if `MlsGroup::create_message()` fails.
    pub fn create_message(&mut self, signer: &impl Signer, msg: &str) -> Result<MlsMessageOut, CreateMessageError> {
        Ok(
            self.group
                .create_message(&(*PROVIDER), signer, msg.as_bytes())?
        )
    }

    /// Merges an incoming commit (such as a member being added to or removed from the group).
    /// 
    /// # Panics
    /// 
    /// Panics if `MlsGroup::merge_staged_commit()` fails.
    /// 
    /// # TODO
    /// 
    /// Replace `unwrap()` with more robust error handling.
    pub fn merge_commit(&mut self, commit: StagedCommit) {
        self.group
            .merge_staged_commit(&(*PROVIDER), commit)
            .unwrap();
    }

    /// Converts any MLS message with the `Into<ProtocolMessage>` into a `ProcessedMessage`.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::ProcessMessageError()` containing any errors returned by
    /// `MlsGroup::proces_message()`.
    pub fn process_message(&mut self, msg: impl Into<ProtocolMessage>) -> Result<ProcessedMessage, ApplicationError> {
        match self.group.process_message(&(*PROVIDER), msg.into()) {
            Ok(processed_message) => Ok(processed_message),
            Err(err) => Err(ApplicationError::ProcessMessageError(err))
        }
    }

    /// Returns a commit `MlsMessageOut` to remove a specified member from the group.
    /// 
    /// # Panics
    /// 
    /// Panics if `MlsGroup::remove_members()` fails.
    /// 
    /// # TODO
    /// 
    /// Replace `unwrap()` with more robust error handling.
    pub fn remove_member(&mut self, signer: &impl Signer, member_index: u32) -> MlsMessageOut {
        let member_index = LeafNodeIndex::new(member_index);
        
        let (commit, _, _) = self.group
            .remove_members(&(*PROVIDER), signer, &[member_index])
            .unwrap();

        commit
    }

    /// Returns a commit `MlsMessageOut` to update the sender's key package.
    /// 
    /// # Panics
    /// 
    /// Panics if `MlsGroup::self_update()` fails.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::MlsKeyStoreError` if `MlsGroup::merge_pending_commit()` fails. Returns
    /// an `ApplicationError::KeyUpdateError` if `MlsGroup::self_update()` fails.
    /// 
    /// # TODO
    /// 
    /// Replace `unwrap()` with more robust error handling.
    pub fn update_keys(&mut self, signer: &impl Signer) -> Result<MlsMessageOut, ApplicationError> {
        let Ok(_) = self.group.merge_pending_commit(&(*PROVIDER)) else { return Err(ApplicationError::MlsKeyStoreError) };

        if let Ok((msg, _, _)) = self.group.self_update(&(*PROVIDER), signer) {
            Ok(msg)
        } else {
            Err(ApplicationError::KeyUpdateError)
        }
    }
}