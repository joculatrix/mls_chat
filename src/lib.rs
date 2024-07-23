use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;

// prelude for easy use in main:
pub use crate::controller::Controller;
pub use crate::network::server::Server;
pub use crate::errors::ApplicationError;
pub use crate::user::User;

#[macro_use]
extern crate lazy_static;

// constants for use in the group and user mods:
static CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519;
lazy_static!( static ref PROVIDER: OpenMlsRustCrypto = OpenMlsRustCrypto::default(); );


pub mod controller;
pub mod errors;
pub mod group;
pub mod network;
pub mod user;
pub mod view;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_key_package() {
        let mut bob = User::build(String::from("bob")).unwrap();
        let key_package = bob.generate_key_package();
        let key_package = key_package.tls_serialize_detached();

        assert!(key_package.is_ok(), "Key package returns error: {:?}", key_package);
    }

    #[test]
    fn deserialize_key_package() {
        let mut bob = User::build(String::from("bob")).unwrap();
        let key_package = bob.generate_key_package();
        let key_package = key_package.tls_serialize_detached().unwrap();
        let key_package = KeyPackageIn::tls_deserialize(&mut key_package.as_slice());

        assert!(key_package.is_ok(), "Key package returns error: {:?}", key_package);
    }

    #[test]
    fn update_keys() {
        let mut bob = User::build(String::from("bob")).unwrap();
        let _key_package = bob.generate_key_package();
        let update = bob.update_keys();

        assert!(update.is_ok(), "Key update returns error: {:?}", update);
    }

    #[test]
    fn join_from_welcome() {
        let mut bob = User::build(String::from("bob")).unwrap();
        let key_package = KeyPackageIn::tls_deserialize(&mut
            (bob.generate_key_package()
                .tls_serialize_detached()
                .unwrap())
                .as_slice())
                .unwrap();
        let mut alice = User::build(String::from("alice")).unwrap();
        let res = alice.add_member(key_package);

        assert!(res.is_ok(), "add_member returns error: {:?}", res);

        let (_commit, welcome) = res.unwrap();
        let welcome = Welcome::tls_deserialize(&mut
            (welcome.tls_serialize_detached()
            .unwrap())
            .as_slice());
        
        assert!(welcome.is_ok(), "Welcome::tls_deserialize returns error: {:?}", welcome);

        let welcome = welcome.unwrap();
        let res = bob.join_group(welcome);

        assert!(res.is_ok(), "join_group returns error: {:?}", res);
    }
}