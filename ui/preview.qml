import QtQuick
import Quickshell
import QtQuick.Window

// A preview rig, and not part of the shell.
//
// It renders the island in each of its appearances and writes them out as
// PNGs, so the design can be looked at without a compositor, without a
// daemon, and without a screenshot. That is worth having: the morph and the
// cross-fades are the parts of this project that cannot be verified by a
// test, and a picture is the next best thing to looking at it.
//
//   just preview      renders into /tmp/isle-preview
//
// It lives in the main module rather than a subdirectory of its own because
// a file loaded as a root config does not get the parent directory's
// imports: a `import ".."` from a subdirectory resolves to nothing, and the
// singletons and the Island come from the module the file is part of.
ShellRoot {
    id: root

    property int index: 0
    readonly property string output: Quickshell.env("ISLED_PREVIEW_DIR")
        ? Quickshell.env("ISLED_PREVIEW_DIR")
        : "/tmp/isle-preview"

    // When ISLED_SOCKET points at a running daemon, the rig connects to it
    // and the panels are photographed with real data in them. An empty
    // panel is a forgiving thing: it hides every layout fault there is.
    readonly property bool live: Quickshell.env("ISLED_SOCKET") !== ""
    property IsledBridge bridge: IsledBridge {
        autoConnect: root.live
    }

    /// The appearances worth photographing, in the order they happen.
    property var scenarios: [
        {
            "name": "idle",
            "mode": "idle",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 0
        },
        {
            "name": "idle-ambient",
            "mode": "idle",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null,
            "view": 0
        },
        {
            "name": "peek",
            "mode": "idle",
            "hovered": true,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null,
            "view": 0
        },
        {
            "name": "triggered",
            "mode": "triggered",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether",
                "glyph": "music",
                "sticky": true
            },
            "trigger": {
                "kind": "notification",
                "tone": "warn",
                "title": "Build completed",
                "subtitle": "northstar - dev shell evaluated",
                "glyph": "bell",
                "dwell_ms": 5000,
                "sticky": false
            },
            "view": 0
        },
        {
            "name": "full-context",
            "mode": "full",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null,
            "view": 0
        },
        {
            "name": "full-media",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 1
        },
        {
            "name": "full-notifications",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 2
        },
        {
            "name": "full-clipboard",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 3
        },
        {
            "name": "full-calendar",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 4
        },
        {
            "name": "full-todos",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 5
        },
        {
            "name": "full-notes",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 6
        },
        {
            "name": "full-network",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 7
        },
        {
            "name": "full-theme",
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null,
            "view": 8
        }
    ]

    Window {
        id: win
        width: 560
        height: 520
        visible: true
        color: "#101014"

        Island {
            id: island
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: parent.top
            anchors.topMargin: 24
            mode: root.scenarios[root.index].mode
            hovered: root.scenarios[root.index].hovered
            ambient: root.scenarios[root.index].ambient
            trigger: root.scenarios[root.index].trigger
            view: root.scenarios[root.index].view
            bridge: root.bridge
        }
    }

    // The island takes 380ms to change shape and 200ms to cross-fade, so
    // each scenario has to settle before it is photographed.
    // The daemon has to be asked for everything before the first picture,
    // or the panels are photographed empty.
    Timer {
        interval: 1100
        running: root.warm
        repeat: true
        onTriggered: root.shoot()
    }

    property bool warm: false
    Timer {
        interval: 2500
        running: true
        repeat: false
        onTriggered: root.warm = true
    }

    function shoot() {
        if (root.index >= root.scenarios.length) {
            Qt.callLater(Qt.quit);
            return;
        }
        var scenario = root.scenarios[root.index];
        island.grabToImage(function (result) {
            result.saveToFile(root.output + "/" + scenario.name + ".png");
            root.index = root.index + 1;
        });
    }
}
