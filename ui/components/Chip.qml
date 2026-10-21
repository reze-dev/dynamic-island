import QtQuick
import ".."

// A small pill of text, used for kinds, states and counts.
Rectangle {
    id: root

    property string text: ""
    property color tone: Theme.accent
    property bool subtle: false
    property int horizontalPadding: 7

    implicitWidth: label.implicitWidth + horizontalPadding * 2
    implicitHeight: 20
    radius: 10

    color: subtle ? "transparent" : Qt.rgba(tone.r, tone.g, tone.b, 0.13)
    border.width: subtle ? 1 : 0
    border.color: Qt.rgba(tone.r, tone.g, tone.b, 0.22)

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        color: root.tone
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontTiny
        font.weight: Font.DemiBold
        font.letterSpacing: 0.4
    }
}
