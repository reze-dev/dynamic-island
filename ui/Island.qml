import QtQuick

Rectangle {
    id: island

    property bool expanded: false
    property int view: 0
    property bool animateSelf: true

    readonly property int targetWidth: expanded ? Theme.panelWidth : Theme.compactWidth
    readonly property int targetHeight: expanded ? Theme.panelHeight : Theme.compactHeight

    width: animateSelf ? targetWidth : implicitWidth
    height: animateSelf ? targetHeight : implicitHeight
    implicitWidth: targetWidth
    implicitHeight: targetHeight

    color: Theme.surface
    radius: expanded ? Theme.radiusPanel : Theme.radiusPill
    border.width: 1
    border.color: Theme.border

    Behavior on width {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }
    Behavior on height {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }
    Behavior on radius {
        NumberAnimation {
            duration: Theme.morphDuration
            easing.type: Easing.OutQuart
        }
    }

    Keys.onEscapePressed: expanded = false

    Column {
        anchors.centerIn: parent
        spacing: Theme.spacingTight

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            color: Theme.text
            font.family: "Inter"
            font.pixelSize: island.expanded ? 0 : 14
            font.weight: Font.DemiBold
            opacity: island.expanded ? 0 : 1
            text: "compact pill"

            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.crossfadeDuration
                    easing.type: Easing.OutCubic
                }
            }
        }

        PanelHost {
            anchors.fill: parent
            view: island.view
            opacity: island.expanded ? 1 : 0
            visible: opacity > 0

            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.crossfadeDuration
                    easing.type: Easing.OutCubic
                }
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: island.expanded = !island.expanded
    }
}