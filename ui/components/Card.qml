import QtQuick
import ".."

// A card: the surface that panel content sits on.
//
// Material has three of them. Elevated is a raised surface, for the one
// thing in a panel that matters most. Tonal is a surface in the secondary
// container colour. Outlined draws a border and fills with nothing.
Rectangle {
    id: root

    /// elevated | tonal | outlined
    property string variant: "tonal"
    property bool hovered: false
    property bool selected: false
    property color accent: Theme.primary

    implicitHeight: 56
    radius: Theme.radiusMedium
    border.width: variant === "outlined" ? 1 : (selected ? 1 : 0)
    border.color: selected ? accent : Theme.outlineVariant

    color: {
        switch (variant) {
        case "elevated":
            return Theme.surfaceContainerLow;
        case "outlined":
            return "transparent";
        default:
            return Theme.surfaceFill;
        }
    }

    Behavior on color {
        ColorAnimation {
            duration: Theme.stateLayerDuration
        }
    }
}
