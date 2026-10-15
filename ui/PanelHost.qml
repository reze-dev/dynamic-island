import QtQuick
import "panels"

Item {
    id: host

    property int view: 0
    property var stats: ({})
    property var snapshot: ({})

    readonly property var isled: (host.parent && host.parent.isled) ? host.parent.isled : null
    readonly property var notifications: host.isled ? host.isled.notifications : []
    readonly property int unreadCount: host.isled ? host.isled.unreadCount : 0
    readonly property bool holdingBusName: (host.isled && host.isled.snapshot.features)
        ? (host.isled.snapshot.features.notifications === true)
        : false

    ContextPanel {
        anchors.fill: parent
        visible: host.view === 0
        enabled: host.view === 0
        stats: host.stats
        snapshot: host.snapshot
    }

    NotificationsPanel {
        anchors.fill: parent
        visible: host.view === 2
        enabled: host.view === 2
        notifications: host.notifications
        unreadCount: host.unreadCount
        holdingBusName: host.holdingBusName

        onDismissed: (id) => {
            if (host.isled) {
                host.isled.dismissNotification(id);
                host.isled.clearUnread();
            }
        }
        onClearedAll: {
            if (host.isled) {
                host.isled.clearNotifications();
                host.isled.clearUnread();
            }
        }
        onActionInvoked: function(id, key) {
            if (host.isled)
                host.isled.invokeAction(id, key);
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing
        visible: host.view !== 0 && host.view !== 2

        Text {
            text: PanelCatalog.names[host.view] ?? "?"
            color: Theme.text
            font.family: "Inter"
            font.pixelSize: 24
            font.weight: Font.Bold
        }

        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 13
            text: PanelCatalog.subtitles[host.view] ?? ""
        }

        Text {
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 12
            text: "Mod+Ctrl+" + (host.view + 1) + " · placeholder"
        }

        Item {
            width: parent.width
            height: 1
        }
    }
}
