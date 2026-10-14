import QtQuick

Item {
    id: host

    property int view: 0

    readonly property var names: [
        "context",
        "media",
        "notifications",
        "clipboard",
        "calendar",
        "todos",
        "notes",
        "network",
        "theme"
    ]

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing

        Text {
            text: host.names[host.view] ?? "?"
            color: Theme.text
            font.family: "Inter"
            font.pixelSize: 22
            font.weight: Font.Bold
        }

        Text {
            text: "phase 0 placeholder"
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 13
        }

        Item {
            width: parent.width
            height: parent.height - y
        }
    }
}