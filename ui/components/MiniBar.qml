import QtQuick
import ".."

// One line of a bar graph, used by the peek view to show how loaded the
// machine is.
Item {
    id: root

    property real value: 0
    property color tone: Theme.accent
    property real barHeight: 3

    implicitHeight: barHeight

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: Qt.rgba(1, 1, 1, 0.08)
    }

    Rectangle {
        id: fill
        width: Math.max(0, Math.min(1, root.value)) * parent.width
        height: parent.height
        radius: height / 2
        color: root.tone

        Behavior on width {
            NumberAnimation {
                duration: Theme.crossfadeDuration
                easing.type: Easing.OutQuart
            }
        }
    }
}
