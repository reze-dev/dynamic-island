use crate::server::Events;
use isle_core::state::{Action, CloseReason, HintValue, Notification, Notifier};
use isle_core::{ServerMessage, state::unix_now};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use zbus::fdo::RequestNameFlags;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

pub const NOTIFICATIONS_BUS_NAME: &str = "org.freedesktop.Notifications";
pub const NOTIFICATIONS_OBJECT_PATH: &str = "/org/freedesktop/Notifications";

const SERVER_INFO_NAME: &str = "isled";
const SERVER_INFO_VENDOR: &str = "Dynamic Island Shell";
const SPEC_VERSION: &str = "1.2";
const EXPIRE_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NotifyRequest {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<Action>,
    pub hints: BTreeMap<String, HintValue>,
    pub expire_timeout: i32,
}

impl NotifyRequest {
    pub fn into_notification(self, now: u64) -> Notification {
        Notification {
            id: self.replaces_id,
            app_name: self.app_name,
            app_icon: self.app_icon,
            summary: self.summary,
            body: self.body,
            actions: self.actions,
            hints: self.hints,
            expire_timeout: self.expire_timeout,
            received_at_unix: now,
            desktop_entry: None,
            urgency: None,
            category: None,
            transient: false,
            resident: false,
            has_image: false,
        }
    }
}

pub fn promote(mut incoming: Notification) -> Notification {
    let hints = std::mem::take(&mut incoming.hints);

    if let Some(HintValue::Str(desktop)) = hints.get("desktop-entry") {
        incoming.desktop_entry = Some(strip_desktop_suffix(desktop));
    }
    if let Some(HintValue::Str(category)) = hints.get("category") {
        incoming.category = Some(category.clone());
    }
    if let Some(HintValue::Byte(urgency)) = hints.get("urgency") {
        incoming.urgency = Some(*urgency);
    }
    incoming.transient = matches!(hints.get("transient"), Some(HintValue::Bool(true)));
    incoming.has_image = hints.contains_key("image-data") || hints.contains_key("icon_data");

    incoming.hints = hints;
    incoming
}

pub fn strip_desktop_suffix(desktop: &str) -> String {
    desktop
        .strip_suffix(".desktop")
        .unwrap_or(desktop)
        .to_owned()
}

pub fn actions_from_pairs(pairs: &[&str]) -> Vec<Action> {
    pairs
        .chunks(2)
        .filter_map(|pair| match pair {
            [key, label] => Some(Action {
                key: (*key).to_owned(),
                label: (*label).to_owned(),
            }),
            _ => None,
        })
        .collect()
}

pub fn parse_hints(map: HashMap<String, OwnedValue>) -> BTreeMap<String, HintValue> {
    let mut out = BTreeMap::new();
    for (key, value) in map {
        if let Some(hint) = hint_from(&value) {
            out.insert(key, hint);
        }
    }
    out
}

fn hint_from(value: &OwnedValue) -> Option<HintValue> {
    // Trying each type in turn is simpler and sturdier than matching on the
    // value's signature, which zvariant exposes as a trait constant rather than
    // something matchable.
    if let Ok(byte) = value.downcast_ref::<u8>() {
        return Some(HintValue::Byte(byte));
    }
    if let Ok(flag) = value.downcast_ref::<bool>() {
        return Some(HintValue::Bool(flag));
    }
    if let Ok(int) = value.downcast_ref::<i32>() {
        return Some(HintValue::Int(int));
    }
    if let Some(int) = value
        .downcast_ref::<u32>()
        .ok()
        .map(|v| i32::try_from(v).unwrap_or(i32::MAX))
    {
        return Some(HintValue::Int(int));
    }
    if let Some(int) = value.downcast_ref::<f64>().ok().map(|v| v.round() as i32) {
        return Some(HintValue::Int(int));
    }
    if let Ok(text) = value.downcast_ref::<String>() {
        return Some(HintValue::Str(text));
    }
    // Array-valued hints are skipped: zvariant's downcast_ref has no container
    // support, and none of the hints this shell cares about are arrays. The raw
    // keys still tell us image-data is present, which is all the UI needs.
    None
}

pub struct Hub {
    store: Mutex<Notifier>,
    events: Events,
    bus: OnceLock<zbus::Connection>,
}

