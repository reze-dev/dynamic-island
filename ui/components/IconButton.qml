import QtQuick
import ".."

// A round, borderless button that lights up under the pointer.
Item {
    id: root

    property string icon: "spark"
    property color tone: Theme.textDim
    property bool selected: false
    property bool active: false
    property int size: 30
    property string tooltip: ""

    signal clicked()

    implicitWidth: size
    implicitHeight: size
    opacity: enabled ? 1 : 0.35
    enabled: true

    Rectangle {
        id: plate
        anchors.fill: parent
        radius: width / 2
        color: root.active
            ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.20)
            : (hover.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent")
        border.width: root.selected ? 1 : 0
        border.color: Theme.accent

        Behavior on color {
            ColorAnimation {
                duration: Theme.crossfadeDuration
            }
        }
    }

    Icon {
        anchors.centerIn: parent
        name: root.icon
        width: Math.round(root.size * 0.55)
        height: width
        color: root.selected || root.active ? Theme.accent : (hover.containsMouse ? Theme.text : root.tone)
        strokeWidth: 1.8
    }

    // The tooltip. Plain QML, because QtQuick.Controls is not linked into
    // the Quickshell engine.
    Rectangle {
        visible: hover.containsMouse && root.tooltip !== ""
        anchors.top: parent.bottom
        anchors.topMargin: 6
        anchors.horizontalCenter: parent.horizontalCenter
        width: tip.implicitWidth + 14
        height: 24
        radius: 7
        color: Theme.islandSurfacePanel
        border.width: 1
        border.color: Theme.border
        z: 40

        Text {
            id: tip
            anchors.centerIn: parent
            text: root.tooltip
            color: Theme.textDim
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
        }
    }

    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        enabled: root.enabled
        onClicked: root.clicked()
    }
}
