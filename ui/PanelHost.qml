import QtQuick

Item {
    id: host

    property int view: 0

    readonly property string title: PanelCatalog.names[view] ?? "?"
    readonly property string subtitle: PanelCatalog.subtitles[view] ?? ""

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing

        Text {
            text: host.title
            color: Theme.text
            font.family: "Inter"
            font.pixelSize: 24
            font.weight: Font.Bold
        }

        Text {
            text: host.subtitle
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 13
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.border
        }

        Text {
            text: "Mod+Alt+" + (host.view + 1) + " · placeholder — phase 1"
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 12
        }

        Item {
            width: parent.width
            height: parent.height - y
        }
    }
}