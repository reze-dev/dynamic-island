import QtQuick
import ".."

// The title of a panel section, with an optional right-hand note.
//
// Material's list subheader: a small label in the quiet colour, with
// nothing else around it.
Item {
    id: root

    property string title: ""
    property string note: ""
    property color tone: Theme.onSurfaceVariant

    implicitHeight: 20

    Text {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        text: root.title
        color: root.tone
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelLarge.size
        font.weight: Theme.type.labelLarge.weight
        font.letterSpacing: Theme.type.labelLarge.tracking
    }

    Text {
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        visible: root.note !== ""
        text: root.note
        color: Theme.onSurfaceVariant
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.labelSmall.size
        font.weight: Theme.type.labelSmall.weight
        font.letterSpacing: Theme.type.labelSmall.tracking
    }
}
