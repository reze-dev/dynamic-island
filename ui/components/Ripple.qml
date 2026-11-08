import QtQuick
import ".."

// The ripple: the wave of colour that spreads from the point of contact
// when something is pressed.
//
// Material puts two things on an interactive surface. The state layer is a
// translucent wash that says "the pointer is here". The ripple is the wave
// that says "you pressed this", and it starts where the pointer landed
// rather than in the middle, which is the part that makes it feel like the
// surface reacted rather than merely having been pressed.
Item {
    id: root

    /// The colour of the wave. Usually the on-colour of the surface it
    /// sits on.
    property color color: Theme.primary
    /// How strong the wave is. Material uses twelve per cent of the
    /// on-colour.
    property real strength: 0.12

    /// Where the wave starts. Defaults to the middle of the item.
    property real originX: width / 2
    property real originY: height / 2

    /// How far the wave has to spread to cover the whole surface.
    readonly property real bound: {
        var dx = Math.max(originX, width - originX);
        var dy = Math.max(originY, height - originY);
        return Math.sqrt(dx * dx + dy * dy);
    }

    Rectangle {
        id: wave
        color: root.color
        opacity: 0
        radius: width / 2
        width: root.bound * 2
        height: root.bound * 2
        x: root.originX - width / 2
        y: root.originY - height / 2
        transformOrigin: Item.Center
        scale: 0
        visible: scale > 0.001
    }

    NumberAnimation {
        id: expand
        target: wave
        property: "scale"
        from: 0
        to: 1
        duration: Theme.rippleDuration
        easing: Theme.decelerate
    }

    NumberAnimation {
        id: fade
        target: wave
        property: "opacity"
        to: 0
        duration: Theme.rippleFadeDuration
        easing: Theme.accelerate
    }

    /// Send the wave out from a point.
    function start(px, py) {
        originX = px === undefined ? width / 2 : px;
        originY = py === undefined ? height / 2 : py;
        expand.stop();
        fade.stop();
        wave.scale = 0;
        wave.opacity = root.strength;
        expand.start();
    }

    /// Let the wave fade where it stands.
    function stop() {
        expand.stop();
        fade.from = wave.opacity;
        fade.to = 0;
        fade.start();
    }
}
