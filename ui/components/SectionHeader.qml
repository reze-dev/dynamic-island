import QtQuick
import ".."

// The title of a panel section, with an optional right-hand note.
Item {
    id: root

    property string title: ""
    property string note: ""
    property color tone: Theme.text

    implicitHeight: 20

    Text {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        text: root.title
        color: root.tone
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontMedium
        font.weight: Font.DemiBold
        font.letterSpacing: -0.1
    }

    Text {
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        visible: root.note !== ""
        text: root.note
        color: Theme.textFaint
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontTiny
        font.weight: Font.Medium
        font.letterSpacing: 0.8
    }
}
