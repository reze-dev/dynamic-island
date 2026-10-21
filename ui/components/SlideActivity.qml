import QtQuick
import ".."

// Two rows of activity content, one sliding in as the other slides out.
//
// The island changes what it is saying more often than it changes shape —
// a track changes, a notification is replaced by another — and a cross-fade
// reads as "a new window opened". A slide reads as "this is the same thing
// saying something else", which is what is wanted.
Item {
    id: root

    property var activity: null
    property bool compact: false

    property bool frontIsNew: true

    ActivityRow {
        id: back
        anchors.fill: parent
        opacity: 0
    }

    ActivityRow {
        id: front
        anchors.fill: parent
    }

    NumberAnimation {
        id: slideOut
        property: "y"
        duration: Theme.slideDuration
        easing.type: Easing.OutQuart
    }

    NumberAnimation {
        id: slideIn
        property: "y"
        duration: Theme.slideDuration
        easing.type: Easing.OutQuart
    }

    function show(next) {
        if (next === null || next === undefined) {
            front.activity = null;
            back.activity = null;
            front.y = 0;
            back.y = 0;
            return;
        }

        var outgoing = frontIsNew ? front : back;
        var incoming = frontIsNew ? back : front;

        slideOut.stop();
        slideIn.stop();

        incoming.activity = next;
        incoming.z = 2;
        incoming.opacity = 1;
        incoming.y = root.height;

        outgoing.z = 1;
        outgoing.opacity = 1;
        outgoing.y = 0;

        slideOut.target = outgoing;
        slideOut.from = 0;
        slideOut.to = -root.height;
        slideOut.start();

        slideIn.target = incoming;
        slideIn.from = root.height;
        slideIn.to = 0;
        slideIn.start();

        frontIsNew = !frontIsNew;
    }

    onActivityChanged: show(activity)
    onCompactChanged: {
        front.compact = compact;
        back.compact = compact;
    }

    Component.onCompleted: {
        front.compact = compact;
        back.compact = compact;
        front.activity = activity;
    }
}
