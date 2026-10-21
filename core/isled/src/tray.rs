//! The system tray.
//!
//! The shell is a StatusNotifierHost: it owns
//! `org.kde.StatusNotifierWatcher` on the session bus, watches for items
//! appearing, and keeps their state so the panel can draw them.
//!
//! Two things make this fiddly. The first is that an item registers itself
//! in one of two ways — by calling the watcher, or by simply exporting the
//! object and hoping a host notices — so both are handled. The second is
//! that there is no way to ask an item "has anything changed?", so the
//! properties are read again on a timer and compared.
//!
//! The state is deliberately small: the icon *name*, not the icon itself.
//! An item may offer a pixmap instead of a name, and rendering that means
//! decoding an image the shell would then have to encode again for the
//! panel. The name is what an icon theme can resolve, and it is what the
//! overwhelming majority of items provide.

use crate::server::Events;
use isle_core::ServerMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use zbus::zvariant::{OwnedValue, Value};

/// The bus name a StatusNotifierHost owns.
const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
/// Where the watcher object lives.
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
/// Where an item's object lives.
const ITEM_PATH: &str = "/StatusNotifierItem";
/// The interface an item implements.
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

/// How often the items are read again.
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// One thing in the tray.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayItem {
    /// The application's own identifier for the item.
    pub id: String,
    /// The title, which is usually the application's name.
    pub title: String,
    /// `Passive`, `Active` or `NeedsAttention`.
    pub status: String,
    /// `ApplicationStatus`, `Communications`, `SystemServices` and so on.
    pub category: String,
    /// The icon to draw.
    pub icon: String,
    /// The icon to draw when the item wants attention.
    pub attention_icon: String,
    /// The tooltip, if the item has one.
    pub tooltip: String,
    /// Whether a click should open a menu rather than do something.
    pub is_menu: bool,
    /// The bus name of the application, which is how it is addressed.
    pub service: String,
}

impl TrayItem {
    /// The icon that should be drawn right now.
    pub fn effective_icon(&self) -> &str {
        if self.status == "NeedsAttention" && !self.attention_icon.is_empty() {
            &self.attention_icon
        } else {
            &self.icon
        }
    }

    /// The item as the panel sees it. The icon is already the one to draw,
    /// so the panel does not have to know the rule, and the icon has been
    /// resolved to a file so the panel does not have to know the theme
    /// either.
    pub fn payload(&self) -> serde_json::Value {
        let icon = self.effective_icon();
        // The attention icon is often not in the theme, and the ordinary one
        // is a better answer than nothing at all.
        let path = crate::icons::find(icon).or_else(|| {
            let other = if icon == self.icon {
                &self.attention_icon
            } else {
                &self.icon
            };
            crate::icons::find(other)
        });
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "status": self.status,
            "category": self.category,
            "icon": icon,
            "icon_path": path.map(|p| p.to_string_lossy().into_owned()),
            "tooltip": self.tooltip,
            "is_menu": self.is_menu,
            "service": self.service,
        })
    }
}

/// The registry of tray items.
#[derive(Clone)]
pub struct Tray {
    inner: Arc<Mutex<Registry>>,
    events: Events,
    connection: Option<Arc<zbus::Connection>>,
}

#[derive(Default)]
struct Registry {
    /// Known items, by bus name.
    items: HashMap<String, TrayItem>,
    /// The bus names of the registered hosts.
    hosts: Vec<String>,
}

