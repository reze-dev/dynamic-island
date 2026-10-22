import QtQuick
import ".."

// A button: Material's text button, with an optional glyph.
//
// Five variants. Filled is the primary colour and carries the only filled
// button in a panel. Tonal is the secondary container. Elevated is a raised
// surface. Outlined draws a border. Text is nothing but the label, and is
// what a panel uses for its quietest actions.
//
// The label sits inside a row with the glyph, and the whole thing is one
// tap target.
Item {
    id: root

    /// The variant: filled | tonal | elevated | outlined | text
    property string variant: "filled"
    property string text: ""
    /// The glyph, or the empty string for a button with no icon.
    property string icon: ""

    signal clicked()

    readonly property bool hasIcon: icon !== ""

    readonly property int controlHeight: 40
    readonly property int horizontalPadding: hasIcon ? 16 : 24

    implicitHeight: controlHeight
    implicitWidth: row.width + horizontalPadding * 2

    readonly property color labelColor: {
        switch (variant) {
        case "tonal":
            return Theme.onSecondaryContainer;
        case "elevated":
            return Theme.primary;
        case "outlined":
            return Theme.primary;
        case "text":
            return Theme.primary;
        default:
            return Theme.onPrimary;
        }
    }

    readonly property color plateColor: {
        switch (variant) {
        case "tonal":
            return Theme.secondaryContainer;
        case "elevated":
            return Theme.surfaceContainerLow;
        case "outlined":
            return "transparent";
        case "text":
            return "transparent";
        default:
            return Theme.primary;
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusFull
        color: root.plateColor
        border.width: root.variant === "outlined" ? 1 : 0
        border.color: Theme.outline
        opacity: root.enabled ? 1 : 0.38

        Behavior on color {
            ColorAnimation {
                duration: Theme.stateLayerDuration
            }
        }
    }

    StateLayer {
        anchors.fill: parent
        radius: width / 2
        color: root.labelColor
        hovered: hover.containsMouse
        pressed: hover.pressed
    }

    Ripple {
        id: wave
        anchors.fill: parent
        color: root.labelColor
    }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8

        Icon {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.hasIcon
            name: root.icon
            width: 18
            height: width
            color: root.labelColor
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.text !== ""
            text: root.text
            color: root.labelColor
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge.size
            font.weight: Theme.type.labelLarge.weight
            font.letterSpacing: Theme.type.labelLarge.tracking
        }
    }

    MouseArea {
        id: hover
        anchors.fill: parent
        enabled: root.enabled
        hoverEnabled: true
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onPressed: (mouse) => wave.start(mouse.x, mouse.y)
        onReleased: wave.stop()
        onExited: wave.stop()
        onClicked: root.clicked()
    }
}
