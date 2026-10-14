use crate::paths::socket_path;
use isle_core::{ClientMessage, ServerMessage, Session, Snapshot};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
const EVENT_BUFFER: usize = 64;

pub type Events = broadcast::Sender<ServerMessage>;

pub struct Server {
    socket: PathBuf,
    snapshot: Snapshot,
    events: Events,
    shutdown: tokio::sync::Notify,
    started_at: u64,
}

impl Server {
    pub fn new() -> Arc<Self> {
        Self::with_socket(socket_path())
    }

    pub fn with_socket(socket: PathBuf) -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Arc::new(Self {
            socket,
            snapshot: snapshot(),
            events,
            shutdown: tokio::sync::Notify::new(),
            started_at: isle_core::state::unix_now(),
        })
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn trigger_shutdown(&self) {
        self.shutdown.notify_one();
    }

    pub async fn serve(self: Arc<Self>) -> std::io::Result<()> {
        let path = self.socket().to_path_buf();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let _ = tokio::fs::remove_file(&path).await;

        let listener = UnixListener::bind(&path)?;
        tracing::info!(
            socket = %path.display(),
            compositor = ?self.snapshot.session.compositor,
            "listening"
        );

        let ticks = Arc::clone(&self);
        tokio::spawn(async move { ticks.tick_forever().await });

        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler is always constructible");

        tokio::select! {
            () = Arc::clone(&self).accept_loop(&listener) => {}
            _ = tokio::signal::ctrl_c() => tracing::info!("SIGINT received"),
            _ = sigterm.recv() => tracing::info!("SIGTERM received"),
            () = self.shutdown.notified() => tracing::info!("shutdown requested"),
        }

        let _ = tokio::fs::remove_file(&path).await;
        Ok(())
    }

    async fn accept_loop(self: Arc<Self>, listener: &UnixListener) {
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let conn = Arc::clone(&self);
                    tokio::spawn(async move { conn.handle_connection(stream).await });
                }
                Err(err) => {
                    tracing::warn!(%err, "accept failed");
                }
            }
        }
    }

    async fn tick_forever(&self) {
        let mut ticker = tokio::time::interval(TICK_INTERVAL);
        loop {
            ticker.tick().await;
            let uptime = isle_core::state::unix_now().saturating_sub(self.started_at);
            let _ = self.events.send(ServerMessage::event(
                "session.tick",
                serde_json::json!({ "uptime_secs": uptime }),
            ));
        }
    }

    async fn handle_connection(&self, stream: UnixStream) {
        match self.serve_connection(stream).await {
            Ok(()) => tracing::info!("client disconnected"),
            Err(err) if is_disconnect(&err) => tracing::info!("client disconnected"),
            Err(err) => tracing::warn!(%err, "connection ended"),
        }
    }

    async fn serve_connection(&self, stream: UnixStream) -> std::io::Result<()> {
        let (read_half, mut write_half) = stream.into_split();
        let mut events = self.events.subscribe();

        let snapshot = ServerMessage::State {
            data: self.snapshot.clone(),
        };
        write_half.write_all(snapshot.encode().as_bytes()).await?;

        let mut commands = BufReader::new(read_half).lines();
        loop {
            tokio::select! {
                line = commands.next_line() => {
                    let Some(line) = line? else { return Ok(()) };
                    if line.trim().is_empty() {
                        continue;
                    }
                    let reply = match serde_json::from_str::<ClientMessage>(&line) {
                        Ok(message) => self.dispatch(message),
                        Err(err) => ServerMessage::reply_err(0, format!("malformed command: {err}")),
                    };
                    write_half.write_all(reply.encode().as_bytes()).await?;
                }
                event = events.recv() => {
                    let message = match event {
                        Ok(message) => message,
                        Err(broadcast::error::RecvError::Lagged(missed)) => ServerMessage::event(
                            "session.lagged",
                            serde_json::json!({ "missed": missed }),
                        ),
                        Err(broadcast::error::RecvError::Closed) => return Ok(()),
                    };
                    write_half.write_all(message.encode().as_bytes()).await?;
                }
            }
        }
    }

    fn dispatch(&self, message: ClientMessage) -> ServerMessage {
        let ClientMessage::Cmd { id, name, args } = message;
        match name.as_str() {
            "ping" => ServerMessage::reply_ok(id),
            "view.set" => {
                let index = args
                    .get("index")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                let topic = serde_json::json!({ "index": index });
                let _ = self.events.send(ServerMessage::event("view.set", topic));
                ServerMessage::reply_ok(id)
            }
            "view.prev" | "view.next" => {
                let delta = if name == "view.next" { 1 } else { -1 };
                let _ = self.events.send(ServerMessage::event(
                    "view.step",
                    serde_json::json!({ "delta": delta }),
                ));
                ServerMessage::reply_ok(id)
            }
            "island.toggle" => {
                let _ = self
                    .events
                    .send(ServerMessage::event("island.toggle", serde_json::json!({})));
                ServerMessage::reply_ok(id)
            }
            "island.focus" => {
                let _ = self
                    .events
                    .send(ServerMessage::event("island.focus", serde_json::json!({})));
                ServerMessage::reply_ok(id)
            }
            other => ServerMessage::reply_err(id, format!("unknown command: {other}")),
        }
    }
}

