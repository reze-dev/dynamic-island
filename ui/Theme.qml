pragma Singleton

import QtQuick

// The design tokens.
//
// Everything visual in the shell comes from here, so that the island, the
// panels and the components all move and shade together. The palette is the
// one from the reference mock: a near-black island on a faintly violet
// surface, with a lavender accent and a small set of semantic tones.
QtObject {
    id: theme

    // ---- surfaces ----

    /// The island itself. Almost black: the idle state is meant to read as a
    /// hole punched in the top of the screen.
    readonly property color islandSurface: Qt.rgba(0.020, 0.020, 0.031, 0.96)
    /// The island when a panel is open.
    readonly property color islandSurfacePanel: Qt.rgba(0.043, 0.043, 0.063, 0.98)
    /// Cards and rows inside a panel.
    readonly property color card: Qt.rgba(1, 1, 1, 0.038)
    /// A card the pointer is over.
    readonly property color cardHover: Qt.rgba(1, 1, 1, 0.070)
    /// A recessed well, for inputs.
    readonly property color well: Qt.rgba(0, 0, 0, 0.28)

    readonly property color border: Qt.rgba(1, 1, 1, 0.085)
    readonly property color borderStrong: Qt.rgba(1, 1, 1, 0.16)

    // ---- text ----

    readonly property color text: Qt.rgba(0.937, 0.945, 0.969)
    readonly property color textDim: Qt.rgba(0.937, 0.945, 0.969, 0.58)
    readonly property color textFaint: Qt.rgba(0.937, 0.945, 0.969, 0.34)

    // ---- accent and tones ----

    readonly property color accent: Qt.rgba(0.769, 0.655, 0.906)
    readonly property color accentDim: Qt.rgba(0.769, 0.655, 0.906, 0.16)

    readonly property color toneNeutral: Qt.rgba(0.937, 0.945, 0.969, 0.45)
    readonly property color toneInfo: Qt.rgba(0.769, 0.655, 0.906)
    readonly property color toneGood: Qt.rgba(0.639, 0.745, 0.549)
    readonly property color toneWarn: Qt.rgba(0.965, 0.757, 0.467)
    readonly property color toneCritical: Qt.rgba(0.922, 0.435, 0.573)

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

    // ---- geometry ----

    readonly property int radiusPill: Math.round(compactHeight / 2)
    readonly property int radiusPanel: 26
    readonly property int radiusCard: 13
    readonly property int radiusSmall: 9

    readonly property int spacingTight: 8
    readonly property int spacing: 16
    readonly property int spacingLoose: 24

    // The four sizes the island takes. Idle is the smallest thing that can
    // still carry a dot and a clock; the rest grow from it.
    readonly property int compactWidth: 132
    readonly property int compactHeight: 36
    readonly property int peekWidth: 320
    readonly property int peekHeight: 42
    readonly property int triggeredWidth: 380
    readonly property int triggeredHeight: 56
    readonly property int panelWidth: 520
    readonly property int panelHeight: 440

    // ---- motion ----

    /// How long the island takes to change size. Long enough to read as a
    /// physical object, short enough not to feel like waiting.
    readonly property int morphDuration: 380
    /// How long content takes to fade.
    readonly property int crossfadeDuration: 200
    /// How long content takes to slide. Slightly longer than the morph, so
    /// the text arrives as the shape settles.
    readonly property int slideDuration: 420
    /// The delay before content starts moving, so the geometry leads.
    readonly property int contentDelay: 40

    // ---- type ----

    readonly property string fontFamily: "Inter"
    readonly property string monoFamily: "JetBrains Mono, monospace"
    readonly property int fontTiny: 10
    readonly property int fontSmall: 11
    readonly property int fontBody: 12
    readonly property int fontMedium: 13
    readonly property int fontLarge: 15
    readonly property int fontTitle: 17
}
