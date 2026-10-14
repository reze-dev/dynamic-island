import QtQuick

FocusScope {
    id: scope

    property alias text: input.text
    property string placeholder: "type here…"
    property bool showPlaceholder: input.text.length === 0

    signal submitted(string value)
    signal escapePressed()

    implicitWidth: input.implicitWidth
    implicitHeight: input.implicitHeight

    TextInput {
        id: input

        anchors.fill: parent
        color: Theme.text
        font.family: "Inter"
        font.pixelSize: 15
        selectByMouse: true
        selectionColor: Theme.accent
        clip: true

        onAccepted: scope.submitted(text)

        Keys.onEscapePressed: {
            scope.escapePressed();
            event.accepted = true;
        }
    }

    Text {
        anchors.fill: parent
        visible: scope.showPlaceholder
        color: Theme.textDim
        font: input.font
        text: scope.placeholder
    }
}