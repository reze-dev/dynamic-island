pragma Singleton

import QtQuick

QtObject {
    readonly property color surface: Qt.rgba(0.055, 0.059, 0.086, 0.92)
    readonly property color surfaceRaised: Qt.rgba(0.098, 0.106, 0.145, 0.96)
    readonly property color border: Qt.rgba(1, 1, 1, 0.09)
    readonly property color text: Qt.rgba(0.937, 0.945, 0.969)
    readonly property color textDim: Qt.rgba(0.937, 0.945, 0.969, 0.55)
    readonly property color accent: Qt.rgba(0.494, 0.729, 0.898)

    readonly property int radiusPill: Math.round(compactHeight / 2)
    readonly property int radiusPanel: 28

    readonly property int spacingTight: 8
    readonly property int spacing: 16
    readonly property int spacingLoose: 24

    readonly property int compactWidth: 260
    readonly property int compactHeight: 40
    readonly property int panelWidth: 520
    readonly property int panelHeight: 420

    readonly property int morphDuration: 340
    readonly property int crossfadeDuration: 180
}