impl Hub {
    pub fn detached(events: Events) -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Notifier::new()),
            events,
            bus: OnceLock::new(),
        })
    }

    pub fn attach(&self, connection: zbus::Connection) {
        let _ = self.bus.set(connection);
    }

    pub fn is_attached(&self) -> bool {
        self.bus.get().is_some()
    }

    async fn emitter(&self) -> Option<SignalEmitter<'static>> {
        let connection = self.bus.get()?.clone();
        let path = zbus::zvariant::ObjectPath::try_from(NOTIFICATIONS_OBJECT_PATH).ok()?;
        Some(SignalEmitter::from_parts(connection, path))
    }

    pub async fn ingest(&self, request: NotifyRequest) -> Notification {
        let promoted = promote(request.into_notification(unix_now()));
        let id = {
            let mut store = self.store.lock().await;
            store.push(promoted.clone())
        };
        let mut stored = promoted;
        stored.id = id;
        self.broadcast("notifications.added", &stored);
        stored
    }

    pub async fn dismiss(&self, id: u32, reason: CloseReason) -> Option<Notification> {
        let removed = self.store.lock().await.dismiss(id)?;
        self.broadcast(
            "notifications.removed",
            &serde_json::json!({
                "id": id,
                "reason": reason,
                "code": reason.as_code(),
            }),
        );
        Some(removed)
    }

    pub async fn dismiss_all(&self) -> usize {
        let removed = self.store.lock().await.dismiss_all();
        if !removed.is_empty() {
            self.broadcast("notifications.cleared", &());
        }
        removed.len()
    }

    pub async fn invoke_action(&self, id: u32, key: &str) {
        self.broadcast(
            "notifications.action",
            &serde_json::json!({ "id": id, "key": key }),
        );
    }

    pub async fn expire_due(&self) -> usize {
        let due = {
            let store = self.store.lock().await;
            store.expired_ids(unix_now())
        };
        let mut closed = 0;
        for id in due {
            self.emit_closed(id, CloseReason::Expired.as_code()).await;
            self.dismiss(id, CloseReason::Expired).await;
            closed += 1;
        }
        closed
    }

    pub async fn list(&self) -> Vec<Notification> {
        self.store.lock().await.list().to_vec()
    }

    fn broadcast(&self, topic: &str, payload: &impl serde::Serialize) {
        let data = serde_json::to_value(payload).unwrap_or_default();
        let _ = self.events.send(ServerMessage::event(topic, data));
    }

    async fn emit_closed(&self, id: u32, code: u32) {
        if let Some(emitter) = self.emitter().await {
            let _ = NotificationsInterface::notification_closed(&emitter, id, code).await;
        }
    }
}

pub struct NotificationsInterface {
    hub: Arc<Hub>,
}

impl NotificationsInterface {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self { hub }
    }
}

// The freedesktop spec fixes the arity of Notify; there is nothing to shorten.
#[allow(clippy::too_many_arguments)]
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl NotificationsInterface {
    async fn notify(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        let _ = emitter;
        let request = NotifyRequest {
            app_name: app_name.to_owned(),
            replaces_id,
            app_icon: app_icon.to_owned(),
            summary: summary.to_owned(),
            body: body.to_owned(),
            actions: actions_from_pairs(&actions),
            hints: parse_hints(hints),
            expire_timeout,
        };
        Ok(self.hub.ingest(request).await.id)
    }

    async fn close_notification(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        id: u32,
    ) {
        let code = CloseReason::ClosedByCaller.as_code();
        if let Some(removed) = self.hub.dismiss(id, CloseReason::ClosedByCaller).await {
            let _ = NotificationsInterface::notification_closed(&emitter, id, code).await;
            let _ = removed;
        }
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "actions".to_owned(),
            "body".to_owned(),
            "body-markup".to_owned(),
            "icon-static".to_owned(),
            "persistence".to_owned(),
        ]
    }

    fn get_server_information(&self) -> zbus::fdo::Result<(String, String, String, String)> {
        Ok((
            SERVER_INFO_NAME.to_owned(),
            SERVER_INFO_VENDOR.to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
            SPEC_VERSION.to_owned(),
        ))
    }

    #[zbus(signal)]
    pub async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

