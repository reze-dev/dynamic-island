import QtQuick
import QtQuick.Shapes
import ".."

// A ring that fills as the pomodoro runs.
//
// The sweep is a real arc rather than a trick with two half circles, so it
// can show any fraction without a visible seam.
Item {
    id: root

    property real progress: 0
    property color tone: Theme.toneGood
    property real thickness: 4
    property string caption: ""
    property string subcaption: ""

    implicitWidth: 96
    implicitHeight: 96

    readonly property real radius: Math.max(1, Math.min(width, height) / 2 - thickness / 2)

    // The empty track.
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeColor: Qt.rgba(1, 1, 1, 0.08)
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: root.radius
                radiusY: root.radius
                startAngle: 0
                sweepAngle: 360
            }
        }
    }

    // The filled part.
    Shape {
        anchors.fill: parent
        visible: root.progress > 0.001
        ShapePath {
            strokeColor: root.tone
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: root.radius
                radiusY: root.radius
                // Twelve o'clock, sweeping clockwise.
                startAngle: 90
                sweepAngle: -Math.max(0, Math.min(1, root.progress)) * 360
            }
        }
    }

    Column {
        anchors.centerIn: parent
        spacing: 1

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.caption
            color: Theme.text
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontTitle
            font.weight: Font.DemiBold
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.subcaption !== ""
            text: root.subcaption
            color: Theme.textDim
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            font.letterSpacing: 0.6
        }
    }
}
