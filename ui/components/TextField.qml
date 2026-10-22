import QtQuick
import ".."

// A text field.
//
// Material has two. The filled one rests on a coloured field. The outlined
// one draws a border. Both carry a label above them, a leading glyph, and
// a line of support text below that turns into the error when there is
// one.
//
// The field is a plain QML TextInput, because QtQuick.Controls is not
// linked into the Quickshell engine.
Item {
    id: root

    property alias text: input.text
    property string placeholder: ""
    /// The label above the field.
    property string label: ""
    /// The glyph at the front of the field.
    property string icon: ""
    /// The glyph at the back of the field.
    property string trailingIcon: ""
    /// filled | outlined
    property string variant: "filled"
    /// The line under the field, and whether it is an error.
    property string support: ""
    property bool error: false
    property bool readOnly: false

    signal accepted()
    signal escapePressed()

    readonly property bool outlined: variant === "outlined"

    implicitHeight: column.implicitHeight
    implicitWidth: 200

    Column {
        id: column
        width: parent.width
        spacing: 4

        Text {
            visible: root.label !== ""
            text: root.label
            color: root.error ? Theme.error : Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
        }

        Rectangle {
            id: field
            width: parent.width
            height: 44
            radius: root.outlined ? Theme.radiusMedium : Theme.radiusMedium
            color: root.outlined ? "transparent" : Theme.surfaceFillLow
            border.width: root.outlined ? (input.activeFocus ? 2 : 1) : (input.activeFocus ? 2 : 0)
            border.color: root.error ? Theme.error
                : (input.activeFocus ? Theme.primary : Theme.outlineVariant)

            Behavior on border.color {
                ColorAnimation {
                    duration: Theme.stateLayerDuration
                }
            }

            Icon {
                id: lead
                anchors.left: parent.left
                anchors.leftMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                visible: root.icon !== ""
                name: root.icon
                width: 20
                height: width
                color: input.activeFocus ? Theme.primary : Theme.onSurfaceVariant
            }

            TextInput {
                id: input
                anchors.left: lead.visible ? lead.right : parent.left
                anchors.leftMargin: lead.visible ? 8 : 14
                anchors.right: trail.visible ? trail.left : parent.right
                anchors.rightMargin: trail.visible ? 8 : 14
                anchors.verticalCenter: parent.verticalCenter
                clip: true
                readOnly: root.readOnly
                color: Theme.onSurface
                selectionColor: Theme.primaryContainerDim
                selectedTextColor: Theme.onSurface
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.bodyLarge.size
                font.letterSpacing: Theme.type.bodyLarge.tracking
                selectByMouse: true

                onAccepted: root.accepted()
                Keys.onEscapePressed: (event) => {
                    root.escapePressed();
                    event.accepted = true;
                }
            }

            // The placeholder sits under the text, so that the text can be
            // typed over it without the field jumping.
            Text {
                anchors.left: input.left
                anchors.right: input.right
                anchors.verticalCenter: input.verticalCenter
                visible: input.text === "" && !input.activeFocus
                text: root.placeholder
                color: Theme.onSurfaceVariant
                font.family: Theme.fontFamily
                font.pixelSize: Theme.type.bodyLarge.size
                font.letterSpacing: Theme.type.bodyLarge.tracking
            }

            Icon {
                id: trail
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                visible: root.trailingIcon !== ""
                name: root.trailingIcon
                width: 20
                height: width
                color: Theme.onSurfaceVariant
            }
        }

        Text {
            visible: root.support !== ""
            text: root.support
            color: root.error ? Theme.error : Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodySmall.size
            font.letterSpacing: Theme.type.bodySmall.tracking
        }
    }
}
