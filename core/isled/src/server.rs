use crate::domain::Domain;
use crate::notify::{self, Hub, NotifyRequest};
use crate::paths::socket_path;
use crate::settings::Settings;
use crate::stats::Sampler;
use crate::store::Store;
use isle_core::live::{Activity, ActivityKind, ActivityTone};
use isle_core::state::{CloseReason, unix_now, unix_now_ms};
use isle_core::{ClientMessage, ServerMessage, Session, Snapshot, SystemStats};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
const PULSE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
const EVENT_BUFFER: usize = 64;

pub type Events = broadcast::Sender<ServerMessage>;

pub struct Server {
    socket: PathBuf,
    snapshot: Snapshot,
    stats: tokio::sync::Mutex<Sampler>,
    events: Events,
    notifications: Arc<Hub>,
    claim_notifications: bool,
    shutdown: tokio::sync::Notify,
    domain: tokio::sync::Mutex<Domain>,
    /// Whether NetworkManager is on the bus. Worked out once, on the first
    /// client that connects.
    network_manager: tokio::sync::OnceCell<bool>,
}

impl Server {
    pub fn new(claim_notifications: bool) -> Arc<Self> {
        Self::with_socket_and_hub(socket_path(), claim_notifications)
    }

    pub fn with_socket_and_hub(socket: PathBuf, claim_notifications: bool) -> Arc<Self> {
        Self::with_domain(socket, claim_notifications, Domain::load(Store::new()))
    }

