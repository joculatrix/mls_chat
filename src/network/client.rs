use std::sync::Arc;
use crate::ApplicationError;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
    task::JoinHandle,
};

pub struct Client {
    input: Arc<Mutex<Vec<Vec<u8>>>>,
    output: Arc<Mutex<Vec<Vec<u8>>>>,
    stream: Option<TcpStream>,
}

impl Client {
    /// Builds a new `Client`. Takes in the IP address (as a `String`) of the `Server` to connect to.
    /// 
    /// # Error
    /// 
    /// Returns an `ApplicationError::ConnectionFailed` if `TcpStream::connect()` can't connect
    /// to the given address.
    pub async fn build(address: String) -> Result<Client, ApplicationError> {
        let input = Arc::new(Mutex::new(vec![]));
        let output = Arc::new(Mutex::new(vec![]));
        let Ok(stream) = TcpStream::connect(&address).await else {
            return Err(ApplicationError::ConnectionFailed);
        };

        Ok(Client {
            input,
            output,
            stream: Some(stream),
        })
    }

    /// Returns a `Vec` of all messages received from the stream since it was last drained.
    /// Removes the returned messages.
    pub async fn get_input(&mut self) -> Vec<Vec<u8>> {
        self.input.lock().await.drain(0..).collect()
    }


    /// Spawns a `tokio::task` to repeatedly send out any outgoing messages and read in incoming messages
    /// from the `Server`. Returns the `JoinHandle<()>` of the task.
    /// 
    /// # Errors
    /// 
    /// Returns an `ApplicationError::ConnectionFailed` if this method was called on a `Client` whose
    /// stream is None.
    /// 
    /// # TODO
    /// 
    /// Check if this function is erroneously sending an EOF byte when it shouldn't. Also, generally
    /// doesn't work. Rethink how to concurrently read and write on the stream.
    pub async fn handle_stream(&mut self) -> Result<JoinHandle<()>, ApplicationError> {
        let input = Arc::clone(&self.input);
        let output = Arc::clone(&self.output);
        let Some(mut stream) = self.stream.take() else { return Err(ApplicationError::ConnectionFailed) };

        Ok(tokio::spawn(async move { loop{
            while let Some(msg) = output.lock().await.pop() {
                match stream.write_all(&msg).await {
                    Ok(_) => (),
                    Err(_) => println!("Failed writing a message to the network stream."),
                }
            }

            let mut buf_reader = BufReader::new(&mut stream);
            let mut buffer = vec![];
            match buf_reader.read(&mut buffer).await {
                Ok(0) => { break; } // EOF
                Ok(_) => input.lock().await.push(buffer),
                _ => (),
            }
        }}))
    }

    /// Adds a newline to the end of a message, then pushes it onto the `Vec` of outgoing
    /// messages. The newline is added due to the `Server` using `AsyncBufReadExt::read_until()` to
    /// separate messages by newlines.
    pub async fn send(&mut self, mut msg: Vec<u8>) {
        msg.push(b'\n');
        self.output.lock().await.push(msg);
    }
}