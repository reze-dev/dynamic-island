#![forbid(unsafe_code)]

mod notify;
mod paths;
mod server;
mod stats;

use isle_core::{ClientMessage, ServerMessage};
use paths::{Args, parse_args, parse_args_json, socket_path};
use std::process::ExitCode;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const USAGE: &str = "\
isled — session daemon for the Dynamic Island Shell

usage:
  isled                    run the daemon
  isled call <name> [args] send a command and print the reply
  isled socket-path        print the IPC socket path
";

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    match parse_args(std::env::args().skip(1)) {
        Args::Serve(claim_notifications) => {
            match tokio::spawn(server::Server::new(claim_notifications).serve()).await {
                Ok(Ok(())) => ExitCode::SUCCESS,
                Ok(Err(err)) => {
                    tracing::error!(%err, "server failed");
                    ExitCode::FAILURE
                }
                Err(err) => {
                    tracing::error!(%err, "server task panicked");
                    ExitCode::FAILURE
                }
            }
        }
        Args::Call { name, raw } => call(&name, &raw).await,
        Args::SocketPath(claim_notifications) => {
            if claim_notifications {
                // the socket path does not depend on the bus name, but accepting the flag here
                // keeps `isled --claim-notifications socket-path` from failing as "unknown"
            }

            println!("{}", socket_path().display());
            ExitCode::SUCCESS
        }
        Args::Help => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Args::Unknown(other) => {
            eprintln!("isled: unknown argument: {other}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

async fn call(name: &str, raw: &str) -> ExitCode {
    let command = ClientMessage::Cmd {
        id: 1,
        name: name.to_owned(),
        args: parse_args_json(raw),
    };
    let line = serde_json::to_string(&command).expect("ClientMessage is always serializable");

    let socket = socket_path();
    let stream = match UnixStream::connect(&socket).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("isled: cannot reach {}: {err}", socket.display());
            return ExitCode::FAILURE;
        }
    };

    let (read_half, mut write_half) = stream.into_split();
    if let Err(err) = write_half.write_all(format!("{line}\n").as_bytes()).await {
        eprintln!("isled: cannot send command: {err}");
        return ExitCode::FAILURE;
    }

    let mut lines = BufReader::new(read_half).lines();
    let mut reply = None;
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(message) = serde_json::from_str::<ServerMessage>(&line) else {
            continue;
        };
        if let ServerMessage::Reply { ok, error, .. } = message {
            reply = Some((ok, error));
            break;
        }
    }

    match reply {
        Some((true, _)) => ExitCode::SUCCESS,
        Some((false, Some(error))) => {
            eprintln!("isled: {error}");
            ExitCode::FAILURE
        }
        Some((false, None)) => {
            eprintln!("isled: command failed");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("isled: no reply");
            ExitCode::FAILURE
        }
    }
}
