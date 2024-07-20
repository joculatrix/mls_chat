use std::{ io::Result, net::IpAddr, process };
use clap::{Parser, Subcommand};
use mls_chat::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// host a chat server on this terminal
    Host {
        /// network port to host on
        #[arg(short, long)]
        port: u16,

        /// number of concurrent connections allowed on server
        #[arg(short, long)]
        size: usize,
    },

    /// connect to an existing server
    Join {
        /// IP address to connect to
        #[arg(short, long, value_name="ADDRESS")]
        target: IpAddr,

        /// network port to join on
        #[arg(short, long)]
        port: u16,

        /// user id to identify with
        #[arg(short, long)]
        id: String,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Host{ port , size } =>
            match host(port, size).await {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("Error: {}", err);
                    process::exit(1)
                }
            }
        Commands::Join{ target, port, id } =>
            join(target, port, id).await,
    }
}

async fn host(port: u16, size: usize) -> Result<()> {
    match server::listen(port, size).await {
        Ok(_) => println!("Server closed successfully."),
        Err(_) => todo!(),
    }

    Ok(())
}

async fn join(target: IpAddr, port: u16, id: String) {
    let mut address = String::new();
    address.push_str(&target.to_string());
    address.push_str(":");
    address.push_str(&port.to_string());

    if let Ok(mut controller) = Controller::build(address, id).await {
        controller.run().await.unwrap();
    } else {
        eprintln!("Unable to initialize controller.");
        process::exit(1);
    }
}