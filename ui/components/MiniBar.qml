import QtQuick
import ".."

// One line of a bar graph, used by the peek view to show how loaded the
// machine is.
//
// Material's linear progress: a track with a rounded end and an indicator
// that fills it. The indicator carries a dot at its leading edge, which is
// what makes it read as a progress bar rather than a coloured rectangle.
//
// The item is as tall as the text beside it, and the bar sits in the middle
// of it. A Row does not align its children vertically, so a bar that is only
// as tall as itself would sit at the top of the line and float above its
// own label.
Item {
    id: root

    property real value: 0
    property color tone: Theme.primary
    property real barHeight: 4

    implicitHeight: 14

    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.barHeight
        radius: height / 2
        color: Theme.surfaceFillHighest
    }

    Rectangle {
        id: fill
        anchors.verticalCenter: track.verticalCenter
        anchors.left: track.left
        height: track.height
        radius: track.radius
        width: Math.max(0, Math.min(1, root.value)) * track.width
        color: root.tone

        Behavior on width {
            NumberAnimation {
                duration: Theme.crossfadeDuration
                easing: Theme.decelerate
            }
        }
    }
}