impl Tray {
    pub fn new(events: Events) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Registry::default())),
            events,
            connection: None,
        }
    }

    /// Every known item.
    pub async fn items(&self) -> Vec<TrayItem> {
        let inner = self.inner.lock().await;
        let mut items: Vec<TrayItem> = inner.items.values().cloned().collect();
        items.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.id.cmp(&b.id)));
        items
    }

    /// The bus names of the items, which is what the watcher's property
    /// reports.
    async fn services(&self) -> Vec<String> {
        let inner = self.inner.lock().await;
        let mut names: Vec<String> = inner.items.keys().cloned().collect();
        names.sort();
        names
    }

    /// Remember an item and tell everyone.
    async fn remember(&self, service: &str, item: TrayItem) {
        {
            let mut inner = self.inner.lock().await;
            inner.items.insert(service.to_string(), item);
        }
        self.broadcast().await;
    }

    /// Forget an item.
    async fn forget(&self, service: &str) {
        {
            let mut inner = self.inner.lock().await;
            inner.items.remove(service);
        }
        self.broadcast().await;
    }

    /// Publish the whole tray state.
    async fn broadcast(&self) {
        let items = self.items().await;
        let payload: Vec<serde_json::Value> = items.iter().map(|i| i.payload()).collect();
        let _ = self.events.send(ServerMessage::event(
            "tray.changed",
            serde_json::Value::Array(payload),
        ));
    }

    /// Read every known item again and publish the result if it changed.
    async fn refresh(&self, connection: &zbus::Connection) {
        let known: Vec<String> = {
            let inner = self.inner.lock().await;
            inner.items.keys().cloned().collect()
        };
        if known.is_empty() {
            return;
        }

        let mut changed = false;
        for service in known {
            let Some(fresh) = read_item(connection, &service).await else {
                // The item is gone.
                let mut inner = self.inner.lock().await;
                if inner.items.remove(&service).is_some() {
                    changed = true;
                }
                continue;
            };
            let mut inner = self.inner.lock().await;
            if inner.items.get(&service) != Some(&fresh) {
                inner.items.insert(service.clone(), fresh);
                changed = true;
            }
        }
        if changed {
            self.broadcast().await;
        }
    }
}

/// The object the shell exports on the bus.
struct WatcherInterface {
    tray: Tray,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherInterface {
    /// An item registering itself. The service is either a bus name or an
    /// object path; a path means the item is on a well-known name, and the
    /// caller is in the message header.
    ///
    /// The method is async rather than spawning a task: the bus runs its
    /// own executor, which has no tokio runtime of its own, so a
    /// `tokio::spawn` here would panic.
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) {
        let name = if service.starts_with('/') {
            // The caller is the item: it exported the object and is asking
            // the watcher to find it.
            header
                .sender()
                .map(|sender| sender.as_str().to_string())
                .unwrap_or_default()
        } else {
            service.to_string()
        };
        if name.is_empty() {
            return;
        }
        if let Some(item) = read_item(connection, &name).await {
            self.tray.remember(&name, item).await;
        }
    }

    /// A host registering itself. The shell is the only host, but the
    /// property has to say whether any host has registered.
    async fn register_status_notifier_host(&self, service: &str) {
        {
            let mut inner = self.tray.inner.lock().await;
            if !inner.hosts.contains(&service.to_string()) {
                inner.hosts.push(service.to_string());
            }
        }
        self.tray.raise_host_registered().await;
    }

    /// The items the shell knows about.
    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.tray.services().await
    }

    /// Whether any host has registered. The shell is a host either way.
    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        let inner = self.tray.inner.lock().await;
        !inner.hosts.is_empty() || true
    }

    /// The version of the protocol.
    #[zbus(property)]
    fn protocol_version(&self) -> u32 {
        1
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}

impl Tray {
    /// Tell the bus that an item appeared.
    async fn raise_item_registered(&self, service: &str) {
        let Some(connection) = self.connection.as_ref() else {
            return;
        };
        let Ok(emitter) = zbus::object_server::SignalEmitter::new(connection, WATCHER_PATH) else {
            return;
        };
        let _ = WatcherInterface::status_notifier_item_registered(&emitter, service).await;
    }

    /// Tell the bus that an item went away.
    async fn raise_item_unregistered(&self, service: &str) {
        let Some(connection) = self.connection.as_ref() else {
            return;
        };
        let Ok(emitter) = zbus::object_server::SignalEmitter::new(connection, WATCHER_PATH) else {
            return;
        };
        let _ = WatcherInterface::status_notifier_item_unregistered(&emitter, service).await;
    }

    /// Tell the bus that a host registered.
    async fn raise_host_registered(&self) {
        let Some(connection) = self.connection.as_ref() else {
            return;
        };
        let Ok(emitter) = zbus::object_server::SignalEmitter::new(connection, WATCHER_PATH) else {
            return;
        };
        let _ = WatcherInterface::status_notifier_host_registered(&emitter).await;
    }
}

