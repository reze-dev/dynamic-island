import QtQuick
import ".."

// One number with a label under it. Used in the context panel and the
// peek view.
Item {
    id: root

    property string label: ""
    property string value: "—"
    property string detail: ""
    property color tone: Theme.onSurface

    implicitHeight: column.implicitHeight

    Column {
        id: column
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 2

        Text {
            text: root.value
            color: root.tone
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.titleLarge.size
            font.weight: Theme.type.titleLarge.weight
            font.letterSpacing: Theme.type.titleLarge.tracking
            elide: Text.ElideRight
            width: parent.width
        }

        Text {
            text: root.label
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
            elide: Text.ElideRight
            width: parent.width
        }

        Text {
            visible: root.detail !== ""
            text: root.detail
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodySmall.size
            font.letterSpacing: Theme.type.bodySmall.tracking
            elide: Text.ElideRight
            width: parent.width
        }
    }
}
