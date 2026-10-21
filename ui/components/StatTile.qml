import QtQuick
import ".."

// One number with a label under it. Used in the context panel and the
// peek view.
Item {
    id: root

    property string label: ""
    property string value: "—"
    property string detail: ""
    property color tone: Theme.text

    implicitHeight: column.implicitHeight

    Column {
        id: column
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 3

        Text {
            text: root.value
            color: root.tone
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontLarge
            font.weight: Font.DemiBold
            elide: Text.ElideRight
            width: parent.width
        }

        Text {
            text: root.label
            color: Theme.textDim
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            font.weight: Font.Medium
            font.letterSpacing: 0.6
            elide: Text.ElideRight
            width: parent.width
        }

        Text {
            visible: root.detail !== ""
            text: root.detail
            color: Theme.textFaint
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTiny
            elide: Text.ElideRight
            width: parent.width
        }
    }
}
