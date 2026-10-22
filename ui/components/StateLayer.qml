import QtQuick
import ".."

// The state layer: the translucent wash Material paints over an
// interactive surface to say that the pointer is on it, that it has
// focus, or that it is being pressed.
//
// The four states are ranked: a dragged thing is more committed than a
// pressed one, which is more committed than a focused one, which is more
// committed than a hovered one.
Item {
    id: root

    /// The colour of the wash. Usually the on-colour of the surface it
    /// sits on.
    property color color: Theme.onSurface
    property bool hovered: false
    property bool focused: false
    property bool pressed: false
    property bool dragged: false
    /// The corner radius of the wash, so that it follows the shape of the
    /// surface underneath it.
    property real radius: 0

    readonly property real strength: {
        if (root.dragged)
            return Theme.stateDragged;
        if (root.pressed)
            return Theme.statePressed;
        if (root.focused)
            return Theme.stateFocus;
        if (root.hovered)
            return Theme.stateHover;
        return 0;
    }

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: Theme.stateLayer(root.color, root.strength)
        visible: root.strength > 0

        Behavior on color {
            ColorAnimation {
                duration: Theme.stateLayerDuration
            }
        }
    }
}
