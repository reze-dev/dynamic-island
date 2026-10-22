import QtQuick
import ".."
import "../components"

// The notification history.
//
// The daemon owns the list; this is the reading of it. Each notification
// knows its urgency, and a critical one is tinted so it stands out from the
// pile without needing to be read.
Item {
    id: panel

    property var bridge: null

    readonly property var notifications: bridge && bridge.notifications ? bridge.notifications : []
    readonly property int unreadCount: bridge ? bridge.unreadCount : 0
    readonly property bool holdingBusName: bridge && bridge.snapshot && bridge.snapshot.features
        ? bridge.snapshot.features.notifications === true
        : false

    SectionHeader {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: Theme.spacing
        title: "History"
    }

    // The count and the action share the header's trailing edge. Below the
    // header they stacked up under the count and read as two right-aligned
    // things fighting for the same corner.
    Row {
        id: trailing
        anchors.verticalCenter: header.verticalCenter
        anchors.right: parent.right
        anchors.rightMargin: Theme.spacing
        spacing: Theme.spacingTight + 2
        visible: panel.notifications.length > 0

        Text {
            id: countLabel
            anchors.verticalCenter: parent.verticalCenter
            text: panel.notifications.length + " · " + panel.unreadCount + " new"
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
        }

        Rectangle {
            id: clearButton
            anchors.verticalCenter: parent.verticalCenter
            width: clearLabel.implicitWidth + 20
            height: 24
            radius: Theme.radiusSmall
            color: clearArea.containsMouse ? Theme.surfaceFillHover : Theme.surfaceFill
            border.width: 1
            border.color: Theme.outlineVariant

            Text {
                id: clearLabel
                anchors.centerIn: parent
                text: "Clear all"
                color: Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.labelSmall.size
                font.weight: Theme.type.labelSmall.weight
                font.letterSpacing: Theme.type.labelSmall.tracking
            }

            MouseArea {
                id: clearArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: panel.bridge
                    ? panel.bridge.callAndSay("notifications.clear", {}, "History cleared")
                    : null
            }
        }
    }

    // The empty state has to be honest about why it is empty.
    Column {
        anchors.centerIn: parent
        spacing: Theme.spacingTight
        visible: panel.notifications.length === 0

        Icon {
            anchors.horizontalCenter: parent.horizontalCenter
            name: "bell"
            width: 22
            height: 22
            color: Theme.onSurfaceVariant
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: panel.holdingBusName
                ? "Nothing yet. Anything calling org.freedesktop.Notifications lands here."
                : "isled is not holding the bus name, so nothing else can be seen here."
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodySmall.size
            font.weight: Theme.type.bodySmall.weight
            font.letterSpacing: Theme.type.bodySmall.tracking
            horizontalAlignment: Text.AlignHCenter
            width: 340
            wrapMode: Text.WordWrap
        }
    }

    ListView {
        id: list
        anchors.top: header.bottom
        anchors.topMargin: Theme.spacing
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.bottomMargin: Theme.spacing
        clip: true
        spacing: 6
        model: panel.notifications

        delegate: Card {
            id: entry
            required property var modelData

            width: list.width
            height: row.implicitHeight + Theme.spacingTight * 2
            radius: Theme.radiusMedium

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                color: "transparent"
                border.width: 1
                border.color: entry.modelData.urgency === 2
                    ? Theme.stateLayer(Theme.toneCritical, 0.35)
                    : "transparent"
            }

            Row {
                id: row
                anchors.left: parent.left
                anchors.leftMargin: Theme.spacingTight + 2
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacingTight + 2
                anchors.top: parent.top
                anchors.topMargin: Theme.spacingTight
                spacing: Theme.spacingTight + 2

                Rectangle {
                    id: noticeIcon
                    width: 28
                    height: 28
                    radius: width / 2
                    color: Theme.stateLayer(Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info"), 0.16)
                    anchors.verticalCenter: parent.verticalCenter

                    Icon {
                        anchors.centerIn: parent
                        name: "bell"
                        width: 14
                        height: 14
                        color: Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info")
                    }
                }

                Column {
                    // What is left once the icon and the dismiss link have
                    // had their share, so the link sits at the right edge
                    // instead of floating in the middle of the row.
                    width: Math.max(0, row.width - noticeIcon.width
                        - dismissText.width - row.spacing * 2)
                    spacing: 2

                    Row {
                        width: parent.width
                        spacing: Theme.spacingTight

                        Text {
                            text: entry.modelData.app_name ? entry.modelData.app_name : "unknown"
                            color: Theme.onSurfaceVariant
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.labelSmall.size
                            font.weight: Theme.type.labelSmall.weight
                            font.letterSpacing: Theme.type.labelSmall.tracking
                            elide: Text.ElideRight
                            width: Math.min(implicitWidth, 90)
                        }

                        Text {
                            text: Format.ago(entry.modelData.received_at_unix)
                            color: Theme.onSurfaceVariant
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.type.labelSmall.size
                            font.weight: Theme.type.labelSmall.weight
                            font.letterSpacing: Theme.type.labelSmall.tracking
                        }
                    }

                    Text {
                        width: parent.width
                        text: entry.modelData.summary ? entry.modelData.summary : "(no summary)"
                        color: Theme.onSurface
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.bodySmall.size
                        font.weight: Theme.type.bodySmall.weight
                        font.letterSpacing: Theme.type.bodySmall.tracking
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        visible: entry.modelData.body !== ""
                        text: entry.modelData.body
                        color: Theme.onSurfaceVariant
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.type.bodySmall.size
                        font.weight: Theme.type.bodySmall.weight
                        font.letterSpacing: Theme.type.bodySmall.tracking
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        wrapMode: Text.WordWrap
                    }

                    Flow {
                        width: parent.width
                        spacing: 6
                        visible: !!(entry.modelData.actions && entry.modelData.actions.length > 0)

                        Repeater {
                            model: entry.modelData.actions ? entry.modelData.actions : []

                            Chip {
                                required property var modelData
                                text: modelData.label ? modelData.label : modelData.key
                                tone: Theme.toneInfo

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: panel.bridge
                                        ? panel.bridge.invokeAction(entry.modelData.id, modelData.key)
                                        : null
                                }
                            }
                        }
                    }
                }

                Text {
                    id: dismissText
                    anchors.verticalCenter: parent.verticalCenter
                    text: "dismiss"
                    color: dismissArea.containsMouse ? Theme.onSurface : Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking

                    MouseArea {
                        id: dismissArea
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: panel.bridge ? panel.bridge.dismissNotification(entry.modelData.id) : null
                    }
                }
            }
        }
    }
}
