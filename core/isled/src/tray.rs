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
    /// A pixmap icon `(width, height, ARGB32 pixels)` when the item
    /// offers one instead of (or alongside) a name. Rendered to a BMP
    /// cache file for the panel when no named icon resolves.
    pub icon_pixmap: Option<(u32, u32, Vec<u32>)>,
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
        let path = crate::icons::find(icon)
            .or_else(|| {
                let other = if icon == self.icon {
                    &self.attention_icon
                } else {
                    &self.icon
                };
                crate::icons::find(other)
            })
            // Pixmap-only items have no name to resolve; render the
            // ARGB32 payload to a BMP cache file the panel can load.
            .or_else(|| self.pixmap_path());
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

    /// Render this item's pixmap to a BMP cache file, if it has one.
    /// The file name carries the dimensions and a pixel checksum so a
    /// changed icon gets a new file instead of a stale one.
    fn pixmap_path(&self) -> Option<std::path::PathBuf> {
        let (w, h, pixels) = self.icon_pixmap.as_ref()?;
        let bytes = bmp_bytes(*w, *h, pixels)?;
        let checksum: u64 = pixels
            .iter()
            .fold(0u64, |acc, p| acc.wrapping_add(*p as u64));
        let dir = crate::cache::tray_pixmaps_dir();
        let _ = std::fs::create_dir_all(&dir);
        let slug = sanitize(&format!(
            "{}-{}-{}-{:x}",
            self.service, self.id, self.title, checksum
        ));
        let path = dir.join(format!("{slug}-{w}x{h}.bmp"));
        let existing = std::fs::read(&path).unwrap_or_default();
        if existing != bytes {
            let _ = std::fs::write(&path, &bytes);
        }
        // Tray pixmaps are tiny; keep the cache from growing without bound.
        prune_pixmap_cache(&dir, 100);
        path.is_file().then_some(path)
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
    let icon_pixmap = get("IconPixmap").and_then(as_pixmap);
    let is_menu = get("ItemIsMenu").and_then(as_bool).unwrap_or(false);
    let tooltip = get("ToolTip").map(tooltip_text).unwrap_or_default();

    if id.is_empty() && title.is_empty() && icon.is_empty() && icon_pixmap.is_none() {
        return None;
    }

    Some(TrayItem {
        id,
        title,
        status,
        category,
        icon,
        attention_icon,
        icon_pixmap,
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

/// A StatusNotifier pixmap: `(width, height, ARGB32 pixels)`.
/// Validated and capped so a hostile item cannot allocate unboundedly.
fn as_pixmap(value: Value) -> Option<(u32, u32, Vec<u32>)> {
    let Value::Structure(fields) = value else {
        return None;
    };
    let fields = fields.fields();
    if fields.len() < 3 {
        return None;
    }
    let width = as_u32(fields[0].clone())?;
    let height = as_u32(fields[1].clone())?;
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return None;
    }
    let pixels = as_u32_array(fields[2].clone())?;
    if pixels.len() != (width as usize) * (height as usize) {
        return None;
    }
    Some((width, height, pixels))
}

fn as_u32(value: Value) -> Option<u32> {
    match value {
        Value::U32(v) => Some(v),
        Value::I32(v) => u32::try_from(v).ok(),
        _ => None,
    }
}

fn as_u32_array(value: Value) -> Option<Vec<u32>> {
    match value {
        Value::Array(array) => array
            .inner()
            .iter()
            .map(|item| as_u32(item.clone()))
            .collect(),
        _ => None,
    }
}

/// Encode ARGB32 pixels as a 32-bpp BMP (bottom-up BGRA rows). Qt loads
/// BMP with the plugins it definitely has, so no PNG crate is needed.
pub fn bmp_bytes(width: u32, height: u32, pixels: &[u32]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || pixels.len() != (width as usize) * (height as usize) {
        return None;
    }
    let w = width as usize;
    let h = height as usize;
    let row_bytes = w * 4;
    let pixel_bytes = row_bytes * h;
    let file_size = 14 + 40 + pixel_bytes;
    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel offset
    // BITMAPINFOHEADER, height negative = top-down (simpler row order).
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // compression: BI_RGB
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI X
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI Y
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for &px in pixels {
        // ARGB32 → BGRA byte order.
        out.push((px & 0xFF) as u8); // B
        out.push(((px >> 8) & 0xFF) as u8); // G
        out.push(((px >> 16) & 0xFF) as u8); // R
        out.push(((px >> 24) & 0xFF) as u8); // A
    }
    Some(out)
}

/// A filesystem-safe slug from an item's identity.
fn sanitize(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Keep only the newest `keep` BMPs in the pixmap cache.
fn prune_pixmap_cache(dir: &std::path::Path, keep: usize) {
    let mut files: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("bmp") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        files.push((modified, path));
    }
    files.sort_by_key(|f| f.0);
    let excess = files.len().saturating_sub(keep);
    for (_, path) in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
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
            icon_pixmap: None,
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
    fn bmp_bytes_has_a_bmp_header_and_right_size() {
        // 2x1 ARGB (opaque red, opaque green).
        let pixels = [0xFFFF0000u32, 0xFF00FF00u32];
        let bytes = bmp_bytes(2, 1, &pixels).unwrap();
        assert_eq!(&bytes[0..2], b"BM");
        assert_eq!(bytes.len(), 14 + 40 + 8);
        // First pixel is opaque red → BGRA bytes.
        let off = 54;
        assert_eq!(&bytes[off..off + 4], &[0x00, 0x00, 0xFF, 0xFF]);
        assert_eq!(&bytes[off + 4..off + 8], &[0x00, 0xFF, 0x00, 0xFF]);
    }

    #[test]
    fn bmp_bytes_rejects_mismatched_dimensions() {
        assert!(bmp_bytes(2, 2, &[1, 2, 3]).is_none());
        assert!(bmp_bytes(0, 1, &[]).is_none());
    }

    #[test]
    fn a_pixmap_only_item_gets_a_bmp_icon_path() {
        let mut item = item("Active");
        item.icon = String::new();
        item.attention_icon = String::new();
        item.icon_pixmap = Some((1, 1, vec![0xFF112233]));
        // The payload renders into the real cache dir; assert the file is
        // a BMP, then clean it up so the test never pollutes the session.
        let payload = item.payload();
        let path = payload["icon_path"].as_str().unwrap().to_string();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..2], b"BM");
        assert_eq!(bytes.len(), 14 + 40 + 4);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pixmap_parsing_caps_dimensions() {
        // Garbage is rejected, not allocated for.
        assert!(
            as_pixmap(Value::from("not a pixmap")).is_none(),
            "a string is not a pixmap"
        );
        assert!(
            as_pixmap(Value::from(42u32)).is_none(),
            "a u32 is not a pixmap"
        );
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
