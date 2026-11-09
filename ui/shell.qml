import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Wayland._WlrLayerShell
import "components"

// The root surface.
//
// One layer-shell surface, full width, transparent. The island is an item
// inside it that changes shape; the surface itself does not resize, so the
// compositor is never asked to reconfigure it mid-animation.
//
// Quickshell's `PanelWindow` has no `namespace` and no input region, so the
// surface is anchored `top`/`left`/`right` and the island is centred inside
// it by hand.
PanelWindow {
    id: root

    color: "transparent"

    exclusionMode: ExclusionMode.Ignore

    // The daemon-written material palette (matugen tie-in). Loaded once
    // at startup and on every change; absent file = built-in theme.
    property FileView paletteFile: FileView {
        path: Quickshell.env("XDG_STATE_HOME")
            ? Quickshell.env("XDG_STATE_HOME") + "/isled/palette.json"
            : Quickshell.env("HOME") + "/.local/state/isled/palette.json"
        watchChanges: true
        onLoaded: root.applyPalette()
        onFileChanged: root.applyPalette()
    }

    function applyPalette() {
        var text = paletteFile.text();
        if (!text)
            return;
        try {
            var parsed = JSON.parse(text);
            if (parsed && parsed.surface && parsed.primary && parsed.surface_ink)
                Theme.matugen = parsed;
        } catch (err) {
            // A half-written palette must never break the shell.
        }
    }

    // The island lives in the topmost (overlay) layer all the time. A normal
    // Top-layer surface can be covered by another overlay (notifications,
    // drag views, fullscreen apps), and then clicks meant for the shell land
    // on the expanded panel instead. Overlay + aboveWindows keeps the island
    // above everything so its input mask is authoritative. The mask below
    // still limits input to the island and snackbar, so topmost does not mean
    // the whole top strip eats clicks. Verified on Sway: swaymsg reports
    // this surface as layer "overlay".
    WlrLayershell.layer: WlrLayer.Overlay
    aboveWindows: true

    // Away/idle: collapse the island, dim it, and lock out panels until
    // the user is back. The daemon broadcasts `context.changed`; the UI
    // reads it into `isled.context`. Boosted contrast lifts the text.
    property bool away: isled.context && isled.context.state === "away"
    property bool reducedMotion: isled.settings
        && isled.settings.reduced_motion === true
    property bool highContrast: isled.settings
        && isled.settings.high_contrast === true

    // The morph duration the daemon resolved for the shell, straight
    // from settings — so `settings.set '{"key":"morph_duration_ms",…}'`
    // actually changes the feel without a rebuild.
    property int morphDuration: (isled.settings
        && isled.settings.morph_duration_ms !== undefined
        && isled.settings.morph_duration_ms > 0)
        ? isled.settings.morph_duration_ms
        : Theme.morphDuration

    // The keyboard belongs to the island only while a panel is open, or
    // while the mouse is over the pill.
    //
    // Niri hands the keyboard to a newly mapped on-demand layer surface, so
    // a surface that is focusable from the start takes the keyboard away
    // from the terminal at login and holds it until something else is
    // clicked. A notification that opens a panel must not do that either:
    // the keyboard has to stay where the reader left it.
    //
    // The hover case is the mouse path into a panel. Niri only grants
    // on-demand keyboard focus on a CLICK (`focus_layer_surface_if_on_demand`
    // in `input/mod.rs`), and a click that arrives while the surface is
    // non-focusable grants nothing — so the click that opens a panel
    // would leave keystrokes in the terminal and force a second click
    // inside. Hovering the pill flips the surface on-demand before the
    // press, so the click that opens the panel also moves the keyboard to
    // it. Hover alone never steals the keyboard: the grant still needs
    // the press. wlroots accepts the interactivity change on a mapped
    // surface even though the protocol says it should not.
    focusable: IslandSettings.keyboardFocus && (island.mode === "full" || island.hovered) && !root.away

    // Away: dim the island and collapse any open panel.
    onAwayChanged: {
        if (away) {
            island.collapse();
        }
    }

    // Push a11y/motion settings into the Theme singleton so colors and
    // stagger/crossfade bindings pick them up without a rebuild.
    // (Component.onCompleted itself is merged at the bottom of the file —
    // QML allows only one.)
    onReducedMotionChanged: Theme.reducedMotion = reducedMotion
    onHighContrastChanged: Theme.highContrast = highContrast
    onMorphDurationChanged: Theme.morphDuration = morphDuration
    onSchemeChanged: Theme.scheme = scheme === "light" ? "light" : "dark"
    property string scheme: isled && isled.settings && isled.settings.theme === "light"
        ? "light"
        : "dark"

    // The clickthrough mask. The surface is as wide as the screen and as
    // tall as the tallest panel, so without a mask every click in that
    // corner of the screen belongs to the island instead of to the window
    // underneath it. It also keeps the keyboard: niri gives the keyboard
    // back to whatever was clicked, so a click that lands on the window
    // below is what returns it.
    mask: Region {
        regions: [
            Region {
                item: island
                radius: island.radius
            },
            Region {
                item: snackbar
            }
        ]
    }

    property int view: 0
    readonly property int viewCount: PanelCatalog.names.length

    // The surface is always the full width and tall enough for the tallest
    // panel, so opening a panel never resizes it.
    implicitWidth: screen.width
    implicitHeight: Theme.panelMaxHeight + 80

    anchors {
        top: true
        left: true
        right: true
    }

    margins {
        top: 10
        left: 0
    }

    // The compositor blurs whatever is behind the island, which is what
    // makes the open panel glass rather than a flat sheet. Only the open
    // panel gets it: the idle island is a hole in the screen and there is
    // nothing behind it worth blurring.
    BackgroundEffect.blurRegion: island.displayState === "full" ? blur : null

    property Region blur: Region {
        item: island
        radius: island.radius
    }

    Island {
        id: island
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        view: root.view
        bridge: root.isled
        ambient: root.isled.activity
        maxHeight: Math.min(Theme.panelMaxHeight, screen.height - Theme.panelScreenMargin)

        onViewRequested: (index) => root.showView(index)
        onViewStepped: (delta) => root.stepView(delta)
    }

    // The bridge. A property rather than a child object, because a QtObject
    // declared as a child of a PanelWindow is never instantiated.
    property IsledBridge isled: IsledBridge {
        onToggleRequested: root.toggleIsland()
        onViewRequested: (index) => root.showView(index)
        onViewStepped: (delta) => root.stepView(delta)
        onFocusRequested: root.focusIsland()
        onActivityTriggered: (activity) => island.triggerWith(activity)
        onNoticeRaised: (message, action) => root.notify(message, action)
    }

    // The snackbar. It sits under the island, and it is the shell's way of
    // saying that something happened. It follows the island's bottom edge,
    // the way the mock's toast does, so it turns up under a short island
    // and under a tall one alike.
    Snackbar {
        id: snackbar
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: island.bottom
        anchors.topMargin: 20
    }

    function notify(message, action) {
        snackbar.open(message, action);
    }

    // The island asks for the keyboard when the panel opens. The click
    // that opened it arrived before the window was focusable, so the
    // request in the island's own click handler went nowhere; this one
    // comes after, and it is what makes Esc reach the island at all.
    onFocusableChanged: {
        if (root.focusable)
            island.forceActiveFocus();
    }

    // ---- routing ----

    function showView(index) {
        view = PanelCatalog.clamp(index);
        island.open();
    }

    function stepView(delta) {
        showView((view + delta + viewCount) % viewCount);
    }

    function toggleIsland() {
        if (island.mode === "full")
            island.collapse();
        else
            island.open();
    }

    function focusIsland() {
        island.open();
    }

    // Startup: push a11y/motion settings into the Theme singleton and ping
    // the daemon so the bridge warms the socket before the first panel.
    Component.onCompleted: {
        Theme.reducedMotion = reducedMotion;
        Theme.highContrast = highContrast;
        Theme.scheme = (isled.settings && isled.settings.theme === "light")
            ? "light"
            : "dark";
        root.isled.call("ping");
    }
}