    /// Build a server around an explicit domain. The tests use this so they
    /// never touch the real `~/.local/state/isled`.
    pub fn with_domain(socket: PathBuf, claim_notifications: bool, domain: Domain) -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let hub = Hub::detached(events.clone());
        if claim_notifications {
            notify::spawn_expiry_sweep(Arc::clone(&hub));
        }
        Arc::new(Self {
            socket,
            snapshot: snapshot(),
            stats: tokio::sync::Mutex::new(Sampler::new()),
            events,
            notifications: hub,
            claim_notifications,
            shutdown: tokio::sync::Notify::new(),
            domain: tokio::sync::Mutex::new(domain),
            network_manager: tokio::sync::OnceCell::new(),
        })
    }

    async fn snapshot_with_notifications(&self) -> Snapshot {
        let mut snapshot = self.snapshot.clone();
        snapshot.features.notifications = self.notifications.is_attached();
        let network_manager = *self
            .network_manager
            .get_or_init(crate::network::present)
            .await;
        snapshot.features.network_manager = network_manager;
        snapshot.notifications = self.notifications.list().await;
        let domain = self.domain.lock().await;
        snapshot =
            snapshot.with_activities(domain.activities.ambient(), domain.activities.recent());
        snapshot
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

        if self.claim_notifications {
            match notify::claim(&self.notifications).await {
                Ok(_connection) => {
                    tracing::info!("notifications are being served by this daemon");
                }
                Err(err) => tracing::warn!(
                    %err,
                    "could not claim {}; continuing without the bus name",
                    notify::NOTIFICATIONS_BUS_NAME
                ),
            }
        }

        let ticks = Arc::clone(&self);
        tokio::spawn(async move { ticks.tick_forever().await });

        let pulse = Arc::clone(&self);
        tokio::spawn(async move { pulse.pulse_forever().await });

        let watcher = Arc::clone(&self);
        tokio::spawn(async move { watcher.watch_notifications().await });

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
            let stats = self.sample().await;
            let _ = self.events.send(ServerMessage::event(
                "system.stats",
                serde_json::to_value(&stats).unwrap_or_default(),
            ));
        }
    }

    /// The fast loop. Everything that has a deadline in it lives here: the
    /// pomodoro, the activity dwell times, the wallpaper rotation and the
    /// check for a todo that has come due.
    async fn pulse_forever(&self) {
        let mut ticker = tokio::time::interval(PULSE_INTERVAL);
        let mut last_due_todo: Option<u64> = None;
        loop {
            ticker.tick().await;
            self.pulse(&mut last_due_todo).await;
        }
    }

    /// One pass of the fast loop.
    async fn pulse(&self, last_due_todo: &mut Option<u64>) {
        let now_unix = unix_now();
        let now_ms = unix_now_ms();

        let (timer_event, due_todo, wallpaper_event, activities_changed) = {
            let mut domain = self.domain.lock().await;

            // The pomodoro.
            let mut timer_event = None;
            if let Some(phase) = domain.timer.advance_if_elapsed(now_ms, now_unix) {
                domain.save_timer();
                timer_event = Some(phase);
            }
            if domain.timer.running {
                let payload = timer_payload(&domain.timer, now_ms);
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.tick", payload));
            }

            // A todo that has come due.
            let due = domain
                .todos
                .next_due(now_unix)
                .filter(|t| *last_due_todo != Some(t.id))
                .map(|t| (t.id, t.text.clone()));
            if let Some((id, _)) = due {
                *last_due_todo = Some(id);
            }

            // The wallpaper rotation.
            let mut wallpaper_event = None;
            if domain.settings.wallpaper_loop {
                let elapsed = now_unix.saturating_sub(domain.wallpaper.last_changed_unix);
                if elapsed >= domain.settings.wallpaper_loop_secs {
                    let pick = domain
                        .wallpaper
                        .random_excluding(domain.wallpaper.current.as_deref())
                        .map(|w| w.path.clone());
                    if let Some(path) = pick {
                        let engine = domain.wallpaper.engine;
                        match engine.apply(Path::new(&path)) {
                            Ok(()) => {
                                domain.wallpaper.current = Some(path.clone());
                                domain.wallpaper.last_changed_unix = now_unix;
                                wallpaper_event = Some(path);
                            }
                            Err(err) => {
                                tracing::warn!(%err, "the wallpaper engine refused an image")
                            }
                        }
                    }
                }
            }

            // Activities that have outlived their dwell.
            let expired = domain.activities.expire(now_unix);

            (
                timer_event,
                due.map(|d| d.1),
                wallpaper_event,
                !expired.is_empty(),
            )
        };

        if let Some(phase) = timer_event {
            let next = if phase.is_break() {
                "focus next"
            } else {
                "break next"
            };
            self.trigger_activity(
                ActivityKind::Timer,
                format!("{} finished", phase.label()),
                format!("{next} · {} rounds", domain_rounds(self).await),
                ActivityTone::Good,
                "timer",
                6_000,
                true,
                Some("timer".to_string()),
            )
            .await;
        }

        if let Some(text) = due_todo {
            self.trigger_activity(
                ActivityKind::Todo,
                text,
                "todo is due".to_string(),
                ActivityTone::Warn,
                "check",
                8_000,
                false,
                None,
            )
            .await;
        }

        if let Some(path) = wallpaper_event {
            let _ = self.events.send(ServerMessage::event(
                "wallpaper.changed",
                serde_json::json!({ "path": path }),
            ));
        }

        // Only say something when something changed. A chatty daemon is a
        // daemon that keeps the UI awake for no reason.
        if activities_changed {
            self.broadcast_activity_state().await;
        }
    }

    /// React to notifications arriving on the bus by turning them into
    /// island activities.
    async fn watch_notifications(&self) {
        let mut events = self.events.subscribe();
        loop {
            match events.recv().await {
                Ok(ServerMessage::Event { topic, data }) if topic == "notifications.added" => {
                    let Some(notification) =
                        serde_json::from_value::<isle_core::state::Notification>(data).ok()
                    else {
                        continue;
                    };
                    let tone = match notification.urgency_level() {
                        2 => ActivityTone::Critical,
                        1 => ActivityTone::Warn,
                        _ => ActivityTone::Neutral,
                    };
                    let title = if notification.summary.trim().is_empty() {
                        notification.app_name.clone()
                    } else {
                        notification.summary.clone()
                    };
                    self.trigger_activity(
                        ActivityKind::Notification,
                        title,
                        notification.app_name.clone(),
                        tone,
                        "bell",
                        5_000,
                        false,
                        Some(notification.id.to_string()),
                    )
                    .await;
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    async fn sample(&self) -> SystemStats {
        self.stats.lock().await.sample()
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
            data: self.snapshot_with_notifications().await,
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
                        Ok(message) => self.dispatch(message).await,
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

    async fn notifications_send(&self, id: u64, args: serde_json::Value) -> ServerMessage {
        let request = NotifyRequest {
            app_name: arg_string(args.get("app_name"), "isled"),
            replaces_id: args
                .get("replaces_id")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32,
            app_icon: arg_string(args.get("app_icon"), "unset"),
            summary: arg_string(args.get("summary"), ""),
            body: arg_string(args.get("body"), ""),
            actions: arg_actions(args.get("actions")),
            hints: arg_hints(args.get("hints")),
            expire_timeout: args
                .get("expire_timeout")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i32,
        };
        let stored = self.notifications.ingest(request).await;
        ServerMessage::reply_with(id, serde_json::json!({ "id": stored.id }))
    }

    /// Push an activity into the queue and tell the island about it.
    ///
    /// This is the single funnel every trigger goes through, whether it came
    /// from a notification on the bus, a todo coming due, the timer changing
    /// phase, or the UI pushing something it knows about (a media player, for
    /// instance).
    #[allow(clippy::too_many_arguments)]
    async fn trigger_activity(
        &self,
        kind: ActivityKind,
        title: String,
        subtitle: String,
        tone: ActivityTone,
        glyph: &str,
        dwell_ms: u64,
        sticky: bool,
        source: Option<String>,
    ) -> u64 {
        let mut activity = Activity::new(kind, title, unix_now())
            .with_subtitle(subtitle)
            .with_tone(tone)
            .with_glyph(glyph)
            .with_dwell_ms(dwell_ms);
        if sticky {
            activity = activity.sticky();
        }
        if let Some(source) = source {
            activity = activity.with_source(source);
        }

        let id = {
            let mut domain = self.domain.lock().await;
            domain.activities.push(activity.clone())
        };

        let _ = self.events.send(ServerMessage::event(
            "activity.triggered",
            serde_json::to_value(&activity).unwrap_or_default(),
        ));
        self.broadcast_activity_state().await;
        id
    }

    /// Publish the ambient activity and the recent list.
    async fn broadcast_activity_state(&self) {
        let payload = {
            let domain = self.domain.lock().await;
            serde_json::json!({
                "ambient": domain.activities.ambient(),
                "recent": domain.activities.recent(),
            })
        };
        let _ = self
            .events
            .send(ServerMessage::event("activity.changed", payload));
    }

    async fn dispatch(&self, message: ClientMessage) -> ServerMessage {
        let ClientMessage::Cmd { id, name, args } = message;
        match name.as_str() {
            "ping" => ServerMessage::reply_ok(id),
            "system.stats" => {
                let stats = self.sample().await;
                let reply = ServerMessage::reply_ok(id);
                let _ = self.events.send(ServerMessage::event(
                    "system.stats",
                    serde_json::to_value(&stats).unwrap_or_default(),
                ));
                reply
            }
            "view.set" => {
                // The index may arrive as {"index": 3} or as a bare 3. The
                // bare form exists so a compositor keybind can be one
                // argument with no quoting to get wrong.
                let index = match &args {
                    serde_json::Value::Number(number) => number.as_i64().unwrap_or(0),
                    serde_json::Value::Object(_) => args
                        .get("index")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    _ => 0,
                };
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
            "notifications.dismiss" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                match self
                    .notifications
                    .dismiss(target as u32, CloseReason::Dismissed)
                    .await
                {
                    Some(_) => ServerMessage::reply_ok(id),
                    None => ServerMessage::reply_err(id, format!("no notification {target}")),
                }
            }
            "notifications.clear" => {
                let count = self.notifications.dismiss_all().await;
                ServerMessage::reply_with(id, serde_json::json!({ "cleared": count }))
            }
            "notifications.action" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let key = args
                    .get("key")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("default");
                self.notifications.invoke_action(target as u32, key).await;
                ServerMessage::reply_ok(id)
            }
            "notifications.send" => self.notifications_send(id, args).await,

            "activity.list" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({
                        "ambient": domain.activities.ambient(),
                        "recent": domain.activities.recent(),
                    }),
                )
            }
            "activity.trigger" => self.activity_trigger(id, args).await,
            "activity.dismiss" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let removed = {
                    let mut domain = self.domain.lock().await;
                    domain.activities.dismiss(target)
                };
                if removed {
                    self.broadcast_activity_state().await;
                    ServerMessage::reply_ok(id)
                } else {
                    ServerMessage::reply_err(id, format!("no activity {target}"))
                }
            }
            "activity.clear" => {
                {
                    let mut domain = self.domain.lock().await;
                    domain.activities.clear();
                }
                self.broadcast_activity_state().await;
                ServerMessage::reply_ok(id)
            }

            "todos.list" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({ "items": domain.todos.ordered() }),
                )
            }
            "todos.add" => {
                let text = arg_string(args.get("text"), "");
                if text.trim().is_empty() {
                    return ServerMessage::reply_err(id, "a todo needs some text");
                }
                let now = unix_now();
                let due = args
                    .get("due_unix")
                    .and_then(serde_json::Value::as_u64)
                    .or_else(|| {
                        args.get("due_in_minutes")
                            .and_then(serde_json::Value::as_u64)
                            .map(|m| now + m * 60)
                    });
                let priority = args
                    .get("priority")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u8;
                let todo_id = {
                    let mut domain = self.domain.lock().await;
                    let id = domain.todos.add(text, due, priority, now);
                    domain.save_todos();
                    id
                };
                let _ = self.events.send(ServerMessage::event(
                    "todos.changed",
                    serde_json::json!({ "id": todo_id }),
                ));
                ServerMessage::reply_with(id, serde_json::json!({ "id": todo_id }))
            }
            "todos.toggle" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let result = {
                    let mut domain = self.domain.lock().await;
                    let done = domain.todos.toggle(target);
                    domain.save_todos();
                    done
                };
                match result {
                    Some(done) => {
                        let _ = self.events.send(ServerMessage::event(
                            "todos.changed",
                            serde_json::json!({ "id": target, "done": done }),
                        ));
                        ServerMessage::reply_with(id, serde_json::json!({ "done": done }))
                    }
                    None => ServerMessage::reply_err(id, format!("no todo {target}")),
                }
            }
            "todos.remove" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let removed = {
                    let mut domain = self.domain.lock().await;
                    let removed = domain.todos.remove(target);
                    domain.save_todos();
                    removed
                };
                if removed {
                    let _ = self.events.send(ServerMessage::event(
                        "todos.changed",
                        serde_json::json!({ "id": target, "removed": true }),
                    ));
                    ServerMessage::reply_ok(id)
                } else {
                    ServerMessage::reply_err(id, format!("no todo {target}"))
                }
            }
            "todos.reorder" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let to = args
                    .get("to")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as usize;
                let moved = {
                    let mut domain = self.domain.lock().await;
                    let moved = domain.todos.reorder(target, to);
                    domain.save_todos();
                    moved
                };
                if moved {
                    let _ = self.events.send(ServerMessage::event(
                        "todos.changed",
                        serde_json::json!({ "id": target, "moved": true }),
                    ));
                    ServerMessage::reply_ok(id)
                } else {
                    ServerMessage::reply_err(id, format!("no todo {target}"))
                }
            }
            "todos.clear_done" => {
                let cleared = {
                    let mut domain = self.domain.lock().await;
                    let cleared = domain.todos.clear_done();
                    domain.save_todos();
                    cleared
                };
                let _ = self.events.send(ServerMessage::event(
                    "todos.changed",
                    serde_json::json!({ "cleared": cleared }),
                ));
                ServerMessage::reply_with(id, serde_json::json!({ "cleared": cleared }))
            }

            "timer.status" => {
                let domain = self.domain.lock().await;
                let now = unix_now_ms();
                ServerMessage::reply_with(id, timer_payload(&domain.timer, now))
            }
            "timer.start" => {
                let minutes = args
                    .get("minutes")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32;
                {
                    let mut domain = self.domain.lock().await;
                    if minutes > 0 {
                        domain.timer.work_minutes = minutes;
                    }
                    domain.timer.start(unix_now_ms(), unix_now());
                    domain.save_timer();
                }
                let payload = {
                    let domain = self.domain.lock().await;
                    timer_payload(&domain.timer, unix_now_ms())
                };
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.changed", payload));
                ServerMessage::reply_ok(id)
            }
            "timer.pause" => {
                {
                    let mut domain = self.domain.lock().await;
                    domain.timer.pause(unix_now_ms());
                    domain.save_timer();
                }
                let payload = {
                    let domain = self.domain.lock().await;
                    timer_payload(&domain.timer, unix_now_ms())
                };
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.changed", payload));
                ServerMessage::reply_ok(id)
            }
            "timer.resume" => {
                {
                    let mut domain = self.domain.lock().await;
                    domain.timer.resume(unix_now_ms());
                    domain.save_timer();
                }
                let payload = {
                    let domain = self.domain.lock().await;
                    timer_payload(&domain.timer, unix_now_ms())
                };
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.changed", payload));
                ServerMessage::reply_ok(id)
            }
            "timer.stop" => {
                {
                    let mut domain = self.domain.lock().await;
                    domain.timer.stop();
                    domain.save_timer();
                }
                let payload = {
                    let domain = self.domain.lock().await;
                    timer_payload(&domain.timer, unix_now_ms())
                };
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.changed", payload));
                ServerMessage::reply_ok(id)
            }
            "timer.durations" => {
                let work = args
                    .get("work")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32;
                let short_break = args
                    .get("short_break")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32;
                let long_break = args
                    .get("long_break")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as u32;
                {
                    let mut domain = self.domain.lock().await;
                    domain.timer.set_durations(work, short_break, long_break);
                    domain.save_timer();
                }
                let payload = {
                    let domain = self.domain.lock().await;
                    timer_payload(&domain.timer, unix_now_ms())
                };
                let _ = self
                    .events
                    .send(ServerMessage::event("timer.changed", payload));
                ServerMessage::reply_ok(id)
            }

            "notes.list" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({
                        "items": domain.notes.list(),
                        "dir": domain.notes.dir().to_string_lossy(),
                    }),
                )
            }
            "notes.get" => {
                let key = arg_string(args.get("id"), "");
                let domain = self.domain.lock().await;
                match domain.notes.get(&key) {
                    Some(note) => {
                        ServerMessage::reply_with(id, serde_json::to_value(note).unwrap())
                    }
                    None => ServerMessage::reply_err(id, format!("no note {key}")),
                }
            }
            "notes.set" => {
                let key = arg_string(args.get("id"), "");
                let title = arg_string(args.get("title"), "");
                let body = arg_string(args.get("body"), "");
                if key.trim().is_empty() {
                    return ServerMessage::reply_err(id, "a note needs an id");
                }
                let result = {
                    let domain = self.domain.lock().await;
                    domain.notes.write(&key, &title, &body)
                };
                match result {
                    Ok(note) => {
                        let _ = self.events.send(ServerMessage::event(
                            "notes.changed",
                            serde_json::json!({ "id": note.id }),
                        ));
                        ServerMessage::reply_with(id, serde_json::to_value(note).unwrap())
                    }
                    Err(err) => {
                        ServerMessage::reply_err(id, format!("could not write the note: {err}"))
                    }
                }
            }
            "notes.new" => {
                let title = arg_string(args.get("title"), "Untitled");
                let key = {
                    let domain = self.domain.lock().await;
                    let base = slugify(&title);
                    let mut candidate = base.clone();
                    let mut n = 2;
                    while domain.notes.get(&candidate).is_some() {
                        candidate = format!("{base}-{n}");
                        n += 1;
                    }
                    candidate
                };
                let result = {
                    let domain = self.domain.lock().await;
                    domain.notes.write(&key, &title, "")
                };
                match result {
                    Ok(note) => {
                        let _ = self.events.send(ServerMessage::event(
                            "notes.changed",
                            serde_json::json!({ "id": note.id }),
                        ));
                        ServerMessage::reply_with(id, serde_json::to_value(note).unwrap())
                    }
                    Err(err) => {
                        ServerMessage::reply_err(id, format!("could not write the note: {err}"))
                    }
                }
            }
            "notes.remove" => {
                let key = arg_string(args.get("id"), "");
                let removed = {
                    let domain = self.domain.lock().await;
                    domain.notes.remove(&key)
                };
                if removed {
                    let _ = self.events.send(ServerMessage::event(
                        "notes.changed",
                        serde_json::json!({ "id": key, "removed": true }),
                    ));
                    ServerMessage::reply_ok(id)
                } else {
                    ServerMessage::reply_err(id, format!("no note {key}"))
                }
            }
            "notes.dir" => {
                let dir = arg_string(args.get("dir"), "");
                if dir.trim().is_empty() {
                    let domain = self.domain.lock().await;
                    return ServerMessage::reply_with(
                        id,
                        serde_json::json!({ "dir": domain.notes.dir().to_string_lossy() }),
                    );
                }
                {
                    let mut domain = self.domain.lock().await;
                    domain.set_notes_dir(PathBuf::from(dir));
                }
                let _ = self.events.send(ServerMessage::event(
                    "notes.changed",
                    serde_json::json!({ "dir": true }),
                ));
                ServerMessage::reply_ok(id)
            }
            "notes.obsidian" => {
                let key = arg_string(args.get("id"), "");
                let domain = self.domain.lock().await;
                match domain.notes.get(&key) {
                    Some(note) => ServerMessage::reply_with(
                        id,
                        serde_json::json!({ "uri": crate::data::notes::obsidian_uri(&note.path) }),
                    ),
                    None => ServerMessage::reply_err(id, format!("no note {key}")),
                }
            }

            "clipboard.list" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({ "items": domain.clipboard.items }),
                )
            }
            "clipboard.add" => {
                let text = arg_string(args.get("text"), "");
                let added = {
                    let mut domain = self.domain.lock().await;
                    let added = domain.clipboard.push(&text, unix_now());
                    if added.is_some() {
                        domain.save_clipboard();
                    }
                    added
                };
                match added {
                    Some(entry_id) => {
                        let _ = self.events.send(ServerMessage::event(
                            "clipboard.changed",
                            serde_json::json!({ "id": entry_id }),
                        ));
                        ServerMessage::reply_with(id, serde_json::json!({ "id": entry_id }))
                    }
                    None => ServerMessage::reply_err(id, "nothing to add"),
                }
            }
            "clipboard.remove" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let removed = {
                    let mut domain = self.domain.lock().await;
                    let removed = domain.clipboard.remove(target);
                    domain.save_clipboard();
                    removed
                };
                if removed {
                    let _ = self.events.send(ServerMessage::event(
                        "clipboard.changed",
                        serde_json::json!({ "id": target, "removed": true }),
                    ));
                    ServerMessage::reply_ok(id)
                } else {
                    ServerMessage::reply_err(id, format!("no clipboard entry {target}"))
                }
            }
            "clipboard.pin" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let pinned = args
                    .get("pinned")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                let result = {
                    let mut domain = self.domain.lock().await;
                    let result = domain.clipboard.set_pinned(target, pinned);
                    domain.save_clipboard();
                    result
                };
                match result {
                    Some(pinned) => {
                        ServerMessage::reply_with(id, serde_json::json!({ "pinned": pinned }))
                    }
                    None => ServerMessage::reply_err(id, format!("no clipboard entry {target}")),
                }
            }
            "clipboard.clear" => {
                let cleared = {
                    let mut domain = self.domain.lock().await;
                    let cleared = domain.clipboard.clear();
                    domain.save_clipboard();
                    cleared
                };
                let _ = self.events.send(ServerMessage::event(
                    "clipboard.changed",
                    serde_json::json!({ "cleared": cleared }),
                ));
                ServerMessage::reply_with(id, serde_json::json!({ "cleared": cleared }))
            }
            "clipboard.paste" => {
                let target = args
                    .get("id")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let text = {
                    let mut domain = self.domain.lock().await;
                    domain.clipboard.use_entry(target)
                };
                match text {
                    Some(text) => match copy_to_clipboard(&text) {
                        Ok(()) => ServerMessage::reply_ok(id),
                        Err(err) => ServerMessage::reply_err(id, err),
                    },
                    None => ServerMessage::reply_err(id, format!("no clipboard entry {target}")),
                }
            }
            "clipboard.capture" => {
                let text = read_clipboard();
                match text {
                    Some(text) => {
                        let added = {
                            let mut domain = self.domain.lock().await;
                            let added = domain.clipboard.push(&text, unix_now());
                            if added.is_some() {
                                domain.save_clipboard();
                            }
                            added
                        };
                        match added {
                            Some(entry_id) => {
                                let _ = self.events.send(ServerMessage::event(
                                    "clipboard.changed",
                                    serde_json::json!({ "id": entry_id }),
                                ));
                                ServerMessage::reply_with(id, serde_json::json!({ "id": entry_id }))
                            }
                            None => ServerMessage::reply_err(id, "the clipboard is empty"),
                        }
                    }
                    None => ServerMessage::reply_err(id, "could not read the clipboard"),
                }
            }
            "network.wifi" => {
                let enabled = args
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                match crate::network::set_wifi_enabled(enabled).await {
                    Ok(()) => ServerMessage::reply_ok(id),
                    Err(err) => ServerMessage::reply_err(id, err),
                }
            }

            "wallpaper.list" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(id, wallpaper_payload(&domain))
            }
            "wallpaper.set" => {
                let path = arg_string(args.get("path"), "");
                self.wallpaper_set(id, path).await
            }
            "wallpaper.setdir" => {
                let dir = arg_string(args.get("dir"), "");
                if dir.trim().is_empty() {
                    return ServerMessage::reply_err(id, "a wallpaper directory is required");
                }
                {
                    let mut domain = self.domain.lock().await;
                    domain.set_wallpaper_dir(PathBuf::from(dir));
                }
                let _ = self.events.send(ServerMessage::event(
                    "wallpaper.changed",
                    serde_json::json!({ "dir": true }),
                ));
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(id, wallpaper_payload(&domain))
            }
            "wallpaper.rescan" => {
                {
                    let mut domain = self.domain.lock().await;
                    domain.rescan_wallpapers();
                }
                let _ = self.events.send(ServerMessage::event(
                    "wallpaper.changed",
                    serde_json::json!({ "rescanned": true }),
                ));
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(id, wallpaper_payload(&domain))
            }
            "wallpaper.loop" => {
                let enabled = args
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                let interval = args
                    .get("interval_secs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(60);
                {
                    let mut domain = self.domain.lock().await;
                    domain.settings.wallpaper_loop = enabled;
                    if interval > 0 {
                        domain.settings.wallpaper_loop_secs = interval;
                    }
                    domain.wallpaper.loop_enabled = enabled;
                    domain.wallpaper.loop_interval_secs = domain.settings.wallpaper_loop_secs;
                    domain.wallpaper.last_changed_unix = unix_now();
                    domain.save_settings();
                }
                let _ = self.events.send(ServerMessage::event(
                    "wallpaper.changed",
                    serde_json::json!({ "loop": enabled }),
                ));
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(id, wallpaper_payload(&domain))
            }
            "wallpaper.next" => {
                let next = {
                    let domain = self.domain.lock().await;
                    domain
                        .wallpaper
                        .random_excluding(domain.wallpaper.current.as_deref())
                        .map(|w| w.path.clone())
                };
                match next {
                    Some(path) => self.wallpaper_set(id, path).await,
                    None => ServerMessage::reply_err(id, "there are no wallpapers to choose from"),
                }
            }
            "wallpaper.status" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(id, wallpaper_payload(&domain))
            }

            "calendar.events" => {
                let domain = self.domain.lock().await;
                let calendar = domain.calendar();
                let now = unix_now() as i64;
                let events: Vec<_> = calendar.upcoming(now).into_iter().cloned().collect();
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({
                        "events": events,
                        "current": calendar.current(now),
                        "dir": domain.settings.calendar_dir,
                    }),
                )
            }
            "calendar.dir" => {
                let dir = arg_string(args.get("dir"), "");
                if dir.trim().is_empty() {
                    let domain = self.domain.lock().await;
                    return ServerMessage::reply_with(
                        id,
                        serde_json::json!({ "dir": domain.settings.calendar_dir }),
                    );
                }
                {
                    let mut domain = self.domain.lock().await;
                    domain.settings.calendar_dir = dir;
                    domain.save_settings();
                }
                ServerMessage::reply_ok(id)
            }

            "settings.get" => {
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::to_value(&domain.settings).unwrap_or_default(),
                )
            }
            "settings.set" => {
                let key = arg_string(args.get("key"), "");
                let value = args.get("value").cloned().unwrap_or_default();
                {
                    let mut domain = self.domain.lock().await;
                    apply_setting(&mut domain.settings, &key, &value);
                    domain.save_settings();
                }
                let _ = self.events.send(ServerMessage::event(
                    "settings.changed",
                    serde_json::json!({ "key": key }),
                ));
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::to_value(&domain.settings).unwrap_or_default(),
                )
            }

            "context.summary" => {
                let stats = self.sample().await;
                let notifications = self.notifications.list().await;
                let domain = self.domain.lock().await;
                ServerMessage::reply_with(
                    id,
                    serde_json::json!({
                        "stats": stats,
                        "ambient": domain.activities.ambient(),
                        "recent": domain.activities.recent(),
                        "todos": domain.todos.ordered(),
                        "next_todo": domain.todos.next_upcoming(),
                        "timer": timer_payload(&domain.timer, unix_now_ms()),
                        "notifications": notifications,
                        "wallpaper": wallpaper_payload(&domain),
                        "settings": domain.settings,
                    }),
                )
            }

            other => ServerMessage::reply_err(id, format!("unknown command: {other}")),
        }
    }

    async fn activity_trigger(&self, id: u64, args: serde_json::Value) -> ServerMessage {
        let kind = args
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .and_then(parse_kind)
            .unwrap_or(ActivityKind::System);
        let title = arg_string(args.get("title"), "Something happened");
        let subtitle = arg_string(args.get("subtitle"), "");
        let tone = args
            .get("tone")
            .and_then(serde_json::Value::as_str)
            .and_then(parse_tone)
            .unwrap_or_else(|| kind.default_tone());
        let glyph = arg_string(args.get("glyph"), "");
        let dwell_ms = args
            .get("dwell_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5_000);
        let sticky = args
            .get("sticky")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let source = args.get("source").and_then(serde_json::Value::as_str);
        let detail = args
            .get("detail")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        let activity_id = self
            .trigger_activity(
                kind,
                title,
                subtitle,
                tone,
                &glyph,
                dwell_ms,
                sticky,
                source.map(str::to_string),
            )
            .await;

        let payload = {
            let domain = self.domain.lock().await;
            domain
                .activities
                .get(activity_id)
                .and_then(|a| a.detail.clone())
                .or(detail)
        };

        ServerMessage::reply_with(
            id,
            serde_json::json!({ "id": activity_id, "detail": payload }),
        )
    }

    async fn wallpaper_set(&self, id: u64, path: String) -> ServerMessage {
        if path.trim().is_empty() {
            return ServerMessage::reply_err(id, "a wallpaper path is required");
        }
        let (engine, known) = {
            let domain = self.domain.lock().await;
            (
                domain.wallpaper.engine,
                domain.wallpaper.find(&path).is_some(),
            )
        };
        if !known && !Path::new(&path).is_file() {
            return ServerMessage::reply_err(id, format!("no such wallpaper: {path}"));
        }
        if let Err(err) = engine.apply(Path::new(&path)) {
            return ServerMessage::reply_err(id, format!("the engine refused the image: {err}"));
        }
        {
            let mut domain = self.domain.lock().await;
            domain.wallpaper.current = Some(path.clone());
            domain.wallpaper.last_changed_unix = unix_now();
        }
        let _ = self.events.send(ServerMessage::event(
            "wallpaper.changed",
            serde_json::json!({ "path": path }),
        ));
        ServerMessage::reply_ok(id)
    }
}

