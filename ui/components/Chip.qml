import QtQuick
import ".."

// A chip: a small piece of text in a rounded box, with an optional glyph.
//
// Material has several kinds. The assist chip carries an icon that acts on
// the thing it sits next to. The filter chip is a toggle, and shows a tick
// when it is on. The suggestion chip is a shortcut to something.
//
// What they share is the shape: eight pixels of corner, a one pixel
// outline, and a label in the middle.
Rectangle {
    id: root

    property string text: ""
    /// The glyph, or the empty string.
    property string icon: ""
    /// Whether the chip is a filter that is switched on.
    property bool checked: false
    property bool filter: false
    property color tone: Theme.onSurfaceVariant
    property int horizontalPadding: 12

    implicitWidth: row.width + horizontalPadding * 2
    implicitHeight: 32
    radius: Theme.radiusSmall

    color: root.checked ? Theme.secondaryContainer : "transparent"
    border.width: 1
    border.color: root.checked ? "transparent" : Theme.outlineVariant

    Behavior on color {
        ColorAnimation {
            duration: Theme.stateLayerDuration
        }
    }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.icon !== ""
            name: root.icon
            width: 18
            height: width
            color: root.checked ? Theme.onSecondaryContainer : root.tone
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.text
            color: root.checked ? Theme.onSecondaryContainer : root.tone
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge.size
            font.weight: Theme.type.labelLarge.weight
            font.letterSpacing: Theme.type.labelLarge.tracking
        }
    }
}
