import QtQuick
import ".."

// A tile: one of the small squares that say what a thing is and whether it
// is on.
//
// Material does not have a component by this name, but it has the pattern:
// a leading icon, a headline, a supporting line, and a colour that says
// whether the thing is live. A grid of these reads at a glance, which is
// what the mock's network panel is built on.
Rectangle {
    id: root

    property string label: ""
    property string detail: ""
    property string icon: ""
    /// Whether the thing this tile stands for is live.
    property bool active: false
    /// An optional control on the trailing edge, such as a switch.
    property Item trailing: null

    implicitHeight: 56
    radius: Theme.radiusLarge
    color: active ? Theme.primaryDim : Theme.surfaceFill
    border.width: 1
    border.color: active ? Theme.primaryContainerDim : "transparent"

    Behavior on color {
        ColorAnimation {
            duration: Theme.stateLayerDuration
        }
    }

    Icon {
        id: glyph
        anchors.left: parent.left
        anchors.leftMargin: Theme.spacingTight + 4
        anchors.verticalCenter: parent.verticalCenter
        name: root.icon
        width: 20
        height: 20
        color: root.active ? Theme.primary : Theme.onSurfaceVariant
    }

    Column {
        anchors.left: glyph.right
        anchors.leftMargin: Theme.spacingTight + 3
        anchors.right: root.trailing ? root.trailing.left : parent.right
        anchors.rightMargin: root.trailing ? Theme.spacingTight : Theme.spacingTight + 4
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Text {
            width: parent.width
            text: root.label
            color: Theme.onSurface
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodyMedium.size
            font.weight: Theme.type.bodyMedium.weight
            font.letterSpacing: Theme.type.bodyMedium.tracking
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            visible: root.detail !== ""
            text: root.detail
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
            elide: Text.ElideRight
        }
    }
}
