use crate::errors::ApplicationError;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    signal,
    sync::broadcast::{channel, error::RecvError, Sender},
};
use tokio_util::sync::CancellationToken;

type Result<T> = std::result::Result<T, ApplicationError>;

pub async fn listen(port: u16, size: usize) -> Result<()> {
    let address = format!("127.0.0.1:{}", port);
    let Ok(listener) = TcpListener::bind(address).await else {
        return Err(ApplicationError::ConnectionFailed)
    };

    let (tx, _) = channel(size);
    let cancel_token = CancellationToken::new();
    let mut handles = vec![];
    let mut id: usize = 0;

    tokio::select! {
        Ok((mut stream, address)) = listener.accept() => {
            let tx = tx.clone();
            let cancel_token = cancel_token.clone();
            handles.push(tokio::spawn(async move { read_stream(id, stream, tx, cancel_token) }));
            id += 1;
        },
        Err(_) = listener.accept() => {
            cancel_token.cancel();
            for handle in handles {
                handle.await;
            }
        },
        _ = signal::ctrl_c() => {
            cancel_token.cancel();
            for handle in handles {
                handle.await;
            }
        }
    }

    Ok(())
}

async fn read_stream(
    id: usize,
    mut stream: TcpStream,
    tx: Sender<Message>,
    cancel: CancellationToken,
) {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut rx = tx.subscribe();

    loop {
        let mut buf = vec![];

        tokio::select! {
            msg = buf_reader.read_until(b'\n', &mut buf) => {
                match msg {
                    Ok(0) => { // EOF
                        println!("Connection {} closed due to remote disconnect.", id);
                        break;
                    }
                    Ok(_) => {
                        match tx.send(Message{ content: buf }) {
                            Ok(n) => println!("Message from {} sent to {} receivers.", id, n),
                            Err(_) => println!("Message from {} not send to any receivers.", id),
                        }
                    }
                    Err(_) => println!("Unable to read from stream {}.", id),
                }
            },
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => writer.write_all(&msg.content).await.unwrap(),
                    Err(RecvError::Closed) => {
                        println!("No active senders. Channel closed.");
                        break;
                    }
                    Err(RecvError::Lagged(n)) => {
                        println!("Receiver {} lagged behind by {} messages.", id, n);
                    }
                }
            },
            _ = cancel.cancelled() => {
                break;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Message {
    // id: usize,
    content: Vec<u8>,
}