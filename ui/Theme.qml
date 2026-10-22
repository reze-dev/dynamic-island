pragma Singleton

import QtQuick

// The design tokens.
//
// Everything visual in the shell comes from here, so that the island, the
// panels and the components all move and shade together. The system is
// Material 3: colour roles, a shape scale, a type scale, motion curves and
// state layers, all in the dark scheme.
//
// The palette is anchored on a single light-blue accent over a near-black
// ground. That is the whole idea of the design: one hue, carried by the
// primary role, with everything else built from neutral surfaces that step
// up in luminance as they come forward.
QtObject {
    id: theme

    // ---- colour roles: primary ----

    // Material names the foreground of a role "on" that role: onPrimary is
    // the colour of text drawn on primary. QML reserves the `on` prefix
    // for signal handlers, so a property called `onPrimary` would be read
    // as a handler for `primaryChanged` and would never be given a value.
    //
    // The colours therefore live under their own names and the Material
    // names are aliases onto them, which keeps the vocabulary intact and
    // the values right.

    // ---- colour roles: primary ----

    /// The accent. A light blue, because Material's dark scheme wants a
    /// primary that is light enough to sit on a dark surface as text.
    readonly property color primary: Qt.rgba(0.824, 0.890, 1.000)
    /// Text and icons drawn on top of the primary.
    readonly property color primaryInk: Qt.rgba(0.000, 0.196, 0.357)
    readonly property alias onPrimary: theme.primaryInk
    /// A quieter accent, for surfaces that want the accent's hue without
    /// the accent's weight.
    readonly property color primaryContainer: Qt.rgba(0.000, 0.282, 0.506)
    readonly property color primaryContainerInk: Qt.rgba(0.827, 0.890, 1.000)
    readonly property alias onPrimaryContainer: theme.primaryContainerInk

    // ---- colour roles: secondary ----

    readonly property color secondary: Qt.rgba(0.737, 0.780, 0.863)
    readonly property color secondaryInk: Qt.rgba(0.149, 0.192, 0.255)
    readonly property alias onSecondary: theme.secondaryInk
    readonly property color secondaryContainer: Qt.rgba(0.235, 0.282, 0.345)
    readonly property color secondaryContainerInk: Qt.rgba(0.847, 0.890, 0.976)
    readonly property alias onSecondaryContainer: theme.secondaryContainerInk

    // ---- colour roles: tertiary ----

    readonly property color tertiary: Qt.rgba(0.843, 0.765, 0.910)
    readonly property color tertiaryInk: Qt.rgba(0.224, 0.173, 0.278)
    readonly property alias onTertiary: theme.tertiaryInk
    readonly property color tertiaryContainer: Qt.rgba(0.322, 0.251, 0.400)
    readonly property color tertiaryContainerInk: Qt.rgba(0.945, 0.851, 1.000)
    readonly property alias onTertiaryContainer: theme.tertiaryContainerInk

    // ---- colour roles: error ----

    readonly property color error: Qt.rgba(1.000, 0.706, 0.671)
    readonly property color errorInk: Qt.rgba(0.412, 0.000, 0.020)
    readonly property alias onError: theme.errorInk
    readonly property color errorContainer: Qt.rgba(0.576, 0.000, 0.039)
    readonly property color errorContainerInk: Qt.rgba(1.000, 0.855, 0.835)
    readonly property alias onErrorContainer: theme.errorContainerInk

    // ---- colour roles: surfaces ----

    /// The ground the island floats over. The layer shell itself is
    /// transparent, so this is only ever a reference.
    readonly property color background: Qt.rgba(0, 0, 0)
    readonly property color backgroundInk: Qt.rgba(0.890, 0.890, 0.890)
    readonly property alias onBackground: theme.backgroundInk
    readonly property color surface: Qt.rgba(0.067, 0.075, 0.082)
    readonly property color surfaceInk: Qt.rgba(0.890, 0.890, 0.890)
    readonly property alias onSurface: theme.surfaceInk
    /// The quieter foreground, for the supporting lines and the labels.
    readonly property color surfaceVariant: Qt.rgba(0.604, 0.627, 0.647)
    readonly property alias onSurfaceVariant: theme.surfaceVariant
    readonly property color surfaceDim: Qt.rgba(0, 0, 0)
    readonly property color surfaceBright: Qt.rgba(0.204, 0.224, 0.243)
    readonly property color surfaceContainerLowest: Qt.rgba(0, 0, 0)
    readonly property color surfaceContainerLow: Qt.rgba(0.043, 0.047, 0.055)
    readonly property color surfaceContainer: Qt.rgba(0.067, 0.075, 0.082)
    readonly property color surfaceContainerHigh: Qt.rgba(0.106, 0.114, 0.125)
    readonly property color surfaceContainerHighest: Qt.rgba(0.141, 0.153, 0.165)

    /// The island's own skin. Nearly, but not entirely, opaque: the
    /// wallpaper should just about show through the expanded panel.
    /// The glass of the open panel: a dark tint over the compositor's
    /// blur. The mock uses .88, which is right when the backdrop is its
    /// own black page and the blur is only there to keep the text behind
    /// from being noisy. This one floats over whatever is on the screen,
    /// so it lets a little more through and the blur reads as glass.
    readonly property color glass: Qt.rgba(0.078, 0.078, 0.078, 0.82)
    /// The idle pill is solid. It is meant to read as a hole in the screen.
    readonly property color islandSurface: Qt.rgba(0, 0, 0, 0.96)

    // ---- colour roles: outlines and fills ----

    readonly property color outline: Qt.rgba(0.447, 0.471, 0.502)
    readonly property color outlineVariant: Qt.rgba(0.251, 0.278, 0.302)

    /// The hairline that edges the expanded panel.
    readonly property color hairline: Qt.rgba(1, 1, 1, 0.102)
    /// The fill of a card, a tile, a row.
    readonly property color surfaceFill: Qt.rgba(1, 1, 1, 0.051)
    /// The fill under the pointer.
    readonly property color surfaceFillHover: Qt.rgba(1, 1, 1, 0.071)
    /// The fill of a recessed field: an input, a well.
    readonly property color surfaceFillLow: Qt.rgba(1, 1, 1, 0.031)
    /// The strongest fill in the set, for tracks and dividers.
    readonly property color surfaceFillHighest: Qt.rgba(1, 1, 1, 0.102)
    /// The scrim that goes over a picture when something is said about it.
    readonly property color scrim: Qt.rgba(0, 0, 0, 0.45)

    // ---- colour roles: the accent, at low alpha ----

    readonly property color primaryDim: Qt.rgba(0.824, 0.890, 1.000, 0.10)
    readonly property color primaryContainerDim: Qt.rgba(0.824, 0.890, 1.000, 0.16)

    // ---- tones ----

    /// The semantic tones an activity can carry. These are not Material
    /// roles, because the daemon speaks in tones rather than in roles, but
    /// they are built out of the same palette.
    readonly property color toneNeutral: Qt.rgba(0.890, 0.890, 0.890, 0.45)
    readonly property color toneInfo: Qt.rgba(0.824, 0.890, 1.000)
    readonly property color toneGood: Qt.rgba(0.494, 0.886, 0.659)
    readonly property color toneWarn: Qt.rgba(0.961, 0.761, 0.420)
    readonly property color toneCritical: Qt.rgba(1.000, 0.706, 0.671)

    /// The colour a tone names. The island's dot is drawn in this.
    function toneColor(tone) {
        switch (tone) {
        case "info":
            return theme.toneInfo;
        case "good":
            return theme.toneGood;
        case "warn":
            return theme.toneWarn;
        case "critical":
            return theme.toneCritical;
        default:
            return theme.toneNeutral;
        }
    }

    // ---- shape scale ----

    /// Material's shape scale. Corners get rounder as the surface gets
    /// bigger, which is why the expanded panel is a 28 and a chip is an 8.
    readonly property var shape: ({
        "none": 0,
        "extraSmall": 4,
        "small": 8,
        "medium": 12,
        "large": 16,
        "extraLarge": 28,
        "full": 999
    })

    readonly property int radiusNone: shape.none
    readonly property int radiusExtraSmall: shape.extraSmall
    readonly property int radiusSmall: shape.small
    readonly property int radiusMedium: shape.medium
    readonly property int radiusLarge: shape.large
    readonly property int radiusExtraLarge: shape.extraLarge
    readonly property int radiusFull: shape.full

    // ---- type scale ----

    /// Material's type scale. `size` is the point size, `line` the line
    /// height, `weight` the font weight and `tracking` the letter spacing
    /// in pixels.
    readonly property var type: ({
        "displaySmall": { size: 36, line: 44, weight: 400, tracking: 0 },
        "headlineSmall": { size: 24, line: 32, weight: 400, tracking: 0 },
        "titleLarge": { size: 22, line: 28, weight: 400, tracking: 0 },
        "titleMedium": { size: 16, line: 24, weight: 500, tracking: 0.15 },
        "titleSmall": { size: 14, line: 20, weight: 500, tracking: 0.1 },
        "bodyLarge": { size: 16, line: 24, weight: 400, tracking: 0.5 },
        "bodyMedium": { size: 14, line: 20, weight: 400, tracking: 0.25 },
        "bodySmall": { size: 12, line: 16, weight: 400, tracking: 0.4 },
        "labelLarge": { size: 14, line: 20, weight: 500, tracking: 0.1 },
        "labelMedium": { size: 12, line: 16, weight: 500, tracking: 0.5 },
        "labelSmall": { size: 11, line: 16, weight: 500, tracking: 0.5 }
    })

    /// The overline: the wide-tracked label the idle island wears.
    readonly property real overlineTracking: 1.6

    // ---- spacing ----

    readonly property int spacingTight: 8
    readonly property int spacing: 16
    readonly property int spacingLoose: 24

    // ---- the island's four sizes ----

    /// The idle pill. Small enough to be a hole in the top of the screen.
    readonly property int idleWidth: 140
    readonly property int idleHeight: 34
    /// What an event looks like: the pill widens to hold a line of text.
    readonly property int activeWidth: 300
    readonly property int activeHeight: 42
    /// A peek on hover.
    readonly property int peekWidth: 300
    readonly property int peekHeight: 42
    /// The panel. As wide as it needs to be and no wider. Ten panels of
    /// navigation in the header is most of that floor, and the panel's own
    /// name has to fit beside them — "notifications" is the longest, and
    /// at 480 it came out as "notificati…".
    readonly property int panelWidth: 520
    /// The tallest a panel is allowed to grow, before the screen says no.
    readonly property int panelMaxHeight: 520
    /// The window the panel is allowed to leave for the rest of the screen.
    readonly property int panelScreenMargin: 110

    // ---- motion ----

    // Material's curves are cubic beziers: the emphasized one is
    // cubic-bezier(0.2, 0, 0, 1), and the standard and decelerate ones
    // are close to that. QML will not take a JavaScript function for an
    // easing curve, so these are the built-in curves that come closest:
    // OutCubic passes through 0.60, 0.88 and 0.98, against Material's
    // 0.61, 0.88 and 0.98.
    readonly property int emphasized: Easing.OutCubic
    /// The curve for anything that enters.
    readonly property int decelerate: Easing.OutQuart
    /// The curve for anything that leaves.
    readonly property int accelerate: Easing.InCubic

    /// How long the island takes to change shape.
    readonly property int morphDuration: 500
    /// How long content takes to fade.
    readonly property int crossfadeDuration: 200
    /// How long content takes to slide. Slightly longer than the morph, so
    /// the text arrives as the shape settles.
    readonly property int slideDuration: 400
    /// How long a state layer takes to appear and to go away again.
    readonly property int stateLayerDuration: 100
    /// How long a ripple takes to spread, and to fade.
    readonly property int rippleDuration: 450
    readonly property int rippleFadeDuration: 250

    // ---- state layers ----

    /// Material paints a translucent wash over an interactive surface when
    /// the pointer is on it, when it has focus, and when it is held down.
    /// These are the opacities of those washes.
    readonly property real stateHover: 0.08
    readonly property real stateFocus: 0.10
    readonly property real statePressed: 0.10
    readonly property real stateDragged: 0.16

    /// The wash itself, for a given colour.
    function stateLayer(color, strength) {
        return Qt.rgba(color.r, color.g, color.b, strength);
    }

    // ---- elevation ----

    /// Material's five shadow levels. The island is the only thing in the
    /// shell that floats, so it is the only thing that casts one.
    readonly property var elevation: ({
        0: [],
        1: [{ x: 0, y: 1, blur: 2, spread: 0, alpha: 0.30 }],
        2: [{ x: 0, y: 2, blur: 6, spread: -1, alpha: 0.32 }],
        3: [{ x: 0, y: 4, blur: 8, spread: -2, alpha: 0.34 }],
        4: [{ x: 0, y: 6, blur: 10, spread: -2, alpha: 0.36 }],
        5: [{ x: 0, y: 8, blur: 12, spread: -3, alpha: 0.38 }]
    })

    // ---- type ----

    readonly property string fontFamily: "Inter"
    readonly property string monoFamily: "JetBrains Mono, monospace"
}
