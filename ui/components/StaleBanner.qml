import QtQuick
import ".."

// A cached-payload staleness banner.
//
// When the daemon is unreachable the panels keep showing the last data
// they loaded — honest, but the reader cannot tell it is stale without a
// banner. This strip appears under the panel header while the bridge is
// offline, and mentions the offline queue when intents are waiting.
//
// The banner is layout-safe: it reserves its height only while visible,
// and the panel's first element anchors to its bottom, so nothing shifts
// or overlaps when the daemon is up.
Item {
    id: root

    property var bridge: null
    readonly property bool stale: !!bridge && bridge.connected === false

    visible: stale
    height: stale ? 28 : 0
    clip: true

    Accessible.role: Accessible.Alert
    Accessible.name: labelText.text

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSmall
        color: Qt.rgba(Theme.toneWarn.r, Theme.toneWarn.g, Theme.toneWarn.b, 0.12)
        border.width: 1
        border.color: Qt.rgba(Theme.toneWarn.r, Theme.toneWarn.g, Theme.toneWarn.b, 0.45)
    }

    Row {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            name: "alert"
            width: 16
            height: 16
            color: Theme.toneWarn
        }

        Text {
            id: labelText
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - 32
            text: {
                var base = "Cached data — isled not answering";
                var queued = bridge && bridge.queuedCount ? bridge.queuedCount : 0;
                if (queued > 0)
                    base += " (" + queued + " queued)";
                return base;
            }
            color: Theme.onSurface
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
            elide: Text.ElideRight
        }
    }
}
