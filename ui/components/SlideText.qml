import QtQuick
import ".."

// A text label that slides when its content changes.
//
// ActivityRow and the peek view swap titles and subtitles more often than
// they change shape — a track changes, a timer ticks, a notification is
// replaced by another. A cross-fade reads as "a new window opened"; a
// vertical slide reads as "the same thing saying something else", which
// is what a content swap is. The two-row mechanics mirror SlideActivity:
// the outgoing line slides up and out while the incoming one slides up
// from below (or fades straight in under reduced motion).
Item {
    id: root

    property string text: ""
    property color color: Theme.onSurface
    property int pixelSize: Theme.type.bodyMedium.size
    property int weight: Theme.type.bodyMedium.weight
    property real tracking: Theme.type.bodyMedium.tracking

    implicitHeight: pixelSize * 1.4
    clip: true

    // The row currently shown, and the one that left. Roles swap on every
    // change, so rapid successive changes still read as motion.
    property bool frontIsNew: true

    Text {
        id: back
        width: parent.width
        text: root.text
        color: root.color
        font.family: Theme.fontFamily
        font.pixelSize: root.pixelSize
        font.weight: root.weight
        font.letterSpacing: root.tracking
        elide: Text.ElideRight
        opacity: 0
    }

    Text {
        id: front
        width: parent.width
        text: root.text
        color: root.color
        font.family: Theme.fontFamily
        font.pixelSize: root.pixelSize
        font.weight: root.weight
        font.letterSpacing: root.tracking
        elide: Text.ElideRight
    }

    NumberAnimation {
        id: slideOut
        property: "y"
        duration: Theme.reducedMotion ? 0 : Theme.slideDuration
        easing: Theme.accelerate
    }

    NumberAnimation {
        id: slideIn
        property: "y"
        duration: Theme.reducedMotion ? 0 : Theme.slideDuration
        easing: Theme.decelerate
    }

    Timer {
        id: settle
        interval: Theme.reducedMotion ? 0 : Theme.slideDuration
        repeat: false
        onTriggered: {
            // Both rows settled on the final text; drop the stale one.
            front.text = root.text;
            back.text = root.text;
            front.y = 0;
            back.y = 0;
            front.opacity = 1;
            back.opacity = 0;
        }
    }

    onTextChanged: {
        var outgoing = frontIsNew ? front : back;
        var incoming = frontIsNew ? back : front;
        // Break the bindings so a content swap shows the OLD text on the
        // outgoing row and the NEW text only on the incoming one.
        outgoing.text = outgoing.text;
        incoming.text = root.text;
        slideOut.stop();
        slideIn.stop();
        settle.stop();
        incoming.z = 2;
        incoming.opacity = 1;
        incoming.y = implicitHeight;
        outgoing.opacity = 1;
        slideOut.target = outgoing;
        slideOut.to = -implicitHeight;
        slideIn.target = incoming;
        slideIn.to = 0;
        frontIsNew = !frontIsNew;
        settle.restart();
        slideOut.start();
        slideIn.start();
    }
}