/// Read an item's state from the bus.
async fn read_item(connection: &zbus::Connection, service: &str) -> Option<TrayItem> {
    let proxy = zbus::Proxy::new(
        connection,
        service,
        ITEM_PATH,
        "org.freedesktop.DBus.Properties",
    )
    .await
    .ok()?;

    let message = proxy.call_method("GetAll", &(ITEM_INTERFACE,)).await.ok()?;
    let raw: HashMap<String, OwnedValue> = message.body().deserialize().ok()?;

    let get = |name: &str| raw.get(name).map(|value| Value::from(value.clone()));

    let id = get("Id").and_then(as_string).unwrap_or_default();
    let title = get("Title").and_then(as_string).unwrap_or_default();
    let status = get("Status").and_then(as_string).unwrap_or_default();
    let category = get("Category").and_then(as_string).unwrap_or_default();
    let icon = get("IconName").and_then(as_string).unwrap_or_default();
    let attention_icon = get("AttentionIconName")
        .and_then(as_string)
        .unwrap_or_default();
    let is_menu = get("ItemIsMenu").and_then(as_bool).unwrap_or(false);
    let tooltip = get("ToolTip").map(tooltip_text).unwrap_or_default();

    if id.is_empty() && title.is_empty() && icon.is_empty() {
        return None;
    }

    Some(TrayItem {
        id,
        title,
        status,
        category,
        icon,
        attention_icon,
        tooltip,
        is_menu,
        service: service.to_string(),
    })
}

/// Ask an item to do its main thing.
pub async fn activate(service: &str, x: i32, y: i32) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| format!("could not reach the session bus: {e}"))?;
    let proxy = zbus::Proxy::new(&connection, service, ITEM_PATH, ITEM_INTERFACE)
        .await
        .map_err(|e| format!("could not reach {service}: {e}"))?;
    proxy
        .call_method("Activate", &(x, y))
        .await
        .map_err(|e| format!("{service} did not accept the activation: {e}"))?;
    Ok(())
}

/// Ask an item to do its secondary thing.
pub async fn secondary_activate(service: &str, x: i32, y: i32) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| format!("could not reach the session bus: {e}"))?;
    let proxy = zbus::Proxy::new(&connection, service, ITEM_PATH, ITEM_INTERFACE)
        .await
        .map_err(|e| format!("could not reach {service}: {e}"))?;
    proxy
        .call_method("SecondaryActivate", &(x, y))
        .await
        .map_err(|e| format!("{service} did not accept the activation: {e}"))?;
    Ok(())
}

/// Claim the bus name and export the watcher.
pub async fn claim(tray: Tray) -> Result<(zbus::Connection, Tray), zbus::Error> {
    let connection = zbus::Connection::session().await?;
    let mut tray = tray;
    tray.connection = Some(Arc::new(connection.clone()));
    let interface = WatcherInterface { tray: tray.clone() };
    // The object is exported before the name is claimed, so a call that
    // arrives in between is not lost.
    connection
        .object_server()
        .at(WATCHER_PATH, interface)
        .await?;
    connection
        .request_name_with_flags(
            WATCHER_BUS_NAME,
            zbus::fdo::RequestNameFlags::DoNotQueue.into(),
        )
        .await?;
    Ok((connection, tray))
}