/// The number of pomodoro rounds completed, for the activity subtitle.
async fn domain_rounds(server: &Server) -> u32 {
    server.domain.lock().await.timer.rounds_completed
}

fn timer_payload(timer: &crate::data::timer::TimerState, now_ms: u64) -> serde_json::Value {
    serde_json::json!({
        "phase": timer.phase,
        "running": timer.running,
        "remaining_ms": timer.remaining_ms(now_ms),
        "elapsed_ms": timer.elapsed_total_ms(now_ms),
        "progress": timer.progress(now_ms),
        "rounds_completed": timer.rounds_completed,
        "work_minutes": timer.work_minutes,
        "short_break_minutes": timer.short_break_minutes,
        "long_break_minutes": timer.long_break_minutes,
    })
}

fn wallpaper_payload(domain: &Domain) -> serde_json::Value {
    serde_json::json!({
        "items": domain.wallpaper.items,
        "dir": domain.wallpaper.dir,
        "current": domain.wallpaper.current,
        "engine": domain.wallpaper.engine,
        "engine_ok": domain.wallpaper.engine.probe().is_ok(),
        "loop_enabled": domain.wallpaper.loop_enabled,
        "loop_interval_secs": domain.wallpaper.loop_interval_secs,
    })
}

fn apply_setting(settings: &mut Settings, key: &str, value: &serde_json::Value) {
    match key {
        "theme" => {
            if let Some(theme) = value.as_str() {
                settings.theme = theme.to_string();
            }
        }
        "auto_context" => {
            if let Some(enabled) = value.as_bool() {
                settings.auto_context = enabled;
            }
        }
        "wallpaper_dir" => {
            if let Some(dir) = value.as_str() {
                settings.wallpaper_dir = dir.to_string();
            }
        }
        "wallpaper_loop" => {
            if let Some(enabled) = value.as_bool() {
                settings.wallpaper_loop = enabled;
            }
        }
        "wallpaper_loop_secs" => {
            if let Some(seconds) = value.as_u64() {
                if seconds > 0 {
                    settings.wallpaper_loop_secs = seconds;
                }
            }
        }
        "notes_dir" => {
            if let Some(dir) = value.as_str() {
                settings.notes_dir = dir.to_string();
            }
        }
        "calendar_dir" => {
            if let Some(dir) = value.as_str() {
                settings.calendar_dir = dir.to_string();
            }
        }
        _ => {}
    }
}

