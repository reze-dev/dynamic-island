import QtQuick

Item {
    id: probe

    property alias typed: field.text
    readonly property bool hasFocus: field.activeFocus
    signal submitted(string value)

    width: 420
    height: 44

    function grabFocus() {
        field.forceActiveFocus(Qt.OtherFocusReason);
    }

    function releaseFocus() {
        field.focus = false;
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.surfaceRaised
        radius: 12
        border.width: 2
        border.color: probe.hasFocus ? Theme.accent : Theme.border

        Behavior on border.color {
            ColorAnimation {
                duration: Theme.crossfadeDuration
            }
        }
    }

    FocusProbe {
        id: field
        anchors.fill: parent
        anchors.margins: 11
        placeholder: "focus probe — Esc to release"
        onSubmitted: (value) => probe.submitted(value)
    }
}