import QtQuick
import ".."

// A switch. Built from a rectangle and a knob, because QtQuick.Controls is
// not available.
Item {
    id: root

    property bool checked: false
    property color tone: Theme.accent

    signal toggled(bool value)

    implicitWidth: 36
    implicitHeight: 20
    opacity: enabled ? 1 : 0.4
    enabled: true

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: root.checked
            ? Qt.rgba(root.tone.r, root.tone.g, root.tone.b, 0.45)
            : Qt.rgba(1, 1, 1, 0.09)
        border.width: 1
        border.color: root.checked ? Qt.rgba(root.tone.r, root.tone.g, root.tone.b, 0.55) : Theme.border

        Behavior on color {
            ColorAnimation {
                duration: Theme.crossfadeDuration
            }
        }
    }

    Rectangle {
        id: knob
        width: track.height - 4
        height: width
        radius: width / 2
        y: (track.height - height) / 2
        x: root.checked ? track.width - width - 2 : 2
        color: root.checked ? root.tone : Theme.textDim

        Behavior on x {
            NumberAnimation {
                duration: Theme.crossfadeDuration
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