/// Watch for items appearing and disappearing, and keep their state fresh.
pub async fn watch(tray: Tray) {
    let Ok(connection) = zbus::Connection::session().await else {
        tracing::warn!("no session bus; the tray will not be watched");
        return;
    };

    // Items that export the object and hope a host notices.
    let rule = match zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus")
        .and_then(|builder| builder.member("NameOwnerChanged"))
    {
        Ok(builder) => builder.build(),
        Err(err) => {
            tracing::warn!(%err, "could not build the bus match rule; the tray will not be watched");
            return;
        }
    };

    let mut stream = match zbus::MessageStream::for_match_rule(rule, &connection, Some(64)).await {
        Ok(stream) => stream,
        Err(err) => {
            tracing::warn!(%err, "could not watch the bus for new names");
            return;
        }
    };

    // Keep the known items fresh.
    let refresher = tray.clone();
    let refresher_connection = connection.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            ticker.tick().await;
            refresher.refresh(&refresher_connection).await;
        }
    });

    use futures_util::StreamExt;
    while let Some(message) = stream.next().await {
        let Ok(message) = message else { continue };
        // The new owner is an empty string when the name went away.
        let Ok((name, _old, new_owner)) = message.body().deserialize::<(String, String, String)>()
        else {
            continue;
        };

        if new_owner.is_empty() {
            // A name went away: if it was an item, forget it.
            if tray.items().await.iter().any(|i| i.service == name) {
                tray.forget(&name).await;
                tray.raise_item_unregistered(&name).await;
            }
        } else {
            // A name appeared: it might be an item.
            if let Some(item) = read_item(&connection, &new_owner).await {
                tray.remember(&new_owner, item).await;
                tray.raise_item_registered(&new_owner).await;
            }
        }
    }
}

/// A string out of a property value.
fn as_string(value: Value) -> Option<String> {
    match value {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

/// A boolean out of a property value.
fn as_bool(value: Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(b),
        _ => None,
    }
}

/// The text of a tooltip, which is a structure of an icon name, a pixmap and
/// two strings.
fn tooltip_text(value: Value) -> String {
    let Value::Structure(fields) = value else {
        return String::new();
    };
    let fields = fields.fields();
    if fields.len() < 4 {
        return String::new();
    }
    // (icon name, icon pixmap, title, description)
    let title = as_string(fields[2].clone()).unwrap_or_default();
    let description = as_string(fields[3].clone()).unwrap_or_default();
    match (title.is_empty(), description.is_empty()) {
        (true, true) => String::new(),
        (true, false) => description,
        (false, true) => title,
        (false, false) => format!("{title} — {description}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(status: &str) -> TrayItem {
        TrayItem {
            id: "a".into(),
            title: "App".into(),
            status: status.into(),
            category: "ApplicationStatus".into(),
            icon: "app-icon".into(),
            attention_icon: "app-attention".into(),
            tooltip: String::new(),
            is_menu: false,
            service: ":1.1".into(),
        }
    }

    #[test]
    fn the_attention_icon_wins_when_the_item_wants_attention() {
        let mut item = item("NeedsAttention");
        assert_eq!(item.effective_icon(), "app-attention");
        item.status = "Active".into();
        assert_eq!(item.effective_icon(), "app-icon");
    }

    #[test]
    fn a_passive_item_keeps_its_own_icon() {
        assert_eq!(item("Passive").effective_icon(), "app-icon");
    }

    #[test]
    fn a_string_is_read_out_of_a_property_value() {
        assert_eq!(as_string(Value::from("hello")), Some("hello".to_string()));
        assert_eq!(as_string(Value::from(3u32)), None);
    }

    #[test]
    fn a_boolean_is_read_out_of_a_property_value() {
        assert_eq!(as_bool(Value::from(true)), Some(true));
        assert_eq!(as_bool(Value::from("yes")), None);
    }

    #[test]
    fn a_tooltip_with_only_a_description_uses_it() {
        let value = Value::from((
            "icon".to_string(),
            Vec::<(i32, i32, Vec<u8>)>::new(),
            String::new(),
            "something happened".to_string(),
        ));
        assert_eq!(tooltip_text(value), "something happened");
    }

    #[test]
    fn a_tooltip_with_a_title_and_a_description_joins_them() {
        let value = Value::from((
            "icon".to_string(),
            Vec::<(i32, i32, Vec<u8>)>::new(),
            "Title".to_string(),
            "Body".to_string(),
        ));
        assert_eq!(tooltip_text(value), "Title — Body");
    }

    #[test]
    fn a_tooltip_with_nothing_in_it_is_empty() {
        let value = Value::from((
            String::new(),
            Vec::<(i32, i32, Vec<u8>)>::new(),
            String::new(),
            String::new(),
        ));
        assert_eq!(tooltip_text(value), "");
    }
}
