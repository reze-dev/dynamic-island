import QtQuick

// A styled text input, for when the shell needs to take a line of text.
//
// Left over from the focus investigation: it is the smallest thing that can
// hold a TextInput and be handed a focus, with a placeholder that does not
// fight the text for room.
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
        color: Theme.onSurface
        font.family: Theme.fontFamily
        font.pixelSize: Theme.type.bodyLarge.size
        font.weight: Theme.type.bodyLarge.weight
        font.letterSpacing: Theme.type.bodyLarge.tracking
        selectByMouse: true
        selectionColor: Theme.primaryContainerDim
        selectedTextColor: Theme.onSurface
        clip: true

        onAccepted: scope.submitted(text)

        Keys.onEscapePressed: (event) => {
            scope.escapePressed();
            event.accepted = true;
        }
    }

    Text {
        anchors.fill: parent
        visible: scope.showPlaceholder
        color: Theme.onSurfaceVariant
        font: input.font
        text: scope.placeholder
    }
}
