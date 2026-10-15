import QtQuick
import ".."

Rectangle {
    id: panel

    property var notifications: []
    property int unreadCount: 0
    property bool holdingBusName: false

    signal dismissed(int id)
    signal clearedAll()
    signal actionInvoked(int id, string key)

    color: "transparent"

    function ago(timestamp) {
        if (timestamp === undefined || timestamp === null)
            return "";
        var seconds = Math.floor(Date.now() / 1000) - timestamp;
        if (seconds < 0)
            seconds = 0;
        if (seconds < 5)
            return "now";
        if (seconds < 60)
            return seconds + "s";
        var minutes = Math.floor(seconds / 60);
        if (minutes < 60)
            return minutes + "m";
        var hours = Math.floor(minutes / 60);
        if (hours < 24)
            return hours + "h";
        return Math.floor(hours / 24) + "d";
    }

    Column {
        anchors.fill: parent
        anchors.margins: Theme.spacingLoose
        spacing: Theme.spacing

        Row {
            width: parent.width
            height: 30

            Text {
                text: "Notifications"
                color: Theme.text
                font.family: "Inter"
                font.pixelSize: 24
                font.weight: Font.Bold
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: panel.notifications.length > 0
                color: Theme.textDim
                font.family: "Inter"
                font.pixelSize: 13
                text: "   " + panel.notifications.length
                    + (panel.unreadCount > 0 ? "  ·  " + panel.unreadCount + " new" : "")
            }

            Item {
                width: parent.width - 170
                height: 30
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: panel.notifications.length > 0
                color: Theme.accent
                font.family: "Inter"
                font.pixelSize: 12
                text: "Clear all"

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: panel.clearedAll()
                }
            }
        }

        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            visible: panel.notifications.length === 0
            color: Theme.textDim
            font.family: "Inter"
            font.pixelSize: 13
            text: panel.holdingBusName
                ? "Nothing yet. Anything calling org.freedesktop.Notifications lands here."
                : "Idle — isled is not holding the bus name, so nothing arrives here yet."
        }

        ListView {
            width: parent.width
            height: Math.max(0, panel.height - 120)
            clip: true
            spacing: Theme.spacingTight
            visible: panel.notifications.length > 0
            model: panel.notifications

            delegate: Rectangle {
                id: row

                required property var modelData

                width: ListView.view.width
                height: body.height + Theme.spacing
                radius: 14
                color: modelData.urgency === 2
                    ? Qt.rgba(0.62, 0.30, 0.36, 0.32)
                    : Theme.surfaceRaised
                border.width: 1
                border.color: Theme.border

                Column {
                    id: body

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.spacingTight
                    spacing: 5

                    Row {
                        width: body.width
                        height: 14

                        Rectangle {
                            width: 5
                            height: 5
                            anchors.verticalCenter: parent.verticalCenter
                            radius: 3
                            color: modelData.urgency === 2
                                ? Qt.rgba(0.96, 0.42, 0.48, 1)
                                : Theme.accent
                        }

                        Item {
                            width: 7
                            height: 14
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                            text: modelData.app_name || "unknown"
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 11
                            text: "   " + ago(modelData.received_at_unix)
                        }
                    }

                    Text {
                        width: body.width
                        color: Theme.text
                        font.family: "Inter"
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        text: modelData.summary || ""
                    }

                    Text {
                        width: body.width
                        wrapMode: Text.WordWrap
                        visible: (modelData.body || "") !== ""
                        color: Theme.textDim
                        font.family: "Inter"
                        font.pixelSize: 12
                        maximumLineCount: 3
                        text: modelData.body || ""
                    }

                    Row {
                        width: body.width
                        height: 16
                        visible: (modelData.actions || []).length > 0

                        Repeater {
                            model: modelData.actions || []

                            delegate: Text {
                                required property var modelData

                                anchors.verticalCenter: parent.verticalCenter
                                color: Theme.accent
                                font.family: "Inter"
                                font.pixelSize: 11
                                text: modelData.label || modelData.key

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: panel.actionInvoked(row.modelData.id, modelData.key)
                                }
                            }

                            Text {
                                width: 14
                                height: 16
                            }
                        }

                        Item {
                            width: Math.max(0, body.width - 100)
                            height: 16
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.textDim
                            font.family: "Inter"
                            font.pixelSize: 11
                            text: "Dismiss"

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: panel.dismissed(row.modelData.id)
                            }
                        }
                    }
                }
            }
        }
    }
}