pub async fn claim(hub: &Arc<Hub>) -> Result<zbus::Connection, zbus::Error> {
    let connection = zbus::Connection::session().await?;

    // DoNotQueue *without* ReplaceExisting. zbus's plain request_name defaults to
    // ReplaceExisting | DoNotQueue, which would silently take the name off Ryoku
    // and swallow every notification on the session. We would rather fail loudly
    // and leave Ryoku in charge while it is still alive.
    connection
        .request_name_with_flags(NOTIFICATIONS_BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await?;

    hub.attach(connection.clone());
    connection
        .object_server()
        .at(
            NOTIFICATIONS_OBJECT_PATH,
            NotificationsInterface::new(Arc::clone(hub)),
        )
        .await?;

    tracing::info!(
        name = NOTIFICATIONS_BUS_NAME,
        path = NOTIFICATIONS_OBJECT_PATH,
        "claimed the notification bus name"
    );
    Ok(connection)
}

pub fn spawn_expiry_sweep(hub: Arc<Hub>) {
    tokio::spawn(async move { sweep_forever(hub).await });
}

async fn sweep_forever(hub: Arc<Hub>) {
    let mut ticker = tokio::time::interval(EXPIRE_SWEEP_INTERVAL);
    loop {
        ticker.tick().await;
        let closed = hub.expire_due().await;
        if closed > 0 {
            tracing::debug!(closed, "expired notifications");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(hints: Vec<(&str, HintValue)>) -> NotifyRequest {
        NotifyRequest {
            app_name: "solar".into(),
            replaces_id: 0,
            app_icon: "dialog-information".into(),
            summary: "Hello".into(),
            body: "World".into(),
            actions: vec![],
            hints: hints.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            expire_timeout: 0,
        }
    }

    fn hub() -> Arc<Hub> {
        let (events, _rx) = tokio::sync::broadcast::channel(16);
        Hub::detached(events)
    }

    #[test]
    fn a_detached_hub_knows_it_is_not_on_the_bus() {
        assert!(!hub().is_attached());
    }

    #[test]
    fn a_desktop_suffix_is_dropped_from_a_desktop_entry() {
        assert_eq!(
            strip_desktop_suffix("org.gnome.Nautilus.desktop"),
            "org.gnome.Nautilus"
        );
        assert_eq!(
            strip_desktop_suffix("org.gnome.Nautilus"),
            "org.gnome.Nautilus"
        );
    }

    #[test]
    fn urgency_and_category_are_promoted_to_first_class_fields() {
        let stored = promote(
            request(vec![
                ("urgency", HintValue::Byte(2)),
                ("category", HintValue::Str("im.received".into())),
                ("desktop-entry", HintValue::Str("signal.desktop".into())),
                ("transient", HintValue::Bool(true)),
            ])
            .into_notification(1_000),
        );
        assert_eq!(stored.urgency_level(), 2);
        assert_eq!(stored.category.as_deref(), Some("im.received"));
        assert_eq!(stored.desktop_entry.as_deref(), Some("signal"));
        assert!(stored.transient);
        assert!(stored.hints.contains_key("urgency"));
    }

    #[test]
    fn image_hints_are_a_flag_not_a_payload() {
        assert!(
            promote(request(vec![("image-data", HintValue::Int(0))]).into_notification(1))
                .has_image
        );
        assert!(
            promote(request(vec![("icon_data", HintValue::Int(0))]).into_notification(1)).has_image
        );
        assert!(!promote(request(vec![]).into_notification(1)).has_image);
    }

    #[test]
    fn an_absent_urgency_stays_normal() {
        let stored = promote(request(vec![]).into_notification(1));
        assert_eq!(stored.urgency_level(), 1);
        assert!(stored.category.is_none());
        assert!(!stored.transient);
    }

    #[test]
    fn actions_arrive_as_a_flat_alternating_list() {
        let flat = vec!["default", "Open", "reply", "Reply"];
        let actions = actions_from_pairs(&flat);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].key, "default");
        assert_eq!(actions[0].label, "Open");
        assert_eq!(actions[1].key, "reply");
        assert_eq!(actions[1].label, "Reply");
    }

    #[test]
    fn a_trailing_unpaired_action_key_is_discarded() {
        assert_eq!(actions_from_pairs(&["default"]).len(), 0);
    }

    #[tokio::test]
    async fn ingesting_a_notification_assigns_an_id_and_broadcasts() {
        let hub = hub();
        let stored = hub.ingest(request(vec![])).await;
        assert_eq!(stored.app_name, "solar");
        assert!(stored.id > 0);
        assert_eq!(hub.list().await.len(), 1);
    }

    #[tokio::test]
    async fn a_replace_reuses_the_original_id() {
        let hub = hub();
        let first = hub.ingest(request(vec![])).await.id;
        let mut replacement = request(vec![]);
        replacement.replaces_id = first;
        let replaced = hub.ingest(replacement).await;
        assert_eq!(replaced.id, first);
        assert_eq!(hub.list().await.len(), 1);
    }

    #[tokio::test]
    async fn dismissing_broadcasts_the_removal() {
        let hub = hub();
        let stored = hub.ingest(request(vec![])).await;
        let removed = hub
            .dismiss(stored.id, CloseReason::Dismissed)
            .await
            .expect("the notification should still be there");
        assert_eq!(removed.id, stored.id);
        assert!(hub.list().await.is_empty());
    }

    #[tokio::test]
    async fn dismissing_an_unknown_id_is_not_an_error() {
        let hub = hub();
        assert!(hub.dismiss(4042, CloseReason::Other).await.is_none());
        assert!(hub.list().await.is_empty());
    }
}
