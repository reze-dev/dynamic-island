import QtQuick
import ".."
import "../components"

// The system tray.
//
// The items are drawn as a row of icons, the way a tray is: the icon is the
// thing you recognise, and the title is a tooltip rather than a label. An
// item that wants attention is tinted, and the icon it asks for is the one
// that is drawn.
Item {
    id: panel

    property var bridge: null

    // The bridge owns the list, because the daemon pushes it whenever an
    // item appears or changes. A copy taken once would go stale the moment
    // an application did something.
    readonly property var items: bridge && bridge.tray ? bridge.tray : []

    readonly property var visibleItems: {
        var list = [];
        for (var i = 0; i < panel.items.length; i++) {
            if (panel.items[i].status !== "Passive")
                list.push(panel.items[i]);
        }
        return list;
    }

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "System tray"
        note: panel.items.length > 0 ? panel.items.length + " ITEMS" : ""
    }

    Text {
        anchors.centerIn: parent
        visible: panel.items.length === 0
        text: "Nothing in the tray."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Text {
        anchors.centerIn: parent
        visible: panel.items.length > 0 && panel.visibleItems.length === 0
        text: "Every item is passive, so none of them are showing."
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodySmall.size
        font.weight: Theme.type.bodySmall.weight
        font.letterSpacing: Theme.type.bodySmall.tracking
    }

    Flow {
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        spacing: Theme.spacingTight

        Repeater {
            model: panel.visibleItems

            Card {
                required property var modelData

                width: 92
                height: 92
                radius: Theme.radiusMedium
                selected: false

                Column {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingTight
                    spacing: 4

                    // The icon. An item that names a file gets it; one that
                    // only names an icon gets a generic glyph, because the
                    // theme may not have it.
                    Item {
                        width: parent.width
                        height: 40

                        Image {
                            id: iconImage
                            anchors.centerIn: parent
                            width: 36
                            height: 36
                            visible: modelData.icon_path !== undefined
                                     && modelData.icon_path !== null
                                     && modelData.icon_path !== ""
                            source: visible ? "file://" + modelData.icon_path : ""
                            asynchronous: true
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                        }

                        Icon {
                            anchors.centerIn: parent
                            visible: !iconImage.visible
                            name: "grid"
                            width: 26
                            height: 26
                            color: Theme.onSurfaceVariant
                        }
                    }

                    Text {
                        width: parent.width
                        text: modelData.title
                        color: Theme.onSurface
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        text: modelData.category === "ApplicationStatus"
                                || modelData.category === ""
                            ? "application"
                            : modelData.category
                        color: Theme.onSurfaceVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }
                }

                // An item that wants attention is tinted, so it is not
                // missed among the rest.
                Rectangle {
                    anchors.fill: parent
                    radius: parent.radius
                    color: "transparent"
                    border.width: modelData.status === "NeedsAttention" ? 1 : 0
                    border.color: Theme.toneWarn
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (panel.bridge) {
                            panel.bridge.call("tray.activate", {
                                "service": modelData.service,
                                "x": 0,
                                "y": 0
                            });
                        }
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: -6
                    radius: Theme.radiusMedium
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.primary
                    visible: tooltipArea.containsMouse
                }

                MouseArea {
                    id: tooltipArea
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }

                // The tooltip.
                Rectangle {
                    visible: tooltipArea.containsMouse && modelData.tooltip !== ""
                    anchors.bottom: parent.top
                    anchors.bottomMargin: 6
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: tip.implicitWidth + 20
                    height: 28
                    radius: Theme.radiusSmall
                    color: Theme.surfaceContainerHighest
                    border.width: 1
                    border.color: Theme.outlineVariant
                    z: 40

                    Text {
                        id: tip
                        anchors.centerIn: parent
                        text: modelData.tooltip
                        color: Theme.onSurfaceVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.labelSmall.size
                        font.weight: Theme.type.labelSmall.weight
                        font.letterSpacing: Theme.type.labelSmall.tracking
                    }
                }
            }
        }
    }
}
