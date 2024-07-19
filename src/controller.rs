use crate::{
    errors::ApplicationError,
    network::client::Client,
    user::User,
    view::ChatWindow
};
use chrono::Utc;
use openmls::prelude::*;


pub struct Controller {
    log: Vec<String>,
    network: Client,
    user: User,
    window: ChatWindow,
}

impl Controller {
    pub async fn build(address: String, uid: String) -> Result<Controller, ApplicationError> {
        let network = Client::build(address).await?;
        let user = User::build(uid)?;

        Ok(Controller {
            log: Vec::new(),
            network,
            user,
            window: ChatWindow::build().unwrap(),
        })
    }

    /// The primary functionality loop for the client application. Continually updates the user interface
    /// with the log of messages sent, as well as sending messages input by the user and spawning/joining the
    /// network stream thread and pulling incoming messages to handle.
    /// 
    /// # TODO
    /// 
    /// Replace instances of `unwrap()` with more robust error handling.
    /// 
    /// Reconfigure to recover from/continue past non-fatal errors.
    pub async fn run(&mut self) -> Result<(), ApplicationError> {
        let kp = self.user.generate_key_package();
        self.serialize_and_send(kp).await?;

        let Ok(_network_handle) = self.network.handle_stream().await else { return Err(ApplicationError::IOError) };

        loop {
            self.window.draw(&self.log).unwrap();
            if !self.window.run()? {
                break;
            }

            match self.window.get_output() {
                Some(s) => {
                    if !s.is_empty() { self.send_chat_msg(s).await?; }
                }
                None => ()
            }      

            for msg in self.network.get_input().await {
                self.handle_messages(msg).await?;
            }
        }

        Ok(())
    }

    /// Helper function for `Controller::run()`. Deserializes and processes incoming messages, then executes
    /// the necessary tasks for each.
    /// 
    /// # Errors
    /// 
    /// Returns any `ApplicationError` types returned by `User::add_member()`, `User::update_keys()`,
    /// `User::process_message()`, or `Controller::serialize_and_send()`.
    /// 
    /// Could also return an `ApplicationError::InvalidMessage` if the input doesn't match any expected types.
    /// 
    /// # TODO
    /// 
    /// Suspected that MLS key packages must be deserialized as `KeyPackageIn::tls_deserialize()` rather than
    /// `MlsMessageIn::tls_deserialize()` extracted to an `MlsMessageInBody::KeyPackage`. Test this more thoroughly
    /// and refactor accordingly if any other types also can't be deserialized as `MlsMessageIn`.
    /// 
    /// Replace `unwrap()` with more robust error handling.
    async fn handle_messages(&mut self, msg: Vec<u8>) -> Result<(), ApplicationError> {
        if let Ok(msg) = MlsMessageIn::tls_deserialize(&mut msg.as_slice()) {
            match msg.extract() {
                MlsMessageInBody::Welcome(w) => {
                    if !self.user.has_group() {
                        self.user.join_group(w)?;
                        let msg = self.user.update_keys()?;
                        self.serialize_and_send(msg).await?;
                    }
                }
                MlsMessageInBody::KeyPackage(kp) => {
                    let (commit, welcome) = self.user.add_member(kp)?;
                    self.serialize_and_send(commit).await?;
                    self.serialize_and_send(welcome).await?;
                }
                MlsMessageInBody::GroupInfo(_) => (),
                MlsMessageInBody::PrivateMessage(msg) => {
                    let protocol_message = msg.into();
                    match self.user.process_message(protocol_message)? {
                        Some(msg) => self.log.push(String::from_utf8(msg).unwrap()),
                        None => (),
                    }
                }
                MlsMessageInBody::PublicMessage(msg) => {
                    let protocol_message = msg.into();
                    match self.user.process_message(protocol_message)? {
                        Some(msg) => self.log.push(String::from_utf8(msg).unwrap()),
                        None => (),
                    }
                }
            }

            Ok(())
        } else if let Ok(kp) = KeyPackageIn::tls_deserialize(&mut msg.as_slice()) {
            let (commit, welcome) = self.user.add_member(kp)?;
            self.serialize_and_send(commit).await?;
            self.serialize_and_send(welcome).await?;
            Ok(())
        }
        else { Err(ApplicationError::InvalidMessage) }
    }

    /// Helper function for `Controller::run()`. Takes the user's input text, adds a timestamp and username to the
    /// message as a prefix, encrypts it, and calls `Controller::serialize_and_send()`. Updates the user's key material
    /// after encryption as required by the MLS protocol, and sends the resulting key update message as well.
    /// 
    /// # Errors
    /// 
    /// Returns any `ApplicationError` types returned from `User::encrypt_message()`, `User::update_keys()`, and
    /// `Controller::serialize_and_send()`.
    async fn send_chat_msg(&mut self, msg: String) -> Result<(), ApplicationError> {
        let time = Utc::now().time().format("%H:%M:%S");
        let msg = format!("[{}] {}: {}", time, self.user.get_id(), msg);

        self.log.push(msg.clone());
        let msg = self.user.encrypt_message(&msg)?;
        self.serialize_and_send(msg).await?;

        let msg = self.user.update_keys()?;
        self.serialize_and_send(msg).await?;

        Ok(())
    }

    /// Helper function to remove repetition of the message serialize and send operations.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::TlsSerializeError` if `tls_serialize_detached()` fails.
    async fn serialize_and_send<T>(&mut self, msg: T) -> Result<(), ApplicationError> where T: TlsSerializeTrait  {
        if let Ok(msg) = msg.tls_serialize_detached() {
            self.network.send(msg).await;
            Ok(())
        } else {
            Err(ApplicationError::TlsSerializeError)
        }
    }
}