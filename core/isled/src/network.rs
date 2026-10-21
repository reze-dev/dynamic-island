//! NetworkManager over D-Bus.
//!
//! Quickshell's own networking service reads NetworkManager perfectly well,
//! and the panel uses it for everything it can display. The one thing it
//! cannot do is write: there is no QML-facing radio switch. So the daemon
//! covers that gap, and only that gap.
//!
//! `zbus` is already a dependency, because the notification daemon needs it,
//! so this costs nothing new.

use zbus::zvariant::Value;

/// The NetworkManager bus name.
const NM_BUS_NAME: &str = "org.freedesktop.NetworkManager";
/// The NetworkManager object path.
const NM_OBJECT_PATH: &str = "/org/freedesktop/NetworkManager";

/// Turn the wireless radio on or off.
///
/// NetworkManager exposes this as the `WirelessEnabled` property on its main
/// interface, which is written through the standard properties interface.
pub async fn set_wifi_enabled(enabled: bool) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| format!("could not reach the session bus: {e}"))?;

    let proxy = zbus::Proxy::new(
        &connection,
        NM_BUS_NAME,
        NM_OBJECT_PATH,
        "org.freedesktop.DBus.Properties",
    )
    .await
    .map_err(|e| format!("could not reach NetworkManager: {e}"))?;

    let value: Value = enabled.into();
    proxy
        .call_method("Set", &(NM_BUS_NAME, "WirelessEnabled", value))
        .await
        .map_err(|e| format!("NetworkManager refused the change: {e}"))?;

    Ok(())
}

/// Whether NetworkManager is on the bus at all.
///
/// The panel shows a different state when there is no network manager, so
/// that "no networks" is not mistaken for "no wifi adapter".
pub async fn present() -> bool {
    let Ok(connection) = zbus::Connection::session().await else {
        return false;
    };
    zbus::Proxy::new(
        &connection,
        NM_BUS_NAME,
        NM_OBJECT_PATH,
        "org.freedesktop.NetworkManager",
    )
    .await
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bus_name_is_the_one_networkmanager_uses() {
        assert_eq!(NM_BUS_NAME, "org.freedesktop.NetworkManager");
        assert_eq!(NM_OBJECT_PATH, "/org/freedesktop/NetworkManager");
    }
}
