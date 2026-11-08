import QtQuick
import ".."

// An icon button: Material's round button that carries a glyph and nothing
// else.
//
// Four variants, which differ in what they are painted with. Standard is
// transparent. Filled is the primary colour. Tonal is the secondary
// container. Outlined draws a border.
//
// A selected button becomes filled, which is how the island's panel
// navigation shows where it is.
Item {
    id: root

    property string icon: "spark"
    property color tone: Theme.onSurfaceVariant
    property bool selected: false
    property bool active: false
    /// Whether the glyph is drawn in the filled variant of the icon, when
    /// it has one.
    property bool filled: false
    /// Whether the button carries a plate even when it is not selected. The
    /// island's panel navigation does: a row of floating glyphs is hard to
    /// read, a row of quiet circles is not.
    property bool plated: false
    /// standard | filled | tonal | outlined
    property string variant: "standard"
    property int size: 40
    property string tooltip: ""

    signal clicked()

    implicitWidth: size
    implicitHeight: size
    opacity: enabled ? 1 : 0.38

    // Accessibility: screen readers announce the tooltip/icon as the
    // button name, keyboard users can tab to it and press Enter/Space.
    Accessible.role: Accessible.Button
    Accessible.name: tooltip !== "" ? tooltip : icon
    activeFocusOnTab: true
    Keys.onReturnPressed: clicked()
    Keys.onSpacePressed: clicked()

    readonly property bool filledVariant: variant === "filled" || (selected && variant === "standard")
    readonly property bool tonalVariant: variant === "tonal"
    readonly property bool outlinedVariant: variant === "outlined"

    readonly property color plateColor: {
        if (root.filledVariant)
            return Theme.primary;
        if (root.tonalVariant)
            return Theme.secondaryContainer;
        if (root.active)
            return Theme.primaryContainerDim;
        if (root.plated)
            return Theme.surfaceFillHighest;
        return "transparent";
    }

    readonly property color glyphColor: {
        if (root.filledVariant)
            return Theme.onPrimary;
        if (root.tonalVariant)
            return Theme.onSecondaryContainer;
        if (root.selected)
            return Theme.primary;
        if (root.active)
            return Theme.primary;
        if (root.plated)
            return Theme.onSurface;
        return root.tone;
    }

    Rectangle {
        id: plate
        anchors.fill: parent
        radius: width / 2
        color: root.plateColor
        border.width: root.outlinedVariant || root.activeFocus ? 1 : 0
        border.color: root.activeFocus ? Theme.primary : Theme.outline

        Behavior on color {
            ColorAnimation {
                duration: Theme.stateLayerDuration
            }
        }
    }

    // The wash that says the pointer is here.
    StateLayer {
        anchors.fill: parent
        radius: width / 2
        color: root.filledVariant ? Theme.onPrimary : Theme.onSurface
        hovered: hover.containsMouse
        pressed: hover.pressed
    }

    Ripple {
        id: wave
        anchors.fill: parent
        color: root.filledVariant ? Theme.onPrimary : Theme.onSurface
    }

    Icon {
        anchors.centerIn: parent
        name: root.icon
        filled: root.filled
        width: Math.round(root.size * 0.6)
        height: width
        color: root.glyphColor

        Behavior on color {
            ColorAnimation {
                duration: Theme.stateLayerDuration
            }
        }
    }

    // The tooltip. Plain QML, because QtQuick.Controls is not linked into
    // the Quickshell engine.
    Rectangle {
        visible: hover.containsMouse && root.tooltip !== ""
        anchors.top: parent.bottom
        anchors.topMargin: 6
        anchors.horizontalCenter: parent.horizontalCenter
        width: tip.implicitWidth + 20
        height: 28
        radius: Theme.radiusSmall
        color: Theme.surfaceContainerHighest
        border.width: 1
        border.color: Theme.outlineVariant
        z: 40

        Text {
            id: tip
            anchors.centerIn: parent
            text: root.tooltip
            color: Theme.onSurfaceVariant
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelSmall.size
            font.weight: Theme.type.labelSmall.weight
            font.letterSpacing: Theme.type.labelSmall.tracking
        }
    }

    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        enabled: root.enabled
        onPressed: (mouse) => wave.start(mouse.x, mouse.y)
        onReleased: wave.stop()
        onExited: wave.stop()
        onClicked: root.clicked()
    }
}