fn parse_kind(value: &str) -> Option<ActivityKind> {
    Some(match value {
        "media" => ActivityKind::Media,
        "notification" => ActivityKind::Notification,
        "timer" => ActivityKind::Timer,
        "meeting" => ActivityKind::Meeting,
        "low-battery" | "low_battery" => ActivityKind::LowBattery,
        "focus" => ActivityKind::Focus,
        "todo" => ActivityKind::Todo,
        "clipboard" => ActivityKind::Clipboard,
        "network" => ActivityKind::Network,
        "system" => ActivityKind::System,
        _ => return None,
    })
}

fn parse_tone(value: &str) -> Option<ActivityTone> {
    Some(match value {
        "neutral" => ActivityTone::Neutral,
        "info" => ActivityTone::Info,
        "good" => ActivityTone::Good,
        "warn" => ActivityTone::Warn,
        "critical" => ActivityTone::Critical,
        _ => return None,
    })
}

/// A file-name-safe version of a title.
fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "note".to_string()
    } else {
        trimmed
    }
}

/// Put text back on the clipboard.
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run wl-copy: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("could not write to wl-copy: {e}"))?;
    }
    drop(child.stdin.take());
    let status = child
        .wait()
        .map_err(|e| format!("wl-copy did not finish: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("wl-copy failed".to_string())
    }
}

