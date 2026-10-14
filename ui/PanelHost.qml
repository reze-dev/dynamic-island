import QtQuick
import "panels"

Item {
    id: host

    property int view: 0
    property var stats: ({})
    property var snapshot: ({})

    readonly property bool hasContext: host.view === 0

    ContextPanel {
        anchors.fill: parent
        visible: host.hasContext
        enabled: host.hasContext
        stats: host.stats
        snapshot: host.snapshot
    }

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing
        visible: !host.hasContext

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
