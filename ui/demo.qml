import QtQuick
import Quickshell
import QtQuick.Window

// A frame-grabbing rig for the morph.
//
// The island's animation is the one thing no test can check and no
// screenshot can show. This drives the island through its states and
// photographs it while the `Behavior` animations are running, so the motion
// can be looked at afterwards — as a GIF, or frame by frame.
//
//   just demo-anim
//
// It lives in the main module rather than a subdirectory of its own because
// a file loaded as a root config does not get the parent directory's
// imports: the singletons and the Island come from the module the file is
// part of.
ShellRoot {
    id: root

    readonly property string output: Quickshell.env("ISLED_DEMO_FRAMES")
        ? Quickshell.env("ISLED_DEMO_FRAMES")
        : "/tmp/isle-anim/frames"
    property int beatIndex: 0
    property int frameIndex: 0
    property int frameNumber: 0

    /// The beat being shown. Clamped, because the bindings below are
    /// evaluated after the last one has been left behind.
    readonly property var beat: root.beats[Math.min(root.beatIndex, root.beats.length - 1)]

    /// The beats, in order. Each one is a state and how long to watch it.
    property var beats: [
        {
            "name": "idle",
            "settle": 700,
            "mode": "idle",
            "hovered": false,
            "ambient": null,
            "trigger": null
        },
        {
            "name": "music",
            "settle": 900,
            "mode": "triggered",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "sticky": true
            },
            "trigger": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "dwell_ms": 600000,
                "sticky": true
            }
        },
        {
            "name": "idle-with-ambient",
            "settle": 900,
            "mode": "idle",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null
        },
        {
            "name": "peek",
            "settle": 900,
            "mode": "idle",
            "hovered": true,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null
        },
        {
            "name": "notification",
            "settle": 900,
            "mode": "triggered",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "sticky": true
            },
            "trigger": {
                "kind": "notification",
                "tone": "warn",
                "title": "Build completed",
                "subtitle": "northstar · dev shell evaluated successfully",
                "glyph": "bell",
                "dwell_ms": 600000,
                "sticky": false
            }
        },
        {
            "name": "idle-after-notification",
            "settle": 900,
            "mode": "idle",
            "hovered": false,
            "ambient": {
                "kind": "media",
                "tone": "info",
                "title": "Glasshouse",
                "subtitle": "Aether · Late Night Systems",
                "glyph": "music",
                "sticky": true
            },
            "trigger": null
        },
        {
            "name": "panel",
            "settle": 1200,
            "mode": "full",
            "hovered": false,
            "ambient": null,
            "trigger": null
        }
    ]

    Window {
        id: win
        width: 560
        height: 520
        visible: true
        color: "#101014"

        // The island lives in a frame of a fixed size, and it is the frame
        // that is photographed. Grabbing the island itself would crop every
        // frame to the island, and the picture would jump about as it grew.
        Item {
            id: stage
            anchors.fill: parent

            Island {
                id: island
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 24
                view: 0
                mode: root.beat.mode
                hovered: root.beat.hovered
                ambient: root.beat.ambient
                trigger: root.beat.trigger
            }
        }
    }

    // One frame every forty milliseconds: twenty-five frames a second, which
    // is enough for the easing to read.
    Timer {
        interval: 40
        running: true
        repeat: true
        onTriggered: root.capture()
    }

    Timer {
        interval: 100
        running: true
        repeat: true
        onTriggered: root.tick()
    }

    property int elapsed: 0

    function tick() {
        if (root.beatIndex >= root.beats.length)
            return;
        root.elapsed = root.elapsed + 100;
        if (root.elapsed >= root.beat.settle) {
            root.elapsed = 0;
            root.beatIndex = root.beatIndex + 1;
            if (root.beatIndex >= root.beats.length) {
                Qt.callLater(Qt.quit);
                return;
            }
        }
    }

    function capture() {
        if (root.beatIndex >= root.beats.length)
            return;
        var name = ("000" + root.frameNumber).slice(-4);
        var target = root.output + "/" + name + "-" + root.beat.name + ".png";
        stage.grabToImage(function (result) {
            result.saveToFile(target);
            root.frameNumber = root.frameNumber + 1;
        });
    }
}
