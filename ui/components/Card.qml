import QtQuick
import ".."

// A card: the rounded, faintly filled surface that panel content sits on.
Rectangle {
    id: root

    property bool hovered: false
    property bool selected: false
    property color accent: Theme.accent

    color: hovered ? Theme.cardHover : Theme.card
    radius: Theme.radiusCard
    border.width: selected ? 1 : 0
    border.color: accent

    Behavior on color {
        ColorAnimation {
            duration: Theme.crossfadeDuration
        }
    }
}
