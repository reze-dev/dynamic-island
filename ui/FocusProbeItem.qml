import QtQuick

// The focus probe: a strip of text input that can be focused and released,
// for working out whether the shell is taking the keyboard at all.
Item {
    id: probe

    property alias typed: field.text
    readonly property bool hasFocus: field.activeFocus
    signal submitted(string value)
    signal escapePressed()

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
        color: Theme.surfaceFill
        radius: Theme.radiusMedium
        border.width: 1
        border.color: probe.hasFocus ? Theme.primary : Theme.outlineVariant

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
        onEscapePressed: probe.escapePressed()
    }
}
