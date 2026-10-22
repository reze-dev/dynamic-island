import QtQuick
import ".."

// A snackbar: the small message that slides up from the bottom of the
// screen to say that something happened.
//
// Material's snackbar is a quiet bar in the container colour, with the
// action in the primary. It is not an alert: it says what just happened
// and offers the one thing you might want to do about it.
Rectangle {
    id: root

    property string text: ""
    /// The label of the action, or the empty string for a message with no
    /// action.
    property string action: ""
    /// Whether the snackbar is on the screen.
    property bool shown: false

    signal actionTriggered()

    readonly property bool hasAction: action !== ""

    implicitHeight: 48
    implicitWidth: row.width + 32
    radius: Theme.radiusExtraSmall
    color: Theme.surfaceContainerHighest
    border.width: 1
    border.color: Theme.outlineVariant

    opacity: shown ? 1 : 0
    visible: opacity > 0.01

    Behavior on opacity {
        NumberAnimation {
            duration: Theme.crossfadeDuration
            easing: Theme.decelerate
        }
    }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.text
            color: Theme.onSurface
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.bodyMedium.size
            font.letterSpacing: Theme.type.bodyMedium.tracking
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.hasAction
            text: root.action
            color: Theme.primary
            font.family: Theme.fontFamily
            font.pixelSize: Theme.type.labelLarge.size
            font.weight: Theme.type.labelLarge.weight
            font.letterSpacing: Theme.type.labelLarge.tracking

            MouseArea {
                anchors.fill: parent
                anchors.margins: -8
                cursorShape: Qt.PointingHandCursor
                onClicked: root.actionTriggered()
            }
        }
    }

    Timer {
        id: hide
        interval: 4000
        onTriggered: root.shown = false
    }

    onShownChanged: {
        if (shown)
            hide.restart();
    }

    /// Say something.
    function open(message, actionLabel) {
        text = message === undefined ? text : message;
        // A message with no action of its own does not keep the last one's.
        action = actionLabel === undefined ? "" : actionLabel;
        shown = true;
        // Saying the same thing twice should still show for the whole
        // time, so the timer is restarted rather than left to run out.
        hide.restart();
    }
}
