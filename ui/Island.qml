import QtQuick
import "components"

// The island.
//
// Four appearances, three sizes:
//
//   idle      — the smallest thing that still says something: a coloured
//               dot and the clock. Pure black, so it reads as a hole in the
//               top of the screen.
//   peek      — what a hover shows. The last thing that happened, and a
//               sliver of the machine's vitals.
//   triggered — what an event looks like. Grows, shows what woke it, and
//               folds back down on its own.
//   full      — a panel. The largest state, reached by a keybind or by
//               clicking the island.
//
// The size change is the morph. Everything else is a cross-fade, except the
// activity itself, which slides.
Rectangle {
    id: island

    // ---- the state machine ----

    /// idle | triggered | full
    property string mode: "idle"
    property bool hovered: false
    property int view: 0

    /// The daemon connection, for the live numbers in the peek view.
    property var bridge: null

    /// The tallest the panel may grow. The shell sets this from the height
    /// of the screen, so that a tall panel never runs off the top of it.
    property int maxHeight: Theme.panelMaxHeight

    /// The shell routes these. They are raised here rather than in the root
    /// because a `PanelWindow` is not an `Item` and cannot take the Keys
    /// attached property.
    signal viewRequested(int index)
    signal viewStepped(int delta)

    /// The persistent condition: music playing, a timer running, the battery
    /// low. It colours the dot and is what the peek shows.
    property var ambient: null
    /// The event that is currently being shown.
    property var trigger: null

    /// The colour the dot remembers. A trigger sets it; so does an ambient
    /// condition, which is the stronger signal.
    property string latchedTone: "neutral"

    readonly property string dotTone: trigger
        ? (trigger.tone ? trigger.tone : "neutral")
        : latchedTone

    readonly property string displayState: {
        if (mode === "full")
            return "full";
        if (mode === "triggered")
            return "triggered";
        return hovered ? "peek" : "idle";
    }

    /// The glyph of whatever is currently on the island, or the one of the
    /// panel that is open.
    readonly property string glyph: {
        if (displayState === "full")
            return PanelCatalog.glyph(view);
        if (trigger)
            return trigger.glyph ? trigger.glyph : "spark";
        if (ambient)
            return ambient.glyph ? ambient.glyph : "spark";
        return "spark";
    }

    // ---- geometry ----

    readonly property int targetWidth: {
        switch (displayState) {
        case "full":
            return Theme.panelWidth;
        case "triggered":
            return Theme.activeWidth;
        case "peek":
            return Theme.peekWidth;
        default:
            return Theme.idleWidth;
        }
    }

    readonly property int targetHeight: {
        switch (displayState) {
        case "full":
            return Math.min(PanelCatalog.heightFor(view), maxHeight);
        case "triggered":
            return Theme.activeHeight;
        case "peek":
            return Theme.peekHeight;
        default:
            return Theme.idleHeight;
        }
    }

    /// A pill is a circle; a panel is a rounded rectangle.
    readonly property int targetRadius: displayState === "full"
        ? Theme.radiusExtraLarge
        : Math.round(targetHeight / 2)

    width: targetWidth
    height: targetHeight
    radius: targetRadius
    color: displayState === "full" ? Theme.glass : Theme.islandSurface
    border.width: 1
    border.color: displayState === "full" ? Theme.hairline : "transparent"
    clip: true

    Behavior on width {
        NumberAnimation {
            duration: Theme.morphDuration
            easing: Theme.emphasized
        }
    }
    Behavior on height {
        NumberAnimation {
            duration: Theme.morphDuration
            easing: Theme.emphasized
        }
    }
    Behavior on radius {
        NumberAnimation {
            duration: Theme.morphDuration
            easing: Theme.emphasized
        }
    }
    Behavior on color {
        ColorAnimation {
            duration: Theme.crossfadeDuration
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: Theme.crossfadeDuration
        }
    }

    // ---- reacting ----

    /// Show an event: grow, show it, and fold back after its dwell time.
    function triggerWith(activity) {
        if (!activity)
            return;
        if (!IslandSettings.autoTrigger) {
            latchedTone = activity.tone ? activity.tone : latchedTone;
            return;
        }
        trigger = activity;
        latchedTone = activity.tone ? activity.tone : latchedTone;
        mode = "triggered";
        dwell.restart();
    }

    function clearTrigger() {
        trigger = null;
        if (mode === "triggered")
            mode = "idle";
    }

    function open() {
        mode = "full";
    }

    function collapse() {
        mode = "idle";
        trigger = null;
    }

    function togglePanel() {
        if (mode === "full")
            collapse();
        else
            open();
    }

    onTriggerChanged: {
        if (trigger)
            latchedTone = trigger.tone ? trigger.tone : latchedTone;
    }

    onAmbientChanged: {
        if (ambient)
            latchedTone = ambient.tone ? ambient.tone : latchedTone;
    }

    Timer {
        id: dwell
        interval: island.trigger && island.trigger.dwell_ms ? island.trigger.dwell_ms : IslandSettings.defaultDwell
        onTriggered: island.clearTrigger()
    }

    // ---- idle: a dot and the clock ----

    Item {
        id: idleContent
        anchors.fill: parent
        opacity: island.displayState === "idle" ? 1 : 0
        visible: opacity > 0.01

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
            }
        }

        Row {
            anchors.centerIn: parent
            spacing: Theme.spacingTight

            Rectangle {
                id: idleDot
                width: 8
                height: 8
                radius: 4
                color: Theme.toneColor(island.dotTone)
                anchors.verticalCenter: parent.verticalCenter

                Behavior on color {
                    ColorAnimation {
                        duration: Theme.crossfadeDuration
                    }
                }
            }

            Text {
                id: clock
                anchors.verticalCenter: parent.verticalCenter
                text: Format.clock()
                color: Theme.onSurfaceVariant
                font.family: Theme.monoFamily
                font.pixelSize: Theme.type.labelMedium.size
                font.weight: Theme.type.labelMedium.weight
                font.letterSpacing: Theme.overlineTracking
            }
        }

        Timer {
            interval: 1000
            running: true
            repeat: true
            onTriggered: clock.text = Format.clock()
        }
    }

    // ---- peek: what a hover shows ----

    Item {
        id: peekContent
        anchors.fill: parent
        opacity: island.displayState === "peek" ? 1 : 0
        visible: opacity > 0.01

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
            }
        }

        Row {
            id: peekRow
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacing
            anchors.right: vitals.left
            anchors.rightMargin: Theme.spacing
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.spacingTight

            Rectangle {
                id: peekDot
                width: 8
                height: 8
                radius: 4
                color: Theme.toneColor(island.dotTone)
                anchors.verticalCenter: parent.verticalCenter

                Behavior on color {
                    ColorAnimation {
                        duration: Theme.crossfadeDuration
                    }
                }
            }

            SlideActivity {
                id: peekActivity
                anchors.verticalCenter: parent.verticalCenter
                width: peekRow.width - peekDot.width - peekRow.spacing
                height: 24
                compact: true
                activity: island.ambient
                    ? island.ambient
                    : (island.trigger ? island.trigger : null)
            }
        }

        Column {
            id: vitals
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacing
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
            width: 96

            Row {
                spacing: Theme.spacingTight
                Text {
                    text: "cpu"
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                    width: 22
                }
                MiniBar {
                    width: 58
                    value: island.bridge && island.bridge.stats
                        ? (island.bridge.stats.cpu_percent || 0) / 100
                        : 0
                    tone: Theme.primary
                }
            }

            Row {
                spacing: Theme.spacingTight
                Text {
                    text: "mem"
                    color: Theme.onSurfaceVariant
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.labelSmall.size
                    font.weight: Theme.type.labelSmall.weight
                    font.letterSpacing: Theme.type.labelSmall.tracking
                    width: 22
                }
                MiniBar {
                    width: 58
                    value: island.bridge && island.bridge.stats
                        ? (island.bridge.stats.memory_used_bytes || 0)
                          / Math.max(1, island.bridge.stats.memory_total_bytes || 1)
                        : 0
                    tone: Theme.toneGood
                }
            }
        }
    }

    // ---- triggered: what an event looks like ----

    Item {
        id: triggeredContent
        anchors.fill: parent
        opacity: island.displayState === "triggered" ? 1 : 0
        visible: opacity > 0.01

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
            }
        }

        Row {
            id: triggerRow
            anchors.left: parent.left
            // The triggered state is a pill, so its corners are round, and
            // the badge has to clear the arc or it is clipped by it. The
            // margin is the radius of the pill.
            anchors.leftMargin: Math.round(Theme.activeHeight / 2)
            anchors.right: parent.right
            anchors.rightMargin: Math.round(Theme.activeHeight / 2)
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.spacingTight + 2

            SlideActivity {
                id: triggerActivity
                anchors.verticalCenter: parent.verticalCenter
                width: triggerRow.width - pulse.width - triggerRow.spacing * 2
                height: 40
                activity: island.trigger
            }

            // The bars that say something is happening.
            Row {
                id: pulse
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3
                height: 14

                Repeater {
                    model: 4

                    Item {
                        id: pulseBar
                        required property int index

                        width: 3
                        height: 14

                        Rectangle {
                            id: bar
                            width: parent.width
                            height: 6
                            y: (parent.height - height) / 2
                            radius: width / 2
                            color: Theme.primary

                            SequentialAnimation on height {
                                running: true
                                loops: Animation.Infinite
                                PauseAnimation {
                                    duration: pulseBar.index * 70
                                }
                                NumberAnimation {
                                    to: 13
                                    duration: 380
                                    easing.type: Easing.InOutSine
                                }
                                NumberAnimation {
                                    to: 6
                                    duration: 380
                                    easing.type: Easing.InOutSine
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- full: a panel ----

    Item {
        id: panelArea
        anchors.fill: parent
        opacity: island.displayState === "full" ? 1 : 0
        visible: opacity > 0.01

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.crossfadeDuration
            }
        }

        // The header: which panel this is, the way to other panels, and the
        // way out.
        Item {
            id: header
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56

            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: Theme.outlineVariant
                opacity: 0.6
            }

            Row {
                id: headerLeft
                anchors.left: parent.left
                anchors.leftMargin: Theme.spacing
                anchors.right: headerRight.left
                anchors.rightMargin: Theme.spacingTight
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.spacingTight + 1

                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: PanelCatalog.glyph(island.view)
                    width: 20
                    height: 20
                    color: Theme.primary
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, headerLeft.width - 28)
                    text: PanelCatalog.name(island.view)
                    color: Theme.onSurface
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.type.titleMedium.size
                    font.weight: Theme.type.titleMedium.weight
                    font.letterSpacing: Theme.type.titleMedium.tracking
                    elide: Text.ElideRight
                }
            }

            Row {
                id: headerRight
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacingTight
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Repeater {
                    model: PanelCatalog.names.length

                    Item {
                        required property int index

                        width: 26
                        height: 26

                        IconButton {
                            anchors.fill: parent
                            icon: PanelCatalog.glyph(parent.index)
                            size: 26
                            selected: island.view === parent.index
                            variant: "standard"
                            plated: true
                            tooltip: PanelCatalog.name(parent.index)
                                + " · " + PanelCatalog.shortcut(parent.index)
                            onClicked: island.view = parent.index
                        }
                    }
                }

                Rectangle {
                    width: 1
                    height: 16
                    color: Theme.outlineVariant
                    anchors.verticalCenter: parent.verticalCenter
                }

                IconButton {
                    icon: "close"
                    size: 26
                    tooltip: "Collapse · Esc"
                    onClicked: island.collapse()
                }
            }
        }

        PanelHost {
            id: panel
            anchors.top: header.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            view: island.view
            bridge: island.bridge
        }
    }

    // ---- the island's own surface ----

    MouseArea {
        id: surface
        anchors.fill: parent
        z: -1
        enabled: island.displayState !== "full"
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: island.hovered = true
        onExited: island.hovered = false
        onClicked: {
            island.forceActiveFocus();
            island.open();
        }
    }

    // ---- the keyboard ----

    Keys.onEscapePressed: (event) => {
        if (island.mode === "full")
            island.collapse();
        else
            island.clearTrigger();
        event.accepted = true;
    }

    Keys.onLeftPressed: (event) => {
        island.viewStepped(-1);
        event.accepted = true;
    }

    Keys.onRightPressed: (event) => {
        island.viewStepped(1);
        event.accepted = true;
    }

    Keys.onPressed: (event) => {
        // Digits pick a panel directly, the way the mock does.
        var text = event.text;
        if (text >= "1" && text <= "9") {
            const index = parseInt(text) - 1;
            island.viewRequested(index);
            event.accepted = true;
        }
    }

    // A faint glow under the island, so it reads as lit from above.
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: -10
        width: parent.width * 0.7
        height: 14
        radius: 7
        color: Theme.toneColor(island.dotTone)
        opacity: 0.10
        z: -2
    }
}