use std::path::Path;

fn is_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::UnexpectedEof
    )
}

pub fn snapshot() -> Snapshot {
    Snapshot::new(
        Session {
            compositor: isle_core::Compositor::detect(),
            wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
            started_at_unix: isle_core::state::unix_now(),
        },
        isle_core::Features::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::time::{Duration, timeout};

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("isled-{name}-{}", std::process::id()))
    }

    #[test]
    fn ping_is_answered() {
        let server = Server::with_socket(PathBuf::from("/dev/null"));
        let reply = server.dispatch(ClientMessage::Cmd {
            id: 1,
            name: "ping".into(),
            args: serde_json::Value::Null,
        });
        assert_eq!(reply, ServerMessage::reply_ok(1));
    }

    #[test]
    fn unknown_commands_fail_loudly_rather_than_silently() {
        let server = Server::with_socket(PathBuf::from("/dev/null"));
        let reply = server.dispatch(ClientMessage::Cmd {
            id: 9,
            name: "volume.set".into(),
            args: serde_json::json!({ "level": 0.4 }),
        });
        assert_eq!(
            reply,
            ServerMessage::reply_err(9, "unknown command: volume.set")
        );
    }

    #[tokio::test]
    async fn a_client_gets_a_snapshot_then_replies() {
        let dir = scratch("handshake");
        let server = Server::with_socket(dir.join("isled.sock"));
        let path = server.socket().to_path_buf();

        let serving = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.serve().await }
        });
        wait_for_socket(&path).await;

        let mut client = UnixStream::connect(&path).await.unwrap();

        let mut buf = vec![0u8; 8192];
        let read = client.read(&mut buf).await.unwrap();
        let state: ServerMessage = serde_json::from_str(str_of(&buf, read).trim()).unwrap();
        assert!(matches!(state, ServerMessage::State { .. }));

        client
            .write_all(b"{\"t\":\"cmd\",\"id\":4,\"name\":\"ping\"}\n")
            .await
            .unwrap();
        let read = client.read(&mut buf).await.unwrap();
        let reply: ServerMessage = serde_json::from_str(str_of(&buf, read).trim()).unwrap();
        assert_eq!(reply, ServerMessage::reply_ok(4));

        drop(client);
        server.trigger_shutdown();
        timeout(Duration::from_secs(5), serving)
            .await
            .expect("server should stop promptly")
            .expect("server task should not panic")
            .expect("server should not error");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn shutdown_removes_the_socket() {
        let dir = scratch("cleanup");
        let server = Server::with_socket(dir.join("isled.sock"));
        let path = server.socket().to_path_buf();

        let serving = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.serve().await }
        });
        wait_for_socket(&path).await;
        assert!(path.exists(), "socket should exist while serving");

        server.trigger_shutdown();
        timeout(Duration::from_secs(5), serving)
            .await
            .expect("server should stop promptly")
            .expect("server task should not panic")
            .expect("server should not error");

        assert!(
            !path.exists(),
            "a leaked socket breaks the next systemd start"
        );
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn unsolicited_events_reach_a_connected_client() {
        let dir = scratch("events");
        let server = Server::with_socket(dir.join("isled.sock"));
        let path = server.socket().to_path_buf();

        let serving = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.serve().await }
        });
        wait_for_socket(&path).await;

        let client = UnixStream::connect(&path).await.unwrap();
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        assert!(matches!(
            serde_json::from_str::<ServerMessage>(line.trim()).unwrap(),
            ServerMessage::State { .. }
        ));

        line.clear();
        let tick = timeout(TICK_INTERVAL * 3, reader.read_line(&mut line))
            .await
            .expect("a tick should arrive without the client asking")
            .unwrap();
        assert_eq!(tick, line.len(), "expected a full line");
        assert!(
            matches!(
                serde_json::from_str::<ServerMessage>(line.trim()).unwrap(),
                ServerMessage::Event { ref topic, .. } if topic == "session.tick"
            ),
            "got: {line}"
        );

        server.trigger_shutdown();
        let _ = timeout(Duration::from_secs(5), serving).await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    async fn wait_for_socket(path: &Path) {
        timeout(Duration::from_secs(5), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("socket should appear");
    }

    fn str_of(buf: &[u8], len: usize) -> String {
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }
}
