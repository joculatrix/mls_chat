use std::{
    collections::HashMap,
    sync::Arc,
};
use crate::errors::ApplicationError;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpListener},
    sync::{mpsc::{self, Receiver, Sender}, Mutex},
};

pub struct Server {
    connections: Arc<Mutex<HashMap<usize, Connection>>>,
    size: usize,
    port: u16,
}

impl Server {
    pub fn new(port: u16, size: usize) -> Server {
        Server {
            connections: Arc::new(Mutex::new(HashMap::new())),
            size,
            port,
        }
    }

    /// Thread-spawning and main functionality loop for the server.
    /// 
    /// # Panics
    /// 
    /// This function panics if `TcpListener::bind()` returns an Err.
    /// 
    /// # TODO
    /// 
    /// Implement better shutdown/cleanup procedures.
    /// 
    /// Replace `unwrap()` with more robust error handling.
    pub async fn run(&mut self) -> Result<(), ApplicationError> {
        let address = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(address).await.unwrap();

        println!("Server opened on port {}.", self.port);

        let (tx, rx) = mpsc::channel(self.size);

        let mut handles = Vec::new();

        let connections = Arc::clone(&self.connections);
        handles.push(tokio::spawn(async move { Self::listen(listener, connections, tx).await }));

        let connections = Arc::clone(&self.connections);
        handles.push(tokio::spawn(async move { Self::handle_input(connections, rx).await }));
        
        for handle in handles {
            match handle.await {
                Ok(_) => (),
                Err(_) => return Err(ApplicationError::JoinError),
            }
        }
        Ok(())
    }

    /// Sends a received message to all open connections except the one from which it originated. Takes in the
    /// necessary `MessageIn` and a reference to the server's connections HashMap -- this is passed from
    /// `Server::handle_input()`, which calls this function and acquires the lock on the connections Mutex.
    /// 
    /// # TODO
    /// 
    /// Do something to handle errors when the writer fails.
    async fn fan_out(msg: MessageIn, connections: &mut HashMap<usize, Connection>) {
        for (k, v) in connections.iter_mut() {
            if *k != msg.id {
                match v.writer.write_all(&msg.contents).await {
                    Ok(_) => println!("Message sent from {} to {}.", msg.id, k),
                    Err(_) => (),
                }
            }
        }
    }

    /// Thread for handling incoming `MessageIn`s from each reader thread's `Sender`. Acquires the lock on the
    /// server's connections HashMap and passes the reference to `Server::fan_out()` to send the message out
    /// to all other connected clients.
    async fn handle_input(connections: Arc<Mutex<HashMap<usize, Connection>>>, mut rx: Receiver<MessageIn>) {
        while let Some(msg) = rx.recv().await {
            let mut connections = connections.lock().await;
            Self::fan_out(msg, &mut connections).await;
        }
    }

    /// Thread for listening to incoming connections. Takes in a `TcpListener`, a cloned Arc of the server's connections
    /// HashMap, and a `Sender<MessageIn>` to clone and give to each connection's thread. Also, maintains an iterative
    /// usize to give as a unique key for each incoming connection added to the HashMap.
    async fn listen(listener: TcpListener, connections: Arc<Mutex<HashMap<usize, Connection>>>, tx: Sender<MessageIn>) {
        let mut id_iter = 1;
    
        while let Ok((stream, _)) = listener.accept().await {
            let (reader, writer) = stream.into_split();

            let connection = Connection{ _approved: true, writer };
            let tx = tx.clone();
            connections.lock().await.insert(id_iter, connection);

            let connections = Arc::clone(&connections);
            tokio::spawn(async move {
                let id = id_iter;

                let address = reader.peer_addr().unwrap();
                println!("Listening on a connection accepted from {}.", address);
                match Self::read_stream(id, reader, tx).await {
                    Ok(_) => println!("Connection to {} closed successfully.", address),
                    Err(_) => println!("ERROR: Problem reading data received from {}. Connection closed unsuccessfully.", address),
                }
                connections.lock().await.remove(&id);
            });
    
            id_iter += 1;
        }
    }
    
    /// Handles reading the incoming data from the `OwnedReadHalf` of a split `TcpStream`. Takes in the id-number matching
    /// the key associated entry in the Server.connections HashMap, the `OwnedReadHalf`, and a `Sender<MessageIn>` to send
    /// messages to the server's `Receiver` to be fanned out to other connections. Returns an `Ok(())` when the thread
    /// successfully closes, or an `Err(ApplicationError::IOError)` on a failed read from the stream.
    /// 
    /// # Panics
    /// 
    /// Panics if `Sender::send()` returns an Err.
    /// 
    /// # TODO
    /// 
    /// Currently, either this function is misreading non-EOF input as EOF, or the client-side `Controller` is failing
    /// or sending on EOF erroneously. Urgently needs investigating.
    /// 
    /// Double-check `OwnedReadHalf`, `OwnedWriteHalf`, and `OwnedReadHalf::readable()`, and alternative solutions to ensure
    /// this function isn't holding a lock on the stream preventing the writer from writing to it while awaiting input.
    /// 
    /// Replace calls to `unwrap()` with more robust error handling.
    async fn read_stream(
        id: usize,
        mut reader: OwnedReadHalf,
        tx: Sender<MessageIn>,
    ) -> Result<(), ApplicationError> {
        loop {
            let Ok(()) = reader.readable().await else { return Err(ApplicationError::IOError) };

            let mut buf = vec![];
            let mut buf_reader = BufReader::new(&mut reader);
            match buf_reader.read_until(b'\n',&mut buf).await {
                Ok(0) => break, // EOF
                Ok(_) => tx.send(MessageIn{ id, contents: buf.to_owned() }).await.unwrap(),
                Err(_) => return Err(ApplicationError::IOError),
            }
        }

        Ok(())
    }
}

/// Holds information the `Server` needs access to about each open connection. Can be expanded
/// to provide more info or functionality. The `Connection` holds an `OwnedWriteHalf` to allow it access
/// to write to the connection while other threads hold the corresponding `OwnedReadHalf`.
struct Connection {
    /// "approved" field currently unused, but could be used in the future to facilitate
    /// approving/rejecting users before accepting them into the group
    _approved: bool,
    writer: OwnedWriteHalf,
}

/// The type used by the `Server` threads' `Sender`s and `Receiver`. The "id" refers to the
/// connection/thread the message was read by.
struct MessageIn {
    id: usize,
    contents: Vec<u8>,
}