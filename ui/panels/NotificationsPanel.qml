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
        note: panel.notifications.length > 0
            ? panel.notifications.length + " · " + panel.unreadCount + " new"
            : ""
    }

    Rectangle {
        id: clearButton
        anchors.top: header.bottom
        anchors.right: parent.right
        anchors.topMargin: Theme.spacingTight
        anchors.rightMargin: Theme.spacing
        width: clearLabel.implicitWidth + 20
        height: 24
        radius: Theme.radiusSmall
        color: clearArea.containsMouse ? Theme.cardHover : Theme.card
        border.width: 1
        border.color: Theme.border
        visible: panel.notifications.length > 0

        Text {
            id: clearLabel
            anchors.centerIn: parent
            text: "Clear all"
            color: Theme.textDim
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            font.weight: Font.Medium
        }

        MouseArea {
            id: clearArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: panel.bridge ? panel.bridge.clearNotifications() : null
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
            color: Theme.textFaint
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: panel.holdingBusName
                ? "Nothing yet. Anything calling org.freedesktop.Notifications lands here."
                : "isled is not holding the bus name, so nothing else can be seen here."
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontSmall
            horizontalAlignment: Text.AlignHCenter
            width: 340
            wrapMode: Text.WordWrap
        }
    }

    ListView {
        id: list
        anchors.top: clearButton.visible ? clearButton.bottom : header.bottom
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
            radius: Theme.radiusSmall

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                color: "transparent"
                border.width: 1
                border.color: entry.modelData.urgency === 2
                    ? Qt.rgba(Theme.toneCritical.r, Theme.toneCritical.g, Theme.toneCritical.b, 0.35)
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
                    width: 26
                    height: 26
                    radius: 8
                    color: Qt.rgba(Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info").r,
                        Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info").g,
                        Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info").b, 0.16)
                    anchors.verticalCenter: parent.verticalCenter

                    Icon {
                        anchors.centerIn: parent
                        name: "bell"
                        width: 13
                        height: 13
                        color: Theme.toneColor(entry.modelData.urgency === 2 ? "critical" : "info")
                    }
                }

                Column {
                    width: Math.max(0, parent.width - 130)
                    spacing: 2

                    Row {
                        width: parent.width
                        spacing: Theme.spacingTight

                        Text {
                            text: entry.modelData.app_name ? entry.modelData.app_name : "unknown"
                            color: Theme.textDim
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontTiny
                            font.weight: Font.Medium
                            elide: Text.ElideRight
                            width: Math.min(implicitWidth, 90)
                        }

                        Text {
                            text: Format.ago(entry.modelData.received_at_unix)
                            color: Theme.textFaint
                            font.family: Theme.fontFamily
                            font.pixelSize: Theme.fontTiny
                        }
                    }

                    Text {
                        width: parent.width
                        text: entry.modelData.summary ? entry.modelData.summary : "(no summary)"
                        color: Theme.text
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        visible: entry.modelData.body !== ""
                        text: entry.modelData.body
                        color: Theme.textDim
                        font.family: Theme.fontFamily
                        font.pixelSize: Theme.fontSmall
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
                    anchors.verticalCenter: parent.verticalCenter
                    text: "dismiss"
                    color: dismissArea.containsMouse ? Theme.text : Theme.textFaint
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTiny
                    font.weight: Font.Medium

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