/// Read whatever is on the clipboard right now.
fn read_clipboard() -> Option<String> {
    let output = std::process::Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

use isle_core::state::HintValue;
use std::collections::BTreeMap;

fn arg_string(value: Option<&serde_json::Value>, fallback: &str) -> String {
    value
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

fn arg_actions(value: Option<&serde_json::Value>) -> Vec<isle_core::state::Action> {
    let Some(value) = value.and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    if value.iter().all(|item| item.is_object()) {
        return value
            .iter()
            .map(|item| isle_core::state::Action {
                key: arg_string(item.get("key"), "default"),
                label: arg_string(item.get("label"), ""),
            })
            .collect();
    }
    let flat: Vec<&str> = value.iter().filter_map(|item| item.as_str()).collect();
    notify::actions_from_pairs(&flat)
}

fn arg_hints(value: Option<&serde_json::Value>) -> BTreeMap<String, HintValue> {
    value
        .and_then(|v| serde_json::from_value::<BTreeMap<String, HintValue>>(v.clone()).ok())
        .unwrap_or_default()
}

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

    /// A server whose state lives in a scratch directory, so the tests never
    /// touch the real one.
    fn server(name: &str) -> Arc<Server> {
        let dir = scratch(name);
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::rooted_at(dir.join("state"), dir.join("data"));
        Server::with_domain(dir.join("isled.sock"), false, Domain::load(store))
    }

    async fn call(server: &Server, name: &str, args: serde_json::Value) -> ServerMessage {
        server
            .dispatch(ClientMessage::Cmd {
                id: 1,
                name: name.into(),
                args,
            })
            .await
    }

    #[tokio::test]
    async fn ping_is_answered() {
        let server = server("ping");
        let reply = call(&server, "ping", serde_json::Value::Null).await;
        assert_eq!(reply, ServerMessage::reply_ok(1));
    }

    #[tokio::test]
    async fn unknown_commands_fail_loudly_rather_than_silently() {
        let server = server("unknown");
        let reply = call(&server, "volume.set", serde_json::json!({ "level": 0.4 })).await;
        assert_eq!(
            reply,
            ServerMessage::reply_err(1, "unknown command: volume.set")
        );
    }

    #[tokio::test]
    async fn a_todo_can_be_added_listed_and_toggled() {
        let server = server("todos");
        let added = call(
            &server,
            "todos.add",
            serde_json::json!({ "text": "write the thing", "due_in_minutes": 30 }),
        )
        .await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = added
        else {
            panic!("expected a reply with data");
        };
        let id = data["id"].as_u64().unwrap();

        let listed = call(&server, "todos.list", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = listed
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["items"].as_array().unwrap().len(), 1);
        assert_eq!(data["items"][0]["text"], "write the thing");

        let toggled = call(&server, "todos.toggle", serde_json::json!({ "id": id })).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = toggled
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["done"], true);
    }

    #[tokio::test]
    async fn a_todo_without_text_is_refused() {
        let server = server("todos-empty");
        let reply = call(&server, "todos.add", serde_json::json!({})).await;
        assert!(matches!(reply, ServerMessage::Reply { ok: false, .. }));
    }

    #[tokio::test]
    async fn the_timer_can_be_started_and_stopped() {
        let server = server("timer");
        call(&server, "timer.start", serde_json::json!({ "minutes": 5 })).await;
        let status = call(&server, "timer.status", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = status
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["running"], true);
        assert_eq!(data["phase"], "work");

        call(&server, "timer.stop", serde_json::Value::Null).await;
        let status = call(&server, "timer.status", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = status
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["running"], false);
        assert_eq!(data["phase"], "idle");
    }

    #[tokio::test]
    async fn an_activity_becomes_ambient_when_it_is_sticky() {
        let server = server("activity");
        call(
            &server,
            "activity.trigger",
            serde_json::json!({
                "kind": "media",
                "title": "Glasshouse",
                "subtitle": "Aether",
                "sticky": true,
                "source": "spotify",
            }),
        )
        .await;

        let listed = call(&server, "activity.list", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = listed
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["ambient"]["kind"], "media");
        assert_eq!(data["ambient"]["title"], "Glasshouse");
    }

    #[tokio::test]
    async fn a_transient_activity_is_not_ambient() {
        let server = server("activity-transient");
        call(
            &server,
            "activity.trigger",
            serde_json::json!({ "kind": "notification", "title": "Build done" }),
        )
        .await;
        let listed = call(&server, "activity.list", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = listed
        else {
            panic!("expected a reply with data");
        };
        assert!(data["ambient"].is_null());
        assert_eq!(data["recent"][0]["kind"], "notification");
    }

    #[tokio::test]
    async fn a_client_gets_a_snapshot_then_replies() {
        let dir = scratch("handshake");
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::rooted_at(dir.join("state"), dir.join("data"));
        let server = Server::with_domain(dir.join("isled.sock"), false, Domain::load(store));
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
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::rooted_at(dir.join("state"), dir.join("data"));
        let server = Server::with_domain(dir.join("isled.sock"), false, Domain::load(store));
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
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::rooted_at(dir.join("state"), dir.join("data"));
        let server = Server::with_domain(dir.join("isled.sock"), false, Domain::load(store));
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
                ServerMessage::Event { ref topic, .. } if topic == "system.stats"
            ),
            "got: {line}"
        );

        server.trigger_shutdown();
        let _ = timeout(Duration::from_secs(5), serving).await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn the_context_summary_gathers_everything() {
        let server = server("context");
        call(
            &server,
            "activity.trigger",
            serde_json::json!({ "kind": "media", "title": "Song", "sticky": true }),
        )
        .await;
        let summary = call(&server, "context.summary", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = summary
        else {
            panic!("expected a reply with data");
        };
        assert!(data["stats"].is_object());
        assert_eq!(data["ambient"]["title"], "Song");
        assert!(data["todos"].is_array());
        assert!(data["timer"].is_object());
    }

    #[tokio::test]
    async fn the_clipboard_keeps_what_it_is_given() {
        let server = server("clipboard");
        call(
            &server,
            "clipboard.add",
            serde_json::json!({ "text": "nix develop" }),
        )
        .await;
        let listed = call(&server, "clipboard.list", serde_json::Value::Null).await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = listed
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["items"][0]["text"], "nix develop");
        assert_eq!(data["items"][0]["kind"], "code");
    }

    #[tokio::test]
    async fn a_note_can_be_written_and_read_back() {
        let server = server("notes");
        let dir = scratch("notes-vault");
        let _ = std::fs::remove_dir_all(&dir);
        call(
            &server,
            "notes.dir",
            serde_json::json!({ "dir": dir.to_string_lossy() }),
        )
        .await;
        call(
            &server,
            "notes.set",
            serde_json::json!({ "id": "greeting", "title": "Greeting", "body": "hello" }),
        )
        .await;
        let got = call(
            &server,
            "notes.get",
            serde_json::json!({ "id": "greeting" }),
        )
        .await;
        let ServerMessage::Reply {
            data: Some(data), ..
        } = got
        else {
            panic!("expected a reply with data");
        };
        assert_eq!(data["title"], "Greeting");
        assert_eq!(data["body"], "hello");
        let _ = std::fs::remove_dir_all(&dir);
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
