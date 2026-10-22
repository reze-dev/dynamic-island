import QtQuick
import ".."

// A switch. Built from two rectangles, because QtQuick.Controls is not
// available.
//
// Material's switch is a wide track with a tall thumb: 52 by 32, with a
// thumb that grows when it is on. The thumb is not a circle, which is the
// detail that makes it read as a switch rather than a checkbox.
Item {
    id: root

    property bool checked: false
    property color tone: Theme.primary

    signal toggled(bool value)

    implicitWidth: 52
    implicitHeight: 32
    opacity: enabled ? 1 : 0.38

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: root.checked ? root.tone : Theme.surfaceContainerHighest
        border.width: root.checked ? 0 : 2
        border.color: Theme.outline

        Behavior on color {
            ColorAnimation {
                duration: Theme.stateLayerDuration
            }
        }
    }

    Rectangle {
        id: knob
        width: root.checked ? 24 : 16
        height: root.checked ? 24 : 16
        radius: root.checked ? 12 : 8
        color: root.checked ? Theme.onPrimary : Theme.outline
        y: (track.height - height) / 2
        x: root.checked ? track.width - width - 4 : 4

        Behavior on x {
            NumberAnimation {
                duration: Theme.stateLayerDuration
                easing.type: Easing.OutQuart
            }
        }

        Behavior on width {
            NumberAnimation {
                duration: Theme.stateLayerDuration
                easing.type: Easing.OutQuart
            }
        }

        Behavior on height {
            NumberAnimation {
                duration: Theme.stateLayerDuration
                easing.type: Easing.OutQuart
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.enabled
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: root.toggled(!root.checked)
    }
}